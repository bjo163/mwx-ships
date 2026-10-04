use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(
                    ColumnDef::new(Deployments::TriggerKind)
                        .string()
                        .not_null()
                        .default("manual"),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(
                    ColumnDef::new(Deployments::SourceDeploymentId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_deployments_source_deployment_id")
                .table(Deployments::Table)
                .col(Deployments::SourceDeploymentId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_deployments_source_deployment_id")
                .table(Deployments::Table)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::SourceDeploymentId)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::TriggerKind)
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Deployments {
    Table,
    TriggerKind,
    SourceDeploymentId,
}
