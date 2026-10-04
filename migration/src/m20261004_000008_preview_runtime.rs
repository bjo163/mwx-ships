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
                    ColumnDef::new(PreviewDeployments::PreviewApplicationId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(PreviewDeployments::Table)
                .add_column(
                    ColumnDef::new(PreviewDeployments::DeploymentId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(PreviewDeployments::Table)
                .add_column(
                    ColumnDef::new(PreviewDeployments::PreviewHostname)
                        .string()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_preview_application_id")
                .table(PreviewDeployments::Table)
                .col(PreviewDeployments::PreviewApplicationId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_preview_application_id")
                .table(PreviewDeployments::Table)
                .to_owned(),
        )
        .await?;

        for column in [
            PreviewDeployments::PreviewHostname,
            PreviewDeployments::DeploymentId,
            PreviewDeployments::PreviewApplicationId,
        ] {
            m.alter_table(
                Table::alter()
                    .table(PreviewDeployments::Table)
                    .drop_column(column)
                    .to_owned(),
            )
            .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PreviewDeployments {
    Table,
    PreviewApplicationId,
    DeploymentId,
    PreviewHostname,
}
