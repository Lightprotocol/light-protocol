use std::fmt::Display;
use serde_json::json;

pub enum Severity {
    Critical,
    Error,
    Warning,
    Info,
}

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "critical"),
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

// summary: A brief text summary of the event, used to generate the
// summaries/titles of any associated alerts.
// The maximum permitted length of this property is 1024 characters.
// severity: The perceived severity of the status the event is describing
// with respect to the affected system. This can be critical, error, warning or info.
// source: The unique location of the affected system, preferably a hostname or FQDN.
pub async fn send_alert(summary: &str, severity: Severity, source: &str) {
    // get routing key from env variable
    let routing_key = std::env::var("PAGERDUTY_ROUTING_KEY").unwrap();

    let request = reqwest::Client::new()
        .post("https://events.pagerduty.com/v2/enqueue")
        .header("Content-Type", "application/json")
        .json(&json!({
            "payload": {
                "summary": summary,
                "severity": severity.to_string(),
                "source": source
            },
            "routing_key": routing_key,
            "event_action": "trigger"
        }));
    let response = request.send().await;
    println!("Response: {:?}", response);
}