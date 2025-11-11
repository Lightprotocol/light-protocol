use std::{
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use tracing::info;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    helpers::get_project_root,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);

pub async fn spawn_prover() {
    if let Some(_project_root) = get_project_root() {
        let prover_path: &str = {
            #[cfg(feature = "devenv")]
            {
                &format!("{}/{}", _project_root.trim(), "cli/test_bin/run")
            }
            #[cfg(not(feature = "devenv"))]
            {
                println!("Running in production mode, using prover binary");
                "light"
            }
        };

        if !health_check(10, 1).await && !IS_LOADING.load(Ordering::Relaxed) {
            IS_LOADING.store(true, Ordering::Relaxed);

            let command = Command::new(prover_path)
                .arg("start-prover")
                .spawn()
                .expect("Failed to start prover process");

            let _ = command.wait_with_output();

            let health_result = health_check(120, 1).await;
            if health_result {
                info!("Prover started successfully");
            } else {
                panic!("Failed to start prover, health check failed.");
            }
        }
    } else {
        panic!("Failed to find project root.");
    };
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let client = reqwest::Client::new();
    let mut result = false;
    for _ in 0..retries {
        match client
            .get(format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
            .send()
            .await
        {
            Ok(_) => {
                result = true;
                break;
            }
            Err(_) => {
                sleep(Duration::from_secs(timeout as u64));
            }
        }
    }
    result
}
