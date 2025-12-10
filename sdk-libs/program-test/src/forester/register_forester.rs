use light_client::rpc::{Rpc, RpcError};
use solana_sdk::signature::{Keypair, Signer};

use super::{
    instructions::{
        create_finalize_registration_instruction, create_register_forester_epoch_pda_instruction,
    },
    types::{get_protocol_config_pda_address, ForesterConfig, ProtocolConfigPda},
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
    let test_keypairs = TestKeypairs::program_test_default();

    register_test_forester(
        rpc,
        &test_keypairs.governance_authority,
        &forester_keypair.pubkey(),
        ForesterConfig::default(),
    )
    .await?;

    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
        .await?
        .ok_or_else(|| RpcError::CustomError("Protocol config not found".to_string()))?
        .config;

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

    if current_slot < protocol_config.registration_phase_length {
        rpc.warp_to_slot(protocol_config.registration_phase_length + 1)?;
    }

    let ix = create_finalize_registration_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        epoch,
    );

    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[forester_keypair])
        .await?;

    Ok(())
}
