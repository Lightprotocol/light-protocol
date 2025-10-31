use std::process::{Command, Stdio};

use light_prover_client::helpers::get_project_root;

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub enable_indexer: bool,
    pub enable_prover: bool,
    pub wait_time: u64,
    pub sbf_programs: Vec<(String, String)>,
    pub limit_ledger_size: Option<u64>,
    pub grpc_port: Option<u64>,
}

impl Default for LightValidatorConfig {
    fn default() -> Self {
        Self {
            enable_indexer: false,
            enable_prover: false,
            wait_time: 35,
            sbf_programs: vec![],
            limit_ledger_size: None,
            grpc_port: None,
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

        if !config.enable_prover {
            path.push_str(" --skip-prover");
        }

        if let Some(grpc_port) = config.grpc_port {
            path.push_str(&format!(" --grpc-port {}", grpc_port));
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
