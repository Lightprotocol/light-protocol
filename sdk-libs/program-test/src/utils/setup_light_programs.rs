use light_client::rpc::RpcError;
use light_sdk::utils::get_registered_program_pda;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::pubkey::Pubkey;

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

/// Creates ProgramTestContext with light protocol and additional programs.
///
/// Programs:
/// 1. light_registry program
/// 2. account_compression program
/// 3. light_compressed_token program
/// 4. light_system_program program
pub async fn setup_light_programs(
    additional_programs: Option<Vec<(&'static str, Pubkey)>>,
) -> Result<ProgramTestContext, RpcError> {
    let mut program_test = ProgramTest::default();
    let sbf_path = std::env::var("SBF_OUT_DIR")
        .map_err(|_| RpcError::CustomError("SBF_OUT_DIR not set.".to_string()))?;
    // find path to bin where light cli stores program binaries.
    let path = find_light_bin().ok_or(RpcError::CustomError(
        "Failed to find light binary path. To use light-program-test zk compression cli needs to be installed and light system programs need to be downloaded. Light system programs are downloaded the first time light test-validator is run.".to_string(),
    ))?;
    std::env::set_var(
        "SBF_OUT_DIR",
        path.to_str().ok_or(RpcError::CustomError(format!(
            "Found invalid light binary path {:?}",
            path
        )))?,
    );
    program_test.add_program("light_registry", light_registry::ID, None);
    program_test.add_program("account_compression", account_compression::ID, None);
    program_test.add_program("light_compressed_token", light_compressed_token::ID, None);
    program_test.add_program(
        "light_system_program_pinocchio",
        light_system_program::ID,
        None,
    );
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    std::env::set_var("SBF_OUT_DIR", sbf_path);
    let registered_program = registered_program_test_account_system_program();
    program_test.add_account(
        get_registered_program_pda(&light_system_program::ID),
        registered_program,
    );
    let registered_program = registered_program_test_account_registry_program();
    program_test.add_account(
        get_registered_program_pda(&light_registry::ID),
        registered_program,
    );
    if let Some(programs) = additional_programs {
        for (name, id) in programs {
            program_test.add_program(name, id, None);
        }
    }
    program_test.set_compute_max_units(1_400_000u64);
    Ok(program_test.start_with_context().await)
}
