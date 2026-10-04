use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::{
    _entities::{applications, server_pool_members},
    server_pools::Model as ServerPoolModel,
    servers::Model as ServerModel,
};

#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    pub server: ServerModel,
    pub used_units: i64,
    pub capacity_units: i64,
    pub weight: i32,
}

pub struct PlacementService;

impl PlacementService {
    pub async fn select(
        db: &DatabaseConnection,
        server_pool_id: i64,
        required_units: i32,
    ) -> Result<PlacementCandidate> {
        if required_units < 1 {
            return Err(Error::BadRequest(
                "resource_units must be at least 1".to_string(),
            ));
        }

        let pool = ServerPoolModel::find_by_id(db, server_pool_id).await?;
        let required_tags = pool.required_tags()?;
        let members = server_pool_members::Entity::find()
            .filter(server_pool_members::Column::ServerPoolId.eq(server_pool_id))
            .all(db)
            .await?;

        let mut candidates = Vec::new();
        for member in members {
            let server = match ServerModel::find_by_id(db, member.server_id).await {
                Ok(server) => server,
                Err(_) => continue,
            };

            if server.organization_id != Some(pool.organization_id) || server.status != "online" {
                continue;
            }

            let server_tags = server.tags()?;
            if !required_tags.iter().all(|tag| server_tags.contains(tag)) {
                continue;
            }

            let assigned = applications::Entity::find()
                .filter(applications::Column::ServerId.eq(server.id))
                .all(db)
                .await?;
            let used_units = assigned
                .iter()
                .map(|application| i64::from(application.resource_units.max(0)))
                .sum::<i64>();
            let capacity_units = i64::from(server.capacity_units.max(1));

            if used_units + i64::from(required_units) > capacity_units {
                continue;
            }

            candidates.push(PlacementCandidate {
                server,
                used_units,
                capacity_units,
                weight: member.weight,
            });
        }

        candidates.sort_by(|left, right| {
            let left_scaled = left.used_units.saturating_mul(right.capacity_units);
            let right_scaled = right.used_units.saturating_mul(left.capacity_units);
            left_scaled
                .cmp(&right_scaled)
                .then_with(|| right.weight.cmp(&left.weight))
                .then_with(|| left.server.id.cmp(&right.server.id))
        });

        candidates.into_iter().next().ok_or_else(|| {
            Error::BadRequest(
                "no server in the selected pool satisfies tags and capacity constraints"
                    .to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilization_sort_is_deterministic() {
        fn candidate(id: i64, used: i64, capacity: i64, weight: i32) -> PlacementCandidate {
            PlacementCandidate {
                server: ServerModel {
                    id,
                    organization_id: Some(1),
                    name: format!("s{id}"),
                    host: "127.0.0.1".to_string(),
                    port: 22,
                    username: "root".to_string(),
                    authentication_type: "ssh_key".to_string(),
                    encrypted_private_key: None,
                    known_host_fingerprint: None,
                    status: "online".to_string(),
                    tags_json: "[]".to_string(),
                    capacity_units: capacity as i32,
                    last_seen_at: None,
                    created_at: chrono::Utc::now().fixed_offset(),
                    updated_at: chrono::Utc::now().fixed_offset(),
                },
                used_units: used,
                capacity_units: capacity,
                weight,
            }
        }

        let mut values = vec![
            candidate(3, 5, 10, 100),
            candidate(2, 2, 10, 50),
            candidate(1, 2, 10, 100),
        ];
        values.sort_by(|left, right| {
            let left_scaled = left.used_units.saturating_mul(right.capacity_units);
            let right_scaled = right.used_units.saturating_mul(left.capacity_units);
            left_scaled
                .cmp(&right_scaled)
                .then_with(|| right.weight.cmp(&left.weight))
                .then_with(|| left.server.id.cmp(&right.server.id))
        });
        assert_eq!(values[0].server.id, 1);
        assert_eq!(values[1].server.id, 2);
        assert_eq!(values[2].server.id, 3);
    }
}
