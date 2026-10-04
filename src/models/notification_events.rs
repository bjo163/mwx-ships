use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

pub use super::_entities::notification_events::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone)]
pub struct NotificationClaim {
    pub event: Model,
    pub should_send: bool,
}

impl Model {
    pub async fn claim(
        db: &DatabaseConnection,
        event_kind: &str,
        severity: &str,
        message: &str,
        dedupe_subject: &str,
        cooldown_seconds: i64,
    ) -> Result<NotificationClaim> {
        let fingerprint = fingerprint(event_kind, dedupe_subject);
        let now = Utc::now();

        if let Some(existing) = Entity::find()
            .filter(notification_events::Column::Fingerprint.eq(&fingerprint))
            .one(db)
            .await?
        {
            let in_cooldown = existing
                .cooldown_until
                .as_ref()
                .map(|until| until.timestamp() > now.timestamp())
                .unwrap_or(false);
            if in_cooldown {
                return Ok(NotificationClaim {
                    event: existing,
                    should_send: false,
                });
            }

            let attempts = existing.attempts.saturating_add(1);
            let mut active: ActiveModel = existing.into();
            active.event_kind = Set(event_kind.to_string());
            active.severity = Set(severity.to_string());
            active.message = Set(message.to_string());
            active.status = Set("pending".to_string());
            active.attempts = Set(attempts);
            active.cooldown_until =
                Set(Some((now + Duration::seconds(cooldown_seconds.max(60))).into()));
            active.updated_at = Set(now.into());
            let event = active.update(db).await?;
            return Ok(NotificationClaim {
                event,
                should_send: true,
            });
        }

        let active = ActiveModel {
            fingerprint: Set(fingerprint),
            event_kind: Set(event_kind.to_string()),
            severity: Set(severity.to_string()),
            message: Set(message.to_string()),
            status: Set("pending".to_string()),
            attempts: Set(1),
            cooldown_until: Set(Some(
                (now + Duration::seconds(cooldown_seconds.max(60))).into(),
            )),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        let event = active.insert(db).await?;
        Ok(NotificationClaim {
            event,
            should_send: true,
        })
    }

    pub async fn mark_sent(db: &DatabaseConnection, id: i64) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set("sent".to_string());
        active.last_sent_at = Set(Some(Utc::now().into()));
        active.last_error = Set(None);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn mark_skipped(db: &DatabaseConnection, id: i64) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set("skipped".to_string());
        active.last_error = Set(None);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn mark_failed(db: &DatabaseConnection, id: i64, error: &str) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set("failed".to_string());
        active.last_error = Set(Some(error.to_string()));
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }
}

fn fingerprint(event_kind: &str, subject: &str) -> String {
    let digest = Sha256::digest(format!("{event_kind}:{subject}").as_bytes());
    format!("sha256:{}", hex::encode(digest))
}
