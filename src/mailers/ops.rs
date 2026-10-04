#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

static alert: Dir<'_> = include_dir!("src/mailers/ops/alert");

pub struct OpsMailer {}
impl Mailer for OpsMailer {}

impl OpsMailer {
    pub async fn send_alert(
        ctx: &AppContext,
        to: &str,
        severity: &str,
        subject: &str,
        message: &str,
    ) -> Result<()> {
        Self::mail_template(
            ctx,
            &alert,
            mailer::Args {
                to: to.to_string(),
                locals: json!({
                    "severity": severity,
                    "subject": subject,
                    "message": message,
                    "host": ctx.config.server.full_url(),
                }),
                ..Default::default()
            },
        )
        .await?;
        Ok(())
    }
}
