use std::process::{Command, Stdio};

use light_prover_client::helpers::get_project_root;
pub use light_prover_client::prover::ProverConfig;

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub enable_indexer: bool,
    pub prover_config: Option<ProverConfig>,
    pub wait_time: u64,
    pub sbf_programs: Vec<(String, String)>,
    pub limit_ledger_size: Option<u64>,
}

impl Default for LightValidatorConfig {
    fn default() -> Self {
        Self {
            enable_indexer: false,
            prover_config: None,
            wait_time: 35,
            sbf_programs: vec![],
            limit_ledger_size: None,
        }
    }
}

pub async fn spawn_validator(config: LightValidatorConfig) {
    if let Some(project_root) = get_project_root() {
        let path = "cli/test_bin/run test-validator";
        let mut path = format!("{}/{}", project_root.trim(), path);
        if !config.enable_indexer {
            path.push_str(" --skip-indexer");
        }

        if let Some(limit_ledger_size) = config.limit_ledger_size {
            path.push_str(&format!(" --limit-ledger-size {}", limit_ledger_size));
        }

        for sbf_program in config.sbf_programs.iter() {
            path.push_str(&format!(
                " --sbf-program {} {}",
                sbf_program.0, sbf_program.1
            ));
        }

        if let Some(prover_config) = config.prover_config {
            prover_config.circuits.iter().for_each(|circuit| {
                path.push_str(&format!(" --circuit {}", circuit));
            });
            if let Some(prover_mode) = prover_config.run_mode {
                path.push_str(&format!(" --prover-run-mode {}", prover_mode));
            }
        } else {
            path.push_str(" --skip-prover");
        }

        println!("Starting validator with command: {}", path);

        let child = Command::new("sh")
            .arg("-c")
            .arg(path)
            .stdin(Stdio::null()) // Detach from stdin
            .stdout(Stdio::null()) // Detach from stdout
            .stderr(Stdio::null()) // Detach from stderr
            .spawn()
            .expect("Failed to start server process");

        // Explicitly `drop` the process to ensure we don't wait on it
        std::mem::drop(child);

        tokio::time::sleep(tokio::time::Duration::from_secs(config.wait_time)).await;
    }
}
