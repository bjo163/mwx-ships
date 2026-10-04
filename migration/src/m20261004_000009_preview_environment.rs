use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(PreviewDeployments::Table)
                .add_column(
                    ColumnDef::new(PreviewDeployments::PreviewEnvironmentId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_preview_environment_id")
                .table(PreviewDeployments::Table)
                .col(PreviewDeployments::PreviewEnvironmentId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_preview_environment_id")
                .table(PreviewDeployments::Table)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(PreviewDeployments::Table)
                .drop_column(PreviewDeployments::PreviewEnvironmentId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PreviewDeployments {
    Table,
    PreviewEnvironmentId,
}
