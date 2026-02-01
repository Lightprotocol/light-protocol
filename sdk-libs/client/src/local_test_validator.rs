use std::process::{Command, Stdio};

use light_prover_client::helpers::get_project_root;

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
            validator_args: vec![],
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
                upgradeable_program.program_id,
                upgradeable_program.program_path,
                upgradeable_program.upgrade_authority
            ));
        }

        if !config.enable_prover {
            path.push_str(" --skip-prover");
        }

        for arg in config.validator_args.iter() {
            path.push_str(&format!(" {}", arg));
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
