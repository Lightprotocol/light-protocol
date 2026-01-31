use std::process::{Command, Stdio};

use light_prover_client::helpers::get_project_root;

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub enable_indexer: bool,
    pub enable_prover: bool,
    pub wait_time: u64,
    /// Non-upgradeable programs: (program_id, program_path)
    pub sbf_programs: Vec<(String, String)>,
    /// Upgradeable programs: (program_id, program_path, upgrade_authority)
    /// Use this when the program needs a valid upgrade authority (e.g., for compression config)
    pub upgradeable_programs: Vec<(String, String, String)>,
    pub limit_ledger_size: Option<u64>,
    /// Use surfpool instead of solana-test-validator
    pub use_surfpool: bool,
}

impl Default for LightValidatorConfig {
    fn default() -> Self {
        Self {
            enable_indexer: false,
            enable_prover: false,
            wait_time: 35,
            sbf_programs: vec![],
            upgradeable_programs: vec![],
            limit_ledger_size: None,
            use_surfpool: true,
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

        for upgradeable_program in config.upgradeable_programs.iter() {
            path.push_str(&format!(
                " --upgradeable-program {} {} {}",
                upgradeable_program.0, upgradeable_program.1, upgradeable_program.2
            ));
        }

        if !config.enable_prover {
            path.push_str(" --skip-prover");
        }

        if config.use_surfpool {
            path.push_str(" --use-surfpool");
        }

        println!("Starting validator with command: {}", path);

        if config.use_surfpool {
            // The CLI starts surfpool, prover, and photon, then exits once all
            // services are ready. Wait for it to finish so we know everything
            // is up before the test proceeds.
            let mut child = Command::new("sh")
                .arg("-c")
                .arg(path)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start server process");
            let status = child.wait().expect("Failed to wait for CLI process");
            assert!(status.success(), "CLI exited with error: {}", status);
        } else {
            let child = Command::new("sh")
                .arg("-c")
                .arg(path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start server process");
            std::mem::drop(child);
            tokio::time::sleep(tokio::time::Duration::from_secs(config.wait_time)).await;
        }
    }
}
