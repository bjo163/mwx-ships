use crate::{
    mailers::auth::AuthMailer,
    models::{
        _entities::users,
        api_tokens::Model as ApiTokenModel,
        audit_events::{AuditEventInput, Model as AuditEventModel},
        auth_rate_limits::Model as AuthRateLimitModel,
        users::{LoginParams, RegisterParams},
    },
    services::access_control::Principal,
    views::auth::{CurrentResponse, LoginResponse},
};
use axum::http::{HeaderMap, StatusCode};
use loco_rs::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub static EMAIL_DOMAIN_RE: OnceLock<Regex> = OnceLock::new();

fn get_allow_email_domain_re() -> &'static Regex {
    EMAIL_DOMAIN_RE.get_or_init(|| {
        Regex::new(r"@example\.com$|@gmail\.com$").expect("Failed to compile regex")
    })
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForgotParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResetParams {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResendVerificationParams {
    pub email: String,
}

/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>,
) -> Result<Response> {
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    let user = match res {
        Ok(user) => user,
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );
            return format::json(());
        }
    };

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;

    format::json(())
}

/// Verify register user. if the user not verified his email, he can't login to
/// the system.
#[debug_handler]
async fn verify(State(ctx): State<AppContext>, Path(token): Path<String>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_verification_token(&ctx.db, &token).await else {
        return unauthorized("invalid token");
    };

    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        let active_model = user.into_active_model();
        let user = active_model.verified(&ctx.db).await?;
        tracing::info!(pid = user.pid.to_string(), "user verified");
    }

    format::json(())
}

/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for security reasons (not exposing users DB
/// list).
#[debug_handler]
async fn forgot(
    State(ctx): State<AppContext>,
    Json(params): Json<ForgotParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return format::json(());
    };

    let user = user
        .into_active_model()
        .set_forgot_password_sent(&ctx.db)
        .await?;

    AuthMailer::forgot_password(&ctx, &user).await?;

    format::json(())
}

/// reset user password by the given parameters
#[debug_handler]
async fn reset(State(ctx): State<AppContext>, Json(params): Json<ResetParams>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_reset_token(&ctx.db, &params.token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::info!("reset token not found");

        return format::json(());
    };
    user.into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    format::json(())
}

/// Creates a user login and returns a token
#[debug_handler]
async fn login(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<LoginParams>,
) -> Result<Response> {
    let user = users::Model::find_by_email(&ctx.db, &params.email)
        .await
        .ok();
    let valid = user
        .as_ref()
        .map(|user| user.verify_password(&params.password))
        .unwrap_or(false);

    if !valid {
        let decision = AuthRateLimitModel::check_and_record(
            &ctx.db,
            "login",
            &params.email.to_ascii_lowercase(),
            5,
            900,
            900,
        )
        .await?;

        let request_id = request_id_from_headers(&headers);
        let _ = AuditEventModel::append(
            &ctx.db,
            AuditEventInput {
                organization_id: None,
                actor_kind: "auth".to_string(),
                actor_id: user
                    .as_ref()
                    .map(|user| user.pid.to_string())
                    .unwrap_or_else(|| "anonymous".to_string()),
                action: "login.failure".to_string(),
                resource_type: Some("session".to_string()),
                resource_id: None,
                outcome: "failure".to_string(),
                request_id: Some(request_id.clone()),
                metadata: Some(serde_json::json!({
                    "rate_limited": !decision.allowed,
                })),
            },
        )
        .await;

        if !decision.allowed {
            return Ok((
                StatusCode::TOO_MANY_REQUESTS,
                format::json(serde_json::json!({
                    "error": {
                        "code": "AUTH_RATE_LIMITED",
                        "message": "Too many failed login attempts",
                        "retry_after_seconds": decision.retry_after_seconds,
                    }
                }))?,
            )
                .into_response());
        }

        return unauthorized("Invalid credentials!");
    }

    let user = user.expect("valid credentials require a loaded user");
    let jwt_secret = ctx.config.get_jwt_config()?;
    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: None,
            actor_kind: "jwt".to_string(),
            actor_id: user.pid.to_string(),
            action: "login.success".to_string(),
            resource_type: Some("session".to_string()),
            resource_id: None,
            outcome: "success".to_string(),
            request_id: Some(request_id_from_headers(&headers)),
            metadata: None,
        },
    )
    .await;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn current(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    format::json(CurrentResponse::new(&principal.user))
}

