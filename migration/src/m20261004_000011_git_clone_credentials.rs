use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(GitIntegrations::Table)
                .add_column(ColumnDef::new(GitIntegrations::GitUsername).string().null())
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(GitIntegrations::Table)
                .drop_column(GitIntegrations::GitUsername)
                .to_owned(),
        )
        .await
    }
}

#[derive(DeriveIden)]
enum GitIntegrations {
    Table,
    GitUsername,
}
