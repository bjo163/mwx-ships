use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(DeploymentRevisions::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(DeploymentRevisions::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::ApplicationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::ServerId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::SourceCommitHash)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::SourceCommitMessage)
                        .text()
                        .null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::ImageReference)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::RevisionHash)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::RuntimeSnapshot)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::Status)
                        .string()
                        .not_null()
                        .default("prepared"),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::HealthyAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(DeploymentRevisions::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_deployment_revisions_application")
                        .from(
                            DeploymentRevisions::Table,
                            DeploymentRevisions::ApplicationId,
                        )
                        .to(Applications::Table, Applications::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_deployment_revisions_server")
                        .from(DeploymentRevisions::Table, DeploymentRevisions::ServerId)
                        .to(Servers::Table, Servers::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_deployment_revisions_app_hash")
                .table(DeploymentRevisions::Table)
                .col(DeploymentRevisions::ApplicationId)
                .col(DeploymentRevisions::RevisionHash)
                .unique()
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .add_column(
                    ColumnDef::new(Applications::CurrentRevisionId)
                        .big_integer()
                        .null(),
                )
                .add_column(
                    ColumnDef::new(Applications::PreviousRevisionId)
                        .big_integer()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .add_column(ColumnDef::new(Deployments::RevisionId).big_integer().null())
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_deployments_revision_id")
                .table(Deployments::Table)
                .col(Deployments::RevisionId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_deployments_revision_id")
                .table(Deployments::Table)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Deployments::Table)
                .drop_column(Deployments::RevisionId)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .drop_column(Applications::PreviousRevisionId)
                .drop_column(Applications::CurrentRevisionId)
                .to_owned(),
        )
        .await?;

        m.drop_table(
            Table::drop()
                .table(DeploymentRevisions::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum DeploymentRevisions {
    Table,
    Id,
    ApplicationId,
    ServerId,
    SourceCommitHash,
    SourceCommitMessage,
    ImageReference,
    RevisionHash,
    RuntimeSnapshot,
    Status,
    HealthyAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    Id,
    CurrentRevisionId,
    PreviousRevisionId,
}

#[derive(DeriveIden)]
enum Deployments {
    Table,
    RevisionId,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}
