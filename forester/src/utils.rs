use light_registry::protocol_config::state::{ProtocolConfig, ProtocolConfigPda};
use light_registry::utils::get_protocol_config_pda_address;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, info};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
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
            wait_time: 35,
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

    debug!("Starting validator with command: {}", path);

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
pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

pub fn u8_arr_to_hex_string(arr: &[u8]) -> String {
    arr.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

pub async fn get_protocol_config<R: RpcConnection>(rpc: &mut R) -> ProtocolConfig {
    let authority_pda = get_protocol_config_pda_address();
    let protocol_config_account = rpc
        .get_anchor_account::<ProtocolConfigPda>(&authority_pda.0)
        .await
        .unwrap()
        .unwrap();
    info!("Protocol config account: {:?}", protocol_config_account);
    protocol_config_account.config
}

pub fn get_current_system_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}
