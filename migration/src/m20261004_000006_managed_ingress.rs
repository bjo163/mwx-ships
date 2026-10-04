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
                    ColumnDef::new(Applications::ActiveRuntimeName)
                        .string()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .add_column(
                    ColumnDef::new(Applications::CandidateRuntimeName)
                        .string()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Domains::Table)
                .add_column(
                    ColumnDef::new(Domains::VerificationStatus)
                        .string()
                        .not_null()
                        .default("pending"),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Domains::Table)
                .add_column(
                    ColumnDef::new(Domains::VerifiedAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Domains::Table)
                .add_column(
                    ColumnDef::new(Domains::TlsStatus)
                        .string()
                        .not_null()
                        .default("pending"),
                )
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Domains::Table)
                .add_column(ColumnDef::new(Domains::LastError).text().null())
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            Domains::LastError,
            Domains::TlsStatus,
            Domains::VerifiedAt,
            Domains::VerificationStatus,
        ] {
            m.alter_table(
                Table::alter()
                    .table(Domains::Table)
                    .drop_column(column)
                    .to_owned(),
            )
            .await?;
        }

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .drop_column(Applications::CandidateRuntimeName)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(Applications::Table)
                .drop_column(Applications::ActiveRuntimeName)
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Applications {
    Table,
    ActiveRuntimeName,
    CandidateRuntimeName,
}

#[derive(DeriveIden)]
enum Domains {
    Table,
    VerificationStatus,
    VerifiedAt,
    TlsStatus,
    LastError,
}
