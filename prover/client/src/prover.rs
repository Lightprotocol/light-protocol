use std::{
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use tracing::info;

#[cfg(feature = "devenv")]
use crate::helpers::get_project_root;
use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    errors::ProverClientError,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);

pub async fn spawn_prover() -> Result<(), ProverClientError> {
    #[cfg(feature = "devenv")]
    let project_root = get_project_root().ok_or(ProverClientError::ProjectRootNotFound)?;

    let prover_path: String = {
        #[cfg(feature = "devenv")]
        {
            format!("{}/{}", project_root.trim(), "cli/test_bin/run")
        }
        #[cfg(not(feature = "devenv"))]
        {
            println!("Running in production mode, using prover binary");
            "light".to_string()
        }
    };

    if health_check(10, 1).await {
        return Ok(());
    }

    let loading_guard = IS_LOADING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok();

    if !loading_guard {
        return if health_check(120, 1).await {
            Ok(())
        } else {
            Err(ProverClientError::HealthCheckFailed)
        };
    }

    let spawn_result = async {
        let command = Command::new(&prover_path)
            .arg("start-prover")
            .spawn()
            .map_err(|error| ProverClientError::ProcessStart(error.to_string()))?;

        command
            .wait_with_output()
            .map_err(|error| ProverClientError::ProcessWait(error.to_string()))?;

        if health_check(120, 1).await {
            info!("Prover started successfully");
            Ok(())
        } else {
            Err(ProverClientError::HealthCheckFailed)
        }
    }
    .await;

    IS_LOADING.store(false, Ordering::Release);

    spawn_result
}

pub async fn spawn_prover_or_log() {
    if let Err(error) = spawn_prover().await {
        tracing::error!("{}", error);
    }
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
