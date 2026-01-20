use light_client::rpc::{Rpc, RpcError};
// When devenv is enabled, use light_registry's types and SDK
#[cfg(feature = "devenv")]
use light_registry::{
    sdk::create_register_forester_instruction, utils::get_forester_pda, ForesterConfig, ForesterPda,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

// When devenv is NOT enabled, use local registry_sdk
#[cfg(not(feature = "devenv"))]
use crate::registry_sdk::{
    create_register_forester_instruction, deserialize_forester_pda, get_forester_pda,
    ForesterConfig, ForesterPda,
};

/// Creates and asserts forester account creation.
pub async fn register_test_forester<R: Rpc>(
    rpc: &mut R,
    governance_authority: &Keypair,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let ix = create_register_forester_instruction(
        &governance_authority.pubkey(),
        &governance_authority.pubkey(),
        forester_authority,
        config,
    );
    rpc.create_and_send_transaction(
        &[ix],
        &governance_authority.pubkey(),
        &[governance_authority],
    )
    .await?;
    assert_registered_forester(
        rpc,
        forester_authority,
        ForesterPda {
            authority: *forester_authority,
            config,
            active_weight: 1,
            ..Default::default()
        },
    )
    .await
}

#[cfg(feature = "devenv")]
async fn assert_registered_forester<R: Rpc>(
    rpc: &mut R,
    forester: &Pubkey,
    expected_account: ForesterPda,
) -> Result<(), RpcError> {
    let pda = get_forester_pda(forester).0;
    let account_data = rpc.get_anchor_account::<ForesterPda>(&pda).await?.unwrap();
    if account_data != expected_account {
        return Err(RpcError::AssertRpcError(format!(
            "Expected account data: {:?}, got: {:?}",
            expected_account, account_data
        )));
    }
    Ok(())
}

#[cfg(not(feature = "devenv"))]
async fn assert_registered_forester<R: Rpc>(
    rpc: &mut R,
    forester: &Pubkey,
    expected_account: ForesterPda,
) -> Result<(), RpcError> {
    let pda = get_forester_pda(forester).0;
    let account = rpc
        .get_account(pda)
        .await?
        .ok_or_else(|| RpcError::CustomError(format!("Forester PDA account not found: {}", pda)))?;
    let account_data = deserialize_forester_pda(&account.data)
        .map_err(|e| RpcError::CustomError(format!("Failed to deserialize ForesterPda: {}", e)))?;
    if account_data != expected_account {
        return Err(RpcError::AssertRpcError(format!(
            "Expected account data: {:?}, got: {:?}",
            expected_account, account_data
        )));
    }
    Ok(())
}
