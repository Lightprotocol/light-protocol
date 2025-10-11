use std::{
    fmt::{Display, Formatter},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use tracing::info;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    helpers::get_project_root,
    proof_type::ProofType,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProverMode {
    Rpc,
    Forester,
    ForesterTest,
    Full,
    FullTest,
}

impl Display for ProverMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProverMode::Rpc => "rpc",
                ProverMode::Forester => "forester",
                ProverMode::ForesterTest => "forester-test",
                ProverMode::Full => "full",
                ProverMode::FullTest => "full-test",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProverConfig {
    pub run_mode: Option<ProverMode>,
    pub circuits: Vec<ProofType>,
}

impl Default for ProverConfig {
    #[cfg(feature = "devenv")]
    fn default() -> Self {
        Self {
            run_mode: Some(ProverMode::ForesterTest),
            circuits: vec![],
        }
    }
    #[cfg(not(feature = "devenv"))]
    fn default() -> Self {
        Self {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }
    }
}

impl ProverConfig {
    pub fn rpc_no_restart() -> Self {
        Self {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }
    }
}

pub async fn spawn_prover(config: ProverConfig) {
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

            let mut command = Command::new(prover_path);
            command.arg("start-prover");

            if let Some(ref mode) = config.run_mode {
                command.arg("--run-mode").arg(mode.to_string());
            }

            for circuit in config.circuits.clone() {
                command.arg("--circuit").arg(circuit.to_string());
            }

            println!("Starting prover with command: {:?}", command);

            let _ = command
                .spawn()
                .expect("Failed to start prover process")
                .wait();

            let health_result = health_check(120, 1).await;
            if health_result {
                info!("Prover started successfully");
            } else {
                panic!("Failed to start prover, health check failed.");
            }
        }
        #[cfg(not(feature = "devenv"))]
        {
            "light"
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
