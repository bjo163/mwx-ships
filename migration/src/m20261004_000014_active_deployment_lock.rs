use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .add_column(
                    ColumnDef::new(Applications::ActiveDeploymentId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_applications_active_deployment")
                .table(Applications::Table)
                .col(Applications::ActiveDeploymentId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_applications_active_deployment")
                .table(Applications::Table)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .drop_column(Applications::ActiveDeploymentId)
                .to_owned(),
        )
        .await
    }
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    ActiveDeploymentId,
}
