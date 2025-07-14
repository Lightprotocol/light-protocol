use light_client::rpc::{Rpc, RpcError};
use light_registry::{
    sdk::create_update_forester_pda_instruction, utils::get_forester_pda, ForesterConfig,
    ForesterPda,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

pub async fn update_test_forester<R: Rpc>(
    rpc: &mut R,
    forester_authority: &Keypair,
    derivation_key: &Pubkey,
    new_forester_authority: Option<&Keypair>,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let mut pre_account_state = rpc
        .get_anchor_account::<ForesterPda>(&get_forester_pda(derivation_key).0)
        .await?
        .unwrap();
    let (signers, new_forester_authority) = if let Some(new_authority) = new_forester_authority {
        pre_account_state.authority = new_authority.pubkey();

        (
            vec![forester_authority, &new_authority],
            Some(new_authority.pubkey()),
        )
    } else {
        (vec![forester_authority], None)
    };
    let ix = create_update_forester_pda_instruction(
        &forester_authority.pubkey(),
        derivation_key,
        new_forester_authority,
        Some(config),
    );

    rpc.create_and_send_transaction(&[ix], &forester_authority.pubkey(), &signers)
        .await?;

    pre_account_state.config = config;
    assert_registered_forester(rpc, derivation_key, pre_account_state).await
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
