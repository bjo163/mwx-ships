use crate::{
    mailers::ops::OpsMailer,
    models::{notification_events, operational_events},
};
use loco_rs::prelude::*;
use reqwest::Url;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct AlertPayload<'a> {
    pub event_kind: &'a str,
    pub severity: &'a str,
    pub subject: &'a str,
    pub message: &'a str,
    pub service: &'static str,
}

pub struct NotificationService;

impl NotificationService {
    pub async fn notify(
        ctx: &AppContext,
        event_kind: &str,
        severity: &str,
        subject: &str,
        message: &str,
    ) -> Result<bool> {
        let cooldown = std::env::var("MOONSHIPS_ALERT_COOLDOWN_SECS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(900)
            .max(60);

        let claim = notification_events::Model::claim(
            &ctx.db, event_kind, severity, message, subject, cooldown,
        )
        .await?;

        if !claim.should_send {
            return Ok(false);
        }

        let webhook = std::env::var("MOONSHIPS_ALERT_WEBHOOK_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let email = std::env::var("MOONSHIPS_ALERT_EMAIL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if webhook.is_none() && email.is_none() {
            let _ = notification_events::Model::mark_skipped(&ctx.db, claim.event.id).await;
            return Ok(false);
        }

        let mut errors = Vec::new();

        if let Some(webhook) = webhook {
            match Self::send_webhook(&webhook, event_kind, severity, subject, message).await {
                Ok(()) => {}
                Err(error) => errors.push(format!("webhook: {error}")),
            }
        }

        if let Some(email) = email {
            if !looks_like_email(&email) {
                errors.push("email: invalid MOONSHIPS_ALERT_EMAIL".to_string());
            } else if let Err(error) =
                OpsMailer::send_alert(ctx, &email, severity, subject, message).await
            {
                errors.push(format!("email: {error}"));
            }
        }

        if errors.is_empty() {
            notification_events::Model::mark_sent(&ctx.db, claim.event.id).await?;
            let _ = operational_events::Model::record(
                &ctx.db,
                "notification_sent",
                "info",
                Some("notification"),
                Some(claim.event.id),
                "Operational notification delivered",
                Some(&serde_json::json!({
                    "event_kind": event_kind,
                    "severity": severity,
                    "subject": subject,
                })),
            )
            .await;
            Ok(true)
        } else {
            let error = errors.join("; ");
            notification_events::Model::mark_failed(&ctx.db, claim.event.id, &error).await?;
            let _ = operational_events::Model::record(
                &ctx.db,
                "notification_failed",
                "warning",
                Some("notification"),
                Some(claim.event.id),
                "Operational notification delivery failed",
                Some(&serde_json::json!({"error": error})),
            )
            .await;
            Ok(false)
        }
    }

    async fn send_webhook(
        url: &str,
        event_kind: &str,
        severity: &str,
        subject: &str,
        message: &str,
    ) -> std::result::Result<(), String> {
        let parsed = Url::parse(url).map_err(|err| err.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("webhook URL must use http or https".to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| err.to_string())?;
        let response = client
            .post(parsed)
            .json(&AlertPayload {
                event_kind,
                severity,
                subject,
                message,
                service: "moonships",
            })
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP {}", response.status().as_u16()))
        }
    }
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !value.contains(char::is_whitespace)
        && !value.contains(['\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_alert_email_shape() {
        assert!(looks_like_email("ops@example.com"));
        assert!(!looks_like_email("not-an-email"));
        assert!(!looks_like_email("ops @example.com"));
    }
}