#[debug_handler]
async fn revoke(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;

    if let Some(token) = principal.api_token.clone() {
        let revoked = ApiTokenModel::revoke(&ctx.db, token.id, principal.user.id).await?;
        let (actor_kind, actor_id) = principal.audit_actor();
        let _ = AuditEventModel::append(
            &ctx.db,
            AuditEventInput {
                organization_id: Some(token.organization_id),
                actor_kind: actor_kind.to_string(),
                actor_id,
                action: "api_token.self_revoke".to_string(),
                resource_type: Some("api_token".to_string()),
                resource_id: Some(token.id.to_string()),
                outcome: "success".to_string(),
                request_id: Some(principal.request_id.clone()),
                metadata: principal.audit_metadata(None),
            },
        )
        .await;
        return format::json(serde_json::json!({
            "data": {
                "token_id": revoked.id,
                "revoked": true
            },
            "message": "API token revoked"
        }));
    }

    let user = users::Model::revoke_sessions(&ctx.db, principal.user.id).await?;
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: None,
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "auth.session.revoke".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(principal.user.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(None),
        },
    )
    .await;
    format::json(serde_json::json!({
        "data": {
            "session_version": user.session_version
        },
        "message": "All JWT sessions revoked"
    }))
}

/// Magic link authentication provides a secure and passwordless way to log in to the application.
///
/// # Flow
/// 1. **Request a Magic Link**:
///    A registered user sends a POST request to `/magic-link` with their email.
///    If the email exists, a short-lived, one-time-use token is generated and sent to the user's email.
///    For security and to avoid exposing whether an email exists, the response always returns 200, even if the email is invalid.
///
/// 2. **Click the Magic Link**:
///    The user clicks the link (/magic-link/{token}), which validates the token and its expiration.
///    If valid, the server generates a JWT and responds with a [`LoginResponse`].
///    If invalid or expired, an unauthorized response is returned.
///
/// This flow enhances security by avoiding traditional passwords and providing a seamless login experience.
async fn magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    let decision = AuthRateLimitModel::check_and_record(
        &ctx.db,
        "magic_link",
        &params.email.to_ascii_lowercase(),
        5,
        900,
        900,
    )
    .await?;
    if !decision.allowed {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            format::json(serde_json::json!({
                "error": {
                    "code": "AUTH_RATE_LIMITED",
                    "message": "Too many magic-link requests",
                    "retry_after_seconds": decision.retry_after_seconds,
                }
            }))?,
        )
            .into_response());
    }

    let email_regex = get_allow_email_domain_re();
    if !email_regex.is_match(&params.email) {
        tracing::debug!(
            email = params.email,
            "The provided email is invalid or does not match the allowed domains"
        );
        return bad_request("invalid request");
    }

    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::debug!(email = params.email, "user not found by email");
        return format::empty_json();
    };

    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    AuthMailer::send_magic_link(&ctx, &user).await?;

    format::empty_json()
}

/// Verifies a magic link token and authenticates the user.
async fn magic_link_verify(
    Path(token): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &token).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        return unauthorized("unauthorized!");
    };

    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;

    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_jwt(&jwt_secret.secret, jwt_secret.expiration)
        .or_else(|_| unauthorized("unauthorized!"))?;

    format::json(LoginResponse::new(&user, &token))
}

#[debug_handler]
async fn resend_verification_email(
    State(ctx): State<AppContext>,
    Json(params): Json<ResendVerificationParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::info!(
            email = params.email,
            "User not found for resend verification"
        );
        return format::json(());
    };

    if user.email_verified_at.is_some() {
        tracing::info!(
            pid = user.pid.to_string(),
            "User already verified, skipping resend"
        );
        return format::json(());
    }

    let user = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;

    AuthMailer::send_welcome(&ctx, &user).await?;
    tracing::info!(pid = user.pid.to_string(), "Verification email re-sent");

    format::json(())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/register", post(register))
        .add("/verify/{token}", get(verify))
        .add("/login", post(login))
        .add("/forgot", post(forgot))
        .add("/reset", post(reset))
        .add("/current", get(current))
        .add("/revoke", post(revoke))
        .add("/magic-link", post(magic_link))
        .add("/magic-link/{token}", get(magic_link_verify))
        .add("/resend-verification-mail", post(resend_verification_email))
}

fn request_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(128).collect())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}
