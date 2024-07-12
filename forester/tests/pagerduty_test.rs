use forester::alerts::{send_alert, Severity};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_address_tree_rollover() {
    // send_alert("test_address_tree_rollover info", Severity::Info, "test_address_tree_rollover").await;
    // send_alert("test_address_tree_rollover warning", Severity::Warning, "test_address_tree_rollover").await;
    // send_alert("test_address_tree_rollover error", Severity::Error, "test_address_tree_rollover").await;
    send_alert("test_address_tree_rollover critical", Severity::Critical, "test_address_tree_rollover").await;
}