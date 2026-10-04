use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(PersistentVolumes::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(PersistentVolumes::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(PersistentVolumes::OrganizationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PersistentVolumes::ServerId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(PersistentVolumes::Name).string().not_null())
                .col(
                    ColumnDef::new(PersistentVolumes::DockerVolumeName)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PersistentVolumes::DeletionProtected)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(PersistentVolumes::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PersistentVolumes::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_persistent_volume_org")
                        .from(PersistentVolumes::Table, PersistentVolumes::OrganizationId)
                        .to(Organizations::Table, Organizations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_persistent_volume_server")
                        .from(PersistentVolumes::Table, PersistentVolumes::ServerId)
                        .to(Servers::Table, Servers::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_persistent_volume_org_server_name")
                .table(PersistentVolumes::Table)
                .col(PersistentVolumes::OrganizationId)
                .col(PersistentVolumes::ServerId)
                .col(PersistentVolumes::Name)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_persistent_volume_docker_name")
                .table(PersistentVolumes::Table)
                .col(PersistentVolumes::DockerVolumeName)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(VolumeAttachments::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(VolumeAttachments::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(VolumeAttachments::VolumeId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(VolumeAttachments::ApplicationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(VolumeAttachments::MountPath)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(VolumeAttachments::ReadOnly)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(VolumeAttachments::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_volume_attachment_volume")
                        .from(VolumeAttachments::Table, VolumeAttachments::VolumeId)
                        .to(PersistentVolumes::Table, PersistentVolumes::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_volume_attachment_application")
                        .from(VolumeAttachments::Table, VolumeAttachments::ApplicationId)
                        .to(Applications::Table, Applications::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_volume_attachment_single_owner")
                .table(VolumeAttachments::Table)
                .col(VolumeAttachments::VolumeId)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_volume_attachment_mount_unique")
                .table(VolumeAttachments::Table)
                .col(VolumeAttachments::ApplicationId)
                .col(VolumeAttachments::MountPath)
                .unique()
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(
            Table::drop()
                .table(VolumeAttachments::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        m.drop_table(
            Table::drop()
                .table(PersistentVolumes::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum PersistentVolumes {
    Table,
    Id,
    OrganizationId,
    ServerId,
    Name,
    DockerVolumeName,
    DeletionProtected,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum VolumeAttachments {
    Table,
    Id,
    VolumeId,
    ApplicationId,
    MountPath,
    ReadOnly,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    Id,
}
