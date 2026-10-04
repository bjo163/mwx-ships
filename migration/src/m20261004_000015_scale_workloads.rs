use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(ServerPools::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ServerPools::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(ServerPools::OrganizationId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(ServerPools::Name).string().not_null())
                .col(ColumnDef::new(ServerPools::Slug).string().not_null())
                .col(
                    ColumnDef::new(ServerPools::RequiredTagsJson)
                        .text()
                        .not_null()
                        .default("[]"),
                )
                .col(
                    ColumnDef::new(ServerPools::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ServerPools::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_server_pool_org")
                        .from(ServerPools::Table, ServerPools::OrganizationId)
                        .to(Organizations::Table, Organizations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_server_pool_org_slug")
                .table(ServerPools::Table)
                .col(ServerPools::OrganizationId)
                .col(ServerPools::Slug)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(ServerPoolMembers::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ServerPoolMembers::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(ServerPoolMembers::ServerPoolId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ServerPoolMembers::ServerId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ServerPoolMembers::Weight)
                        .integer()
                        .not_null()
                        .default(100),
                )
                .col(
                    ColumnDef::new(ServerPoolMembers::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_server_pool_member_pool")
                        .from(ServerPoolMembers::Table, ServerPoolMembers::ServerPoolId)
                        .to(ServerPools::Table, ServerPools::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_server_pool_member_server")
                        .from(ServerPoolMembers::Table, ServerPoolMembers::ServerId)
                        .to(Servers::Table, Servers::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_server_pool_member_unique")
                .table(ServerPoolMembers::Table)
                .col(ServerPoolMembers::ServerPoolId)
                .col(ServerPoolMembers::ServerId)
                .unique()
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(RegistryCredentials::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(RegistryCredentials::Id)
                        .big_integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::OrganizationId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::Name)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::Registry)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::Username)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::EncryptedPassword)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(RegistryCredentials::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_registry_credential_org")
                        .from(
                            RegistryCredentials::Table,
                            RegistryCredentials::OrganizationId,
                        )
                        .to(Organizations::Table, Organizations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_registry_org_name")
                .table(RegistryCredentials::Table)
                .col(RegistryCredentials::OrganizationId)
                .col(RegistryCredentials::Name)
                .unique()
                .to_owned(),
        )
        .await?;

        for column in [
            ColumnDef::new(Servers::TagsJson)
                .text()
                .not_null()
                .default("[]")
                .to_owned(),
            ColumnDef::new(Servers::CapacityUnits)
                .integer()
                .not_null()
                .default(100)
                .to_owned(),
        ] {
            m.alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .add_column(column)
                    .to_owned(),
            )
            .await?;
        }

        for column in [
            ColumnDef::new(Applications::ServerPoolId)
                .big_integer()
                .null()
                .to_owned(),
            ColumnDef::new(Applications::ResourceUnits)
                .integer()
                .not_null()
                .default(1)
                .to_owned(),
            ColumnDef::new(Applications::WorkloadType)
                .string()
                .not_null()
                .default("single")
                .to_owned(),
            ColumnDef::new(Applications::ComposeFilePath)
                .string()
                .null()
                .to_owned(),
            ColumnDef::new(Applications::ComposeProjectName)
                .string()
                .null()
                .to_owned(),
            ColumnDef::new(Applications::RegistryCredentialId)
                .big_integer()
                .null()
                .to_owned(),
        ] {
            m.alter_table(
                Table::alter()
                    .table(Applications::Table)
                    .add_column(column)
                    .to_owned(),
            )
            .await?;
        }

        m.create_index(
            Index::create()
                .name("idx_applications_server_pool")
                .table(Applications::Table)
                .col(Applications::ServerPoolId)
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_applications_server_pool")
                .table(Applications::Table)
                .to_owned(),
        )
        .await?;

        for column in [
            Applications::RegistryCredentialId,
            Applications::ComposeProjectName,
            Applications::ComposeFilePath,
            Applications::WorkloadType,
            Applications::ResourceUnits,
            Applications::ServerPoolId,
        ] {
            m.alter_table(
                Table::alter()
                    .table(Applications::Table)
                    .drop_column(column)
                    .to_owned(),
            )
            .await?;
        }

        for column in [Servers::CapacityUnits, Servers::TagsJson] {
            m.alter_table(
                Table::alter()
                    .table(Servers::Table)
                    .drop_column(column)
                    .to_owned(),
            )
            .await?;
        }

        m.drop_table(
            Table::drop()
                .table(RegistryCredentials::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        m.drop_table(
            Table::drop()
                .table(ServerPoolMembers::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        m.drop_table(
            Table::drop()
                .table(ServerPools::Table)
                .if_exists()
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ServerPools {
    Table,
    Id,
    OrganizationId,
    Name,
    Slug,
    RequiredTagsJson,
    CreatedAt,
    UpdatedAt,
}
#[derive(DeriveIden)]
enum ServerPoolMembers {
    Table,
    Id,
    ServerPoolId,
    ServerId,
    Weight,
    CreatedAt,
}
#[derive(DeriveIden)]
enum RegistryCredentials {
    Table,
    Id,
    OrganizationId,
    Name,
    Registry,
    Username,
    EncryptedPassword,
    CreatedAt,
    UpdatedAt,
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
    TagsJson,
    CapacityUnits,
}
#[derive(DeriveIden)]
enum Applications {
    Table,
    ServerPoolId,
    ResourceUnits,
    WorkloadType,
    ComposeFilePath,
    ComposeProjectName,
    RegistryCredentialId,
}
