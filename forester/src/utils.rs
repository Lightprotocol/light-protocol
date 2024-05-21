use log::info;
use std::process::Command;
use sysinfo::{Signal, System};

pub fn u8_arr_to_hex_string(arr: &[u8]) -> String {
    arr.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
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

pub async fn spawn_test_validator() {
    info!("Starting validator...");
    let path = "../cli/test_bin/run test-validator --skip-indexer --skip-prover";
    Command::new("sh")
        .arg("-c")
        .arg(path)
        .spawn()
        .expect("Failed to start server process");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    info!("Validator started successfully");
}

pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}
