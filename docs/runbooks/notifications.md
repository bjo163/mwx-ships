# Runbook — Operational Alerts

Moonships persists operational state even when no external notification channel is configured.

## Channels
- `MOONSHIPS_ALERT_WEBHOOK_URL` — HTTP(S) JSON webhook.
- `MOONSHIPS_ALERT_EMAIL` — email through the configured Loco mailer.
- `MOONSHIPS_ALERT_COOLDOWN_SECS` — dedupe cooldown, default 900 seconds.

## Alert classes
- repeated deployment failure
- target offline/error
- target disk pressure
- stale deployment execution lease
- backup failure

Notification delivery failure never changes the result of the deployment/backup operation that generated it. Delivery failures are persisted as operational events.

## Alert storm control
A stable event-kind + subject fingerprint owns a cooldown window. Repeated alerts inside the window are suppressed but the underlying health/deployment records continue to update.
