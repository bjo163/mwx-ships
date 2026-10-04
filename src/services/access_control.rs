use axum::http::{header::AUTHORIZATION, HeaderMap};
use loco_rs::{auth::jwt::JWT, prelude::*};
use serde_json::{Map, Value};

use crate::models::{
    api_tokens, applications, deployments,
    organization_memberships::{self, Model as MembershipModel},
    organizations::Model as OrganizationModel,
    projects, servers, users,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    View,
    Deploy,
    ManageProjects,
    ManageServers,
    ManageApplications,
    ManageOrganization,
    Owner,
}

impl Permission {
    pub fn required_scope(self) -> &'static str {
        match self {
            Self::View => "read",
            Self::Deploy => "deploy",
            Self::ManageProjects => "manage:projects",
            Self::ManageServers => "manage:servers",
            Self::ManageApplications => "manage:applications",
            Self::ManageOrganization | Self::Owner => "manage:organization",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub user: users::Model,
    pub actor_kind: &'static str,
    pub actor_id: String,
    pub api_token: Option<api_tokens::Model>,
}

impl Principal {
    pub async fn authenticate(ctx: &AppContext, headers: &HeaderMap) -> Result<Self> {
        let token = bearer_token(headers)?;
        if token.starts_with("msk_") {
            let record = api_tokens::Model::authenticate(&ctx.db, token)
                .await?
                .ok_or_else(|| Error::Unauthorized("invalid or revoked API token".to_string()))?;
            let user = users::Entity::find_by_id(record.user_id)
                .one(&ctx.db)
                .await?
                .ok_or_else(|| Error::Unauthorized("API token user no longer exists".to_string()))?;
            return Ok(Self {
                actor_kind: "api_token",
                actor_id: record.id.to_string(),
                user,
                api_token: Some(record),
            });
        }

        let jwt_config = ctx.config.get_jwt_config()?;
        let claims = JWT::new(&jwt_config.secret)
            .validate(token)
            .map_err(|_| Error::Unauthorized("invalid or expired JWT".to_string()))?
            .claims;
        let user = users::Model::find_by_pid(&ctx.db, &claims.pid)
            .await
            .map_err(|_| Error::Unauthorized("JWT user no longer exists".to_string()))?;

        let token_session_version = claims
            .claims
            .get("session_version")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        if token_session_version != user.session_version {
            return Err(Error::Unauthorized("session has been revoked".to_string()));
        }

        Ok(Self {
            actor_kind: "jwt",
            actor_id: user.pid.to_string(),
            user,
            api_token: None,
        })
    }

    pub async fn membership(
        &self,
        db: &DatabaseConnection,
        organization_id: i64,
    ) -> Result<MembershipModel> {
        if let Some(token) = &self.api_token {
            if token.organization_id != organization_id {
                return Err(Error::Unauthorized(
                    "API token is scoped to another organization".to_string(),
                ));
            }
        }

        MembershipModel::find_for_user(db, organization_id, self.user.id)
            .await?
            .filter(|membership| membership.is_active)
            .ok_or_else(|| Error::Unauthorized("organization access denied".to_string()))
    }

    pub async fn require(
        &self,
        db: &DatabaseConnection,
        organization_id: i64,
        permission: Permission,
    ) -> Result<MembershipModel> {
        let membership = self.membership(db, organization_id).await?;

        if let Some(token) = &self.api_token {
            let scope = permission.required_scope();
            if !token.has_scope(scope) {
                return Err(Error::Unauthorized(format!(
                    "API token lacks required scope '{scope}'"
                )));
            }
        }

        let allowed = match permission {
            Permission::View => membership.can_view(),
            Permission::Deploy => membership.can_deploy(),
            Permission::ManageProjects
            | Permission::ManageServers
            | Permission::ManageApplications
            | Permission::ManageOrganization => membership.can_admin(),
            Permission::Owner => membership.is_owner(),
        };

        if !allowed {
            return Err(Error::Unauthorized(format!(
                "role '{}' does not permit this operation",
                membership.role
            )));
        }

        Ok(membership)
    }

