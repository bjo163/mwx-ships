use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(WebhookDeliveries::Table)
                .add_column(ColumnDef::new(WebhookDeliveries::DedupeKey).string().null())
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_webhook_delivery_intent_dedupe")
                .table(WebhookDeliveries::Table)
                .col(WebhookDeliveries::DedupeKey)
                .unique()
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_index(
            Index::drop()
                .name("idx_webhook_delivery_intent_dedupe")
                .table(WebhookDeliveries::Table)
                .to_owned(),
        )
        .await?;

        m.alter_table(
            Table::alter()
                .table(WebhookDeliveries::Table)
                .drop_column(WebhookDeliveries::DedupeKey)
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WebhookDeliveries {
    Table,
    DedupeKey,
}
