use std::time::Duration;

use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct PagerDutyPayload {
    routing_key: String,
    event_action: String,
    payload: PagerDutyAlertPayload,
}

#[derive(Debug, Serialize)]
struct PagerDutyAlertPayload {
    summary: String,
    severity: String,
    source: String,
}

pub async fn send_pagerduty_alert(
    routing_key: &str,
    summary: &str,
    severity: &str,
    source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let payload = PagerDutyPayload {
        routing_key: routing_key.to_string(),
        event_action: "trigger".to_string(),
        payload: PagerDutyAlertPayload {
            summary: summary.to_string(),
            severity: severity.to_string(),
            source: source.to_string(),
        },
    };

    let response = client
        .post("https://events.pagerduty.com/v2/enqueue")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to send PagerDuty alert. Status: {}, Body: {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    Ok(())
}