    pub async fn organization_ids(&self, db: &DatabaseConnection) -> Result<Vec<i64>> {
        if let Some(token) = &self.api_token {
            self.membership(db, token.organization_id).await?;
            return Ok(vec![token.organization_id]);
        }

        Ok(MembershipModel::list_for_user(db, self.user.id)
            .await?
            .into_iter()
            .filter(|membership| membership.can_view())
            .map(|membership| membership.organization_id)
            .collect())
    }

    pub async fn require_platform_owner(&self, db: &DatabaseConnection) -> Result<()> {
        if self.api_token.is_some() {
            return Err(Error::Unauthorized(
                "platform operations require an interactive owner session".to_string(),
            ));
        }

        let organizations = OrganizationModel::all(db).await?;
        if organizations.is_empty() {
            return Ok(());
        }

        for organization in organizations {
            let membership = self.membership(db, organization.id).await?;
            if !membership.is_owner() {
                return Err(Error::Unauthorized(
                    "platform operations require owner access to every organization"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn resolve_organization(
        &self,
        db: &DatabaseConnection,
        requested: Option<i64>,
    ) -> Result<i64> {
        let organization_ids = self.organization_ids(db).await?;
        if let Some(organization_id) = requested {
            if !organization_ids.contains(&organization_id) {
                return Err(Error::Unauthorized(
                    "requested organization is not accessible".to_string(),
                ));
            }
            return Ok(organization_id);
        }

        match organization_ids.as_slice() {
            [organization_id] => Ok(*organization_id),
            [] => Err(Error::Unauthorized(
                "user has no active organization membership".to_string(),
            )),
            _ => Err(Error::BadRequest(
                "organization_id is required when multiple organizations are available"
                    .to_string(),
            )),
        }
    }

    pub async fn project_organization(
        &self,
        db: &DatabaseConnection,
        project_id: i64,
        permission: Permission,
    ) -> Result<i64> {
        let project = projects::Model::find_by_id(db, project_id).await?;
        let organization_id = project.organization_id.ok_or_else(|| {
            Error::Unauthorized(
                "legacy project is not assigned to an organization; claim it first".to_string(),
            )
        })?;
        self.require(db, organization_id, permission).await?;
        Ok(organization_id)
    }

    pub async fn server_organization(
        &self,
        db: &DatabaseConnection,
        server_id: i64,
        permission: Permission,
    ) -> Result<i64> {
        let server = servers::Model::find_by_id(db, server_id).await?;
        let organization_id = server.organization_id.ok_or_else(|| {
            Error::Unauthorized(
                "legacy server is not assigned to an organization; claim it first".to_string(),
            )
        })?;
        self.require(db, organization_id, permission).await?;
        Ok(organization_id)
    }

    pub async fn application_organization(
        &self,
        db: &DatabaseConnection,
        application_id: i64,
        permission: Permission,
    ) -> Result<i64> {
        let application = applications::Model::find_by_id(db, application_id).await?;
        self.project_organization(db, application.project_id, permission)
            .await
    }

    pub async fn deployment_organization(
        &self,
        db: &DatabaseConnection,
        deployment_id: i64,
        permission: Permission,
    ) -> Result<i64> {
        let deployment = deployments::Model::find_by_id(db, deployment_id).await?;
        self.application_organization(db, deployment.application_id, permission)
            .await
    }

    pub fn audit_actor(&self) -> (&'static str, String) {
        (self.actor_kind, self.actor_id.clone())
    }
}

pub fn session_claims(session_version: i32) -> Map<String, Value> {
    let mut claims = Map::new();
    claims.insert(
        "session_version".to_string(),
        Value::Number(i64::from(session_version).into()),
    );
    claims
}

fn bearer_token(headers: &HeaderMap) -> Result<&str> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| Error::Unauthorized("authorization bearer token is required".to_string()))?;
    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| Error::Unauthorized("authorization bearer token is malformed".to_string()))
}
