use light_client::rpc::{Rpc, RpcError};
use light_registry::{
    sdk::create_register_forester_instruction, utils::get_forester_pda, ForesterConfig, ForesterPda,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
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
