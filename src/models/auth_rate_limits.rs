use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

pub use super::_entities::auth_rate_limits::{self, ActiveModel, Entity, Model};

pub struct RateLimitDecision {
    pub allowed: bool,
    pub retry_after_seconds: Option<i64>,
}

impl Model {
    pub async fn check_and_record(
        db: &DatabaseConnection,
        action: &str,
        raw_key: &str,
        max_attempts: i32,
        window_seconds: i64,
        block_seconds: i64,
    ) -> Result<RateLimitDecision> {
        let now = Utc::now();
        let now_fixed = now.fixed_offset();
        let key_hash = hash_key(raw_key);
        let existing = Entity::find()
            .filter(auth_rate_limits::Column::Action.eq(action))
            .filter(auth_rate_limits::Column::KeyHash.eq(&key_hash))
            .one(db)
            .await?;

        if let Some(model) = existing {
            if let Some(blocked_until) = model.blocked_until {
                if blocked_until > now_fixed {
                    return Ok(RateLimitDecision {
                        allowed: false,
                        retry_after_seconds: Some((blocked_until - now_fixed).num_seconds().max(1)),
                    });
                }
            }

            let window_expired =
                model.window_started_at + Duration::seconds(window_seconds) <= now_fixed;
            let attempts = if window_expired {
                1
            } else {
                model.attempts.saturating_add(1)
            };
            let blocked_until =
                (attempts > max_attempts).then(|| (now + Duration::seconds(block_seconds)).into());

            let mut active: ActiveModel = model.into();
            active.attempts = Set(attempts);
            active.window_started_at = Set(if window_expired {
                now.into()
            } else {
                active.window_started_at.unwrap()
            });
            active.blocked_until = Set(blocked_until);
            active.updated_at = Set(now.into());
            active.update(db).await?;

            return Ok(RateLimitDecision {
                allowed: blocked_until.is_none(),
                retry_after_seconds: blocked_until
                    .map(|until| (until - now_fixed).num_seconds().max(1)),
            });
        }

        ActiveModel {
            action: Set(action.to_string()),
            key_hash: Set(key_hash),
            attempts: Set(1),
            window_started_at: Set(now.into()),
            blocked_until: Set(None),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(RateLimitDecision {
            allowed: true,
            retry_after_seconds: None,
        })
    }
}

fn hash_key(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
