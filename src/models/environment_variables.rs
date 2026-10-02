use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::environment_variables::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct SetEnvVarParams {
    pub key: String,
    pub value: String,
    pub is_secret: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SafeEnvVar {
    pub id: i64,
    pub application_id: i64,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

impl Model {
    pub async fn by_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<Model>> {
        let list = Entity::find()
            .filter(environment_variables::Column::ApplicationId.eq(application_id))
            .order_by_asc(environment_variables::Column::Key)
            .all(db)
            .await?;
        Ok(list)
    }

    pub fn to_safe(&self) -> SafeEnvVar {
        SafeEnvVar {
            id: self.id,
            application_id: self.application_id,
            key: self.key.clone(),
            value: if self.is_secret {
                "********".to_string()
            } else {
                self.encrypted_value.clone()
            },
            is_secret: self.is_secret,
        }
    }
}
