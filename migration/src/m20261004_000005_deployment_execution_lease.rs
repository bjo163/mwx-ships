use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(ColumnDef::new(Deployments::ExecutionToken).string().null())
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(
                    ColumnDef::new(Deployments::LeaseExpiresAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(
                    ColumnDef::new(Deployments::AttemptCount)
                        .integer()
                        .not_null()
                        .default(0),
                )
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::AttemptCount)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::LeaseExpiresAt)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::ExecutionToken)
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Deployments {
    Table,
    ExecutionToken,
    LeaseExpiresAt,
    AttemptCount,
}
