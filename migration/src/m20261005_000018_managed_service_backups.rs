use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(ManagedServiceBackups::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ManagedServiceBackups::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::OrganizationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::ServiceId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::ServerId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::VolumeId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::ArtifactPath)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::HelperImage)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::Status)
                        .string()
                        .not_null()
                        .default("running"),
                )
                .col(ColumnDef::new(ManagedServiceBackups::SizeBytes).big_integer())
                .col(ColumnDef::new(ManagedServiceBackups::Sha256).string())
                .col(
                    ColumnDef::new(ManagedServiceBackups::Verified)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(ColumnDef::new(ManagedServiceBackups::VerificationMessage).text())
                .col(ColumnDef::new(ManagedServiceBackups::ErrorMessage).text())
                .col(
                    ColumnDef::new(ManagedServiceBackups::DeletionProtected)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::CompletedAt)
                        .timestamp_with_time_zone(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBackups::RestoredAt)
                        .timestamp_with_time_zone(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_backup_org")
                        .from(
                            ManagedServiceBackups::Table,
                            ManagedServiceBackups::OrganizationId,
                        )
                        .to(Organizations::Table, Organizations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_backup_service")
                        .from(ManagedServiceBackups::Table, ManagedServiceBackups::ServiceId)
                        .to(ManagedServices::Table, ManagedServices::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_backup_server")
                        .from(ManagedServiceBackups::Table, ManagedServiceBackups::ServerId)
                        .to(Servers::Table, Servers::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_backup_volume")
                        .from(ManagedServiceBackups::Table, ManagedServiceBackups::VolumeId)
                        .to(PersistentVolumes::Table, PersistentVolumes::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_managed_service_backup_service_created")
                .table(ManagedServiceBackups::Table)
                .col(ManagedServiceBackups::ServiceId)
                .col(ManagedServiceBackups::CreatedAt)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(
            Table::drop()
                .table(ManagedServiceBackups::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ManagedServiceBackups {
    Table,
    Id,
    OrganizationId,
    ServiceId,
    ServerId,
    VolumeId,
    ArtifactPath,
    HelperImage,
    Status,
    SizeBytes,
    Sha256,
    Verified,
    VerificationMessage,
    ErrorMessage,
    DeletionProtected,
    CreatedAt,
    CompletedAt,
    RestoredAt,
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ManagedServices {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PersistentVolumes {
    Table,
    Id,
}
