use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(ManagedServices::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ManagedServices::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(ManagedServices::OrganizationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServices::ServerId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServices::VolumeId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(ManagedServices::Name).string().not_null())
                .col(ColumnDef::new(ManagedServices::Slug).string().not_null())
                .col(ColumnDef::new(ManagedServices::Kind).string().not_null())
                .col(ColumnDef::new(ManagedServices::Image).string().not_null())
                .col(
                    ColumnDef::new(ManagedServices::ContainerName)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServices::InternalPort)
                        .integer()
                        .not_null(),
                )
                .col(ColumnDef::new(ManagedServices::DatabaseName).string())
                .col(ColumnDef::new(ManagedServices::Username).string())
                .col(
                    ColumnDef::new(ManagedServices::EncryptedCredentials)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServices::Status)
                        .string()
                        .not_null()
                        .default("declared"),
                )
                .col(
                    ColumnDef::new(ManagedServices::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServices::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_org")
                        .from(ManagedServices::Table, ManagedServices::OrganizationId)
                        .to(Organizations::Table, Organizations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_server")
                        .from(ManagedServices::Table, ManagedServices::ServerId)
                        .to(Servers::Table, Servers::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_volume")
                        .from(ManagedServices::Table, ManagedServices::VolumeId)
                        .to(PersistentVolumes::Table, PersistentVolumes::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_managed_service_org_slug")
                .table(ManagedServices::Table)
                .col(ManagedServices::OrganizationId)
                .col(ManagedServices::Slug)
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_managed_service_container")
                .table(ManagedServices::Table)
                .col(ManagedServices::ContainerName)
                .unique()
                .to_owned(),
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_managed_service_volume")
                .table(ManagedServices::Table)
                .col(ManagedServices::VolumeId)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(ManagedServiceBindings::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ManagedServiceBindings::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBindings::ServiceId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBindings::ApplicationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBindings::EnvPrefix)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBindings::EnvKeysJson)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ManagedServiceBindings::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_binding_service")
                        .from(
                            ManagedServiceBindings::Table,
                            ManagedServiceBindings::ServiceId,
                        )
                        .to(ManagedServices::Table, ManagedServices::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_managed_service_binding_application")
                        .from(
                            ManagedServiceBindings::Table,
                            ManagedServiceBindings::ApplicationId,
                        )
                        .to(Applications::Table, Applications::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_managed_service_binding_unique")
                .table(ManagedServiceBindings::Table)
                .col(ManagedServiceBindings::ServiceId)
                .col(ManagedServiceBindings::ApplicationId)
                .col(ManagedServiceBindings::EnvPrefix)
                .unique()
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(
            Table::drop()
                .table(ManagedServiceBindings::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        m.drop_table(
            Table::drop()
                .table(ManagedServices::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ManagedServices {
    Table,
    Id,
    OrganizationId,
    ServerId,
    VolumeId,
    Name,
    Slug,
    Kind,
    Image,
    ContainerName,
    InternalPort,
    DatabaseName,
    Username,
    EncryptedCredentials,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ManagedServiceBindings {
    Table,
    Id,
    ServiceId,
    ApplicationId,
    EnvPrefix,
    EnvKeysJson,
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
enum PersistentVolumes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    Id,
}
