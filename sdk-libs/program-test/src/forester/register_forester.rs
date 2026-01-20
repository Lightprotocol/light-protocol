use light_client::rpc::{Rpc, RpcError};
// When devenv is enabled, use light_registry's SDK and types
#[cfg(feature = "devenv")]
use light_registry::{
    protocol_config::state::ProtocolConfigPda, sdk::create_finalize_registration_instruction,
    sdk::create_register_forester_epoch_pda_instruction, utils::get_protocol_config_pda_address,
    ForesterConfig,
};
use solana_sdk::signature::{Keypair, Signer};

// When devenv is NOT enabled, use local registry_sdk
#[cfg(not(feature = "devenv"))]
use crate::registry_sdk::{
    create_finalize_registration_instruction, create_register_forester_epoch_pda_instruction,
    deserialize_protocol_config_pda, get_protocol_config_pda_address, ForesterConfig,
};
use crate::{
    accounts::test_keypairs::TestKeypairs, program_test::TestRpc,
    utils::register_test_forester::register_test_forester,
};

/// Registers a forester and sets up the epoch PDA for compress_and_close operations
pub async fn register_forester_for_compress_and_close<R: Rpc + TestRpc>(
    rpc: &mut R,
    forester_keypair: &Keypair,
) -> Result<(), RpcError> {
    // Get test keypairs for governance authority
    let test_keypairs = TestKeypairs::program_test_default();

    // 1. Register the base forester account
    register_test_forester(
        rpc,
        &test_keypairs.governance_authority,
        &forester_keypair.pubkey(),
        ForesterConfig::default(),
    )
    .await?;

    // 2. Get protocol config
    let (protocol_config_pda, _) = get_protocol_config_pda_address();

    #[cfg(feature = "devenv")]
    let protocol_config = {
        let protocol_config_pda_data = rpc
            .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
            .await?
            .ok_or_else(|| RpcError::CustomError("Protocol config not found".to_string()))?;
        protocol_config_pda_data.config
    };

    #[cfg(not(feature = "devenv"))]
    let protocol_config = {
        let protocol_config_account = rpc
            .get_account(protocol_config_pda)
            .await?
            .ok_or_else(|| RpcError::CustomError("Protocol config not found".to_string()))?;
        let protocol_config_pda_data =
            deserialize_protocol_config_pda(&protocol_config_account.data).map_err(|e| {
                RpcError::CustomError(format!("Failed to deserialize protocol config: {}", e))
            })?;
        protocol_config_pda_data.config
    };

    // 3. Get current slot to determine if we need to advance past registration phase
    let current_slot = rpc.get_slot().await?;
    let epoch = 0;

    let instruction = create_register_forester_epoch_pda_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        epoch,
    );
    let signature = rpc
        .create_and_send_transaction(
            &[instruction],
            &forester_keypair.pubkey(),
            &[forester_keypair],
        )
        .await?;
    rpc.confirm_transaction(signature).await?;

    // If we're still in registration phase (first 2 slots), advance past it
    if current_slot < protocol_config.registration_phase_length {
        rpc.warp_to_slot(protocol_config.registration_phase_length + 1)?;
    }

    // 4. Finalize registration
    let ix = create_finalize_registration_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        epoch,
    );

    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[forester_keypair])
        .await?;

    Ok(())
}
