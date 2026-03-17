use std::process::Stdio;

use light_prover_client::helpers::get_project_root;
use tokio::process::Command;

/// Configuration for an upgradeable program to deploy to the validator.
#[derive(Debug, Clone)]
pub struct UpgradeableProgramConfig {
    /// The program ID (public key) of the program
    pub program_id: String,
    /// Path to the compiled program (.so file)
    pub program_path: String,
    /// The upgrade authority for the program
    pub upgrade_authority: String,
}

impl UpgradeableProgramConfig {
    pub fn new(program_id: String, program_path: String, upgrade_authority: String) -> Self {
        Self {
            program_id,
            program_path,
            upgrade_authority,
        }
    }
}

#[derive(Debug)]
pub struct LightValidatorConfig {
    pub enable_indexer: bool,
    pub enable_prover: bool,
    pub wait_time: u64,
    /// Non-upgradeable programs: (program_id, program_path)
    pub sbf_programs: Vec<(String, String)>,
    /// Upgradeable programs to deploy with a valid upgrade authority.
    /// Use this when the program needs a valid upgrade authority (e.g., for compression config).
    pub upgradeable_programs: Vec<UpgradeableProgramConfig>,
    pub limit_ledger_size: Option<u64>,
    /// Use surfpool instead of solana-test-validator
    pub use_surfpool: bool,
    /// Additional arguments to pass to the validator (e.g., "--account <ADDRESS> <FILEPATH>")
    pub validator_args: Vec<String>,
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
            validator_args: vec![],
        }
    }
}

pub async fn spawn_validator(config: LightValidatorConfig) {
    if let Some(project_root) = get_project_root() {
        let project_root = project_root.trim_end_matches(['\n', '\r']);
        let executable = format!("{}/cli/test_bin/run", project_root);
        let mut args = vec!["test-validator".to_string()];
        if !config.enable_indexer {
            args.push("--skip-indexer".to_string());
        }

        if let Some(limit_ledger_size) = config.limit_ledger_size {
            args.push("--limit-ledger-size".to_string());
            args.push(limit_ledger_size.to_string());
        }

        for sbf_program in config.sbf_programs.iter() {
            args.push("--sbf-program".to_string());
            args.push(sbf_program.0.clone());
            args.push(sbf_program.1.clone());
        }

        for upgradeable_program in config.upgradeable_programs.iter() {
            args.push("--upgradeable-program".to_string());
            args.push(upgradeable_program.program_id.clone());
            args.push(upgradeable_program.program_path.clone());
            args.push(upgradeable_program.upgrade_authority.clone());
        }

        if !config.enable_prover {
            args.push("--skip-prover".to_string());
        }

        if config.use_surfpool {
            args.push("--use-surfpool".to_string());
        }

        for arg in config.validator_args.iter() {
            args.push(arg.clone());
        }

        println!(
            "Starting validator with command: {} {}",
            executable,
            args.join(" ")
        );

        if config.use_surfpool {
            // The CLI starts surfpool, prover, and photon, then exits once all
            // services are ready. Wait for it to finish so we know everything
            // is up before the test proceeds.
            let mut child = Command::new(&executable)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start server process");
            let status = child.wait().await.expect("Failed to wait for CLI process");
            assert!(status.success(), "CLI exited with error: {}", status);
        } else {
            let mut child = Command::new(&executable)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start server process");
            tokio::time::sleep(tokio::time::Duration::from_secs(config.wait_time)).await;
            if let Some(status) = child.try_wait().expect("Failed to poll validator process") {
                assert!(
                    status.success(),
                    "Validator exited early with error: {}",
                    status
                );
            }
        }
    }
}
