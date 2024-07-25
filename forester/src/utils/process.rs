use log::debug;
use std::process::Command;
use sysinfo::{Signal, System};

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub path: String,
    pub enable_indexer: bool,
    pub enable_prover: bool,
    pub enable_forester: bool,
    pub wait_time: u64,
}

impl Default for LightValidatorConfig {
    fn default() -> Self {
        Self {
            path: "../cli/test_bin/run test-validator".to_string(),
            enable_indexer: false,
            enable_prover: false,
            enable_forester: false,
            wait_time: 30,
        }
    }
}

pub async fn spawn_validator(config: LightValidatorConfig) {
    debug!("Starting validator...");
    debug!("Config: {:?}", config);
    let mut path = config.path.clone();
    if !config.enable_indexer {
        path.push_str(" --skip-indexer");
    }
    if !config.enable_prover {
        path.push_str(" --skip-prover");
    }
    if !config.enable_forester {
        path.push_str(" --skip-forester");
    }

    Command::new("sh")
        .arg("-c")
        .arg(path)
        .spawn()
        .expect("Failed to start server process");
    tokio::time::sleep(tokio::time::Duration::from_secs(config.wait_time)).await;
    debug!("Validator started successfully");
}

pub async fn restart_photon() {
    kill_photon();
    Command::new("photon")
        .spawn()
        .expect("Failed to start server process");
}

pub fn kill_photon() {
    let mut system = System::new_all();
    system.refresh_all();

    for process in system.processes().values() {
        if process.name() == "photon" {
            process.kill_with(Signal::Term);
        }
    }
}
