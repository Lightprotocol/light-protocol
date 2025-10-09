use light_client::rpc::RpcError;
use light_compressed_account::constants::REGISTERED_PROGRAM_PDA;
#[cfg(feature = "devenv")]
use light_registry::account_compression_cpi::sdk::get_registered_program_pda;
use litesvm::LiteSVM;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_pubkey::Pubkey;
use solana_sdk::pubkey;

use crate::{
    accounts::{
        registered_program_accounts::{
            registered_program_test_account_registry_program,
            registered_program_test_account_system_program,
        },
        test_accounts::NOOP_PROGRAM_ID,
    },
    utils::find_light_bin::find_light_bin,
};

// Program IDs as Pubkeys
const ACCOUNT_COMPRESSION_ID: Pubkey = pubkey!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
const LIGHT_REGISTRY_ID: Pubkey = pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
const LIGHT_COMPRESSED_TOKEN_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Creates ProgramTestContext with light protocol and additional programs.
///
/// Programs:
/// 1. light_registry program
/// 2. account_compression program
/// 3. light_compressed_token program
/// 4. light_system_program program
pub fn setup_light_programs(
    additional_programs: Option<Vec<(&'static str, Pubkey)>>,
) -> Result<LiteSVM, RpcError> {
    let program_test = LiteSVM::new().with_log_bytes_limit(Some(100_000));
    let program_test = program_test.with_compute_budget(ComputeBudget {
        compute_unit_limit: 1_400_000,
        ..Default::default()
    });
    let mut program_test = program_test.with_transaction_history(0);
    let project_root_target_deploy_path = std::env::var("SBF_OUT_DIR")
        .map_err(|_| RpcError::CustomError("SBF_OUT_DIR not set.".to_string()))?;
    // find path to bin where light cli stores program binaries.
    let light_bin_path = find_light_bin().ok_or(RpcError::CustomError(
        "Failed to find light binary path. To use light-program-test zk compression cli needs to be installed and light system programs need to be downloaded. Light system programs are downloaded the first time light test-validator is run.".to_string(),
    ))?;
    let light_bin_path = light_bin_path
        .to_str()
        .ok_or(RpcError::CustomError(format!(
            "Found invalid light binary path {:?}",
            light_bin_path
        )))?;
    let path = format!("{}/light_registry.so", light_bin_path);
    program_test
        .add_program_from_file(LIGHT_REGISTRY_ID, path.clone())
        .inspect_err(|_| {
            println!("Program light_registry bin not found in {}", path);
        })?;
    let path = format!("{}/account_compression.so", light_bin_path);
    program_test
        .add_program_from_file(ACCOUNT_COMPRESSION_ID, path.clone())
        .inspect_err(|_| {
            println!("Program account_compression bin not found in {}", path);
        })?;
    let path = format!("{}/light_compressed_token.so", light_bin_path);
    program_test
        .add_program_from_file(LIGHT_COMPRESSED_TOKEN_ID, path.clone())
        .inspect_err(|_| {
            println!("Program light_compressed_token bin not found in {}", path);
        })?;
    let path = format!("{}/spl_noop.so", light_bin_path);
    program_test
        .add_program_from_file(NOOP_PROGRAM_ID, path.clone())
        .inspect_err(|_| {
            println!("Program spl_noop bin not found in {}", path);
        })?;

    let path = format!("{}/light_system_program_pinocchio.so", light_bin_path);
    program_test
        .add_program_from_file(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID, path.clone())
        .inspect_err(|_| {
            println!(
                "Program light_system_program_pinocchio bin not found in {}",
                path
            );
        })?;

    let registered_program = registered_program_test_account_system_program();
    program_test
        .set_account(
            Pubkey::new_from_array(REGISTERED_PROGRAM_PDA),
            registered_program,
        )
        .map_err(|e| {
            RpcError::CustomError(format!("Setting registered program account failed {}", e))
        })?;
    let registered_program = registered_program_test_account_registry_program();

    #[cfg(feature = "devenv")]
    let registry_pda = get_registered_program_pda(&LIGHT_REGISTRY_ID);

    #[cfg(not(feature = "devenv"))]
    let registry_pda = {
        // Compute the PDA manually in non-devenv mode
        // This is the registered program PDA for light_registry
        Pubkey::find_program_address(
            &[b"registered_program", LIGHT_REGISTRY_ID.as_ref()],
            &ACCOUNT_COMPRESSION_ID,
        )
        .0
    };

    program_test
        .set_account(registry_pda, registered_program)
        .map_err(|e| {
            RpcError::CustomError(format!("Setting registered program account failed {}", e))
        })?;
    if let Some(programs) = additional_programs {
        for (name, id) in programs {
            let path = format!("{}/{}.so", project_root_target_deploy_path, name);
            program_test
                .add_program_from_file(id, path.clone())
                .inspect_err(|_| {
                    println!("Program {} bin not found in {}", name, path);
                })?;
        }
    }
    Ok(program_test)
}
