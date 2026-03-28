use std::{
    process::{Child, Command},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use tokio::time::sleep;
use tracing::info;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    helpers::get_project_root,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);
const STARTUP_HEALTH_CHECK_RETRIES: usize = 300;

pub(crate) fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("failed to build HTTP client")
}

fn monitor_prover_child(mut child: Child) {
    std::thread::spawn(move || match child.wait() {
        Ok(status) => tracing::debug!(?status, "prover launcher exited"),
        Err(error) => tracing::warn!(?error, "failed to wait on prover launcher"),
    });
}

pub async fn spawn_prover() {
    if let Some(_project_root) = get_project_root() {
        let prover_path = {
            #[cfg(feature = "devenv")]
            {
                format!("{}/{}", _project_root.trim(), "cli/test_bin/run")
            }
            #[cfg(not(feature = "devenv"))]
            {
                println!("Running in production mode, using prover binary");
                "light".to_string()
            }
        };

        if health_check(10, 1).await {
            return;
        }

        if IS_LOADING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            if health_check(STARTUP_HEALTH_CHECK_RETRIES, 1).await {
                return;
            }
            panic!("Failed to start prover, health check failed.");
        }

        let mut child = Command::new(&prover_path)
            .arg("start-prover")
            .spawn()
            .unwrap_or_else(|error| panic!("Failed to start prover process: {error}"));

        let health_result = health_check(STARTUP_HEALTH_CHECK_RETRIES, 1).await;
        IS_LOADING.store(false, Ordering::Release);

        if health_result {
            monitor_prover_child(child);
            info!("Prover started successfully");
        } else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Failed to start prover, health check failed.");
        }
    } else {
        panic!("Failed to find project root.");
    };
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let client = build_http_client();
    let timeout_duration = Duration::from_secs(timeout as u64);

    for attempt in 0..retries {
        match client
            .get(format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
            .timeout(timeout_duration)
            .send()
            .await
        {
            Ok(_) => return true,
            Err(_) => {
                if attempt + 1 < retries {
                    sleep(timeout_duration).await;
                }
            }
        }
    }

    false
}
