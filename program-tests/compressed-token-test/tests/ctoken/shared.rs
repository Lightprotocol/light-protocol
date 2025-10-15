// Re-export all necessary imports for test modules
pub use light_compressed_token_sdk::instructions::{
    close::{close_account, close_compressible_account},
    create_associated_token_account::derive_ctoken_ata,
    create_token_account,
};
pub use light_compressible::rent::{RentConfig, SLOTS_PER_EPOCH};
pub use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
pub use light_program_test::{
    forester::compress_and_close_forester, program_test::TestRpc, LightProgramTest,
    ProgramTestConfig,
};
pub use light_test_utils::{
    assert_close_token_account::assert_close_token_account,
    assert_create_token_account::{assert_create_token_account, CompressibleData},
    assert_transfer2::assert_transfer2_compress,
    Rpc, RpcError,
};
pub use light_token_client::{
    actions::transfer2::compress, instructions::transfer2::CompressInput,
};
pub use serial_test::serial;
pub use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
pub use solana_system_interface::instruction::create_account;

/// Shared test context for account operations
pub struct AccountTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub mint_pubkey: Pubkey,
    pub owner_keypair: Keypair,
    pub token_account_keypair: Keypair,
    pub compressible_config: Pubkey,
    pub rent_sponsor: Pubkey,
    pub compression_authority: Pubkey,
}

/// Set up test environment with common accounts and context
pub async fn setup_account_test() -> Result<AccountTestContext, RpcError> {
    let rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let mint_pubkey = Pubkey::new_unique();
    let owner_keypair = Keypair::new();
    let token_account_keypair = Keypair::new();

    Ok(AccountTestContext {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        compression_authority: rpc
            .test_accounts
            .funding_pool_config
            .compression_authority_pda,
        rpc,
        payer,
        mint_pubkey,
        owner_keypair,
        token_account_keypair,
    })
}

/// Create destination account for testing account closure
pub async fn setup_destination_account(
    rpc: &mut LightProgramTest,
) -> Result<(Keypair, u64), RpcError> {
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Fund destination account
    rpc.context
        .airdrop(&destination_pubkey, 1_000_000)
        .map_err(|_| RpcError::AssertRpcError("Failed to airdrop to destination".to_string()))?;

    let initial_lamports = rpc.get_account(destination_pubkey).await?.unwrap().lamports;

    Ok((destination_keypair, initial_lamports))
}

pub async fn create_and_assert_token_account(
    context: &mut AccountTestContext,
    compressible_data: CompressibleData,
    name: &str,
) {
    println!("Account creation initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: compressible_data.rent_sponsor,
                pre_pay_num_epochs: compressible_data.num_prepaid_epochs,
                lamports_per_write: compressible_data.lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: compressible_data.account_version,
            },
        )
        .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await
        .unwrap();

    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        Some(compressible_data),
    )
    .await;
}

/// Create token account expecting failure with specific error code
pub async fn create_and_assert_token_account_fails(
    context: &mut AccountTestContext,
    compressible_data: CompressibleData,
    name: &str,
    expected_error_code: u32,
) {
    println!(
        "Account creation (expecting failure) initiated for: {}",
        name
    );

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let create_token_account_ix =
        light_compressed_token_sdk::instructions::create_compressible_token_account(
            light_compressed_token_sdk::instructions::CreateCompressibleTokenAccount {
                account_pubkey: token_account_pubkey,
                mint_pubkey: context.mint_pubkey,
                owner_pubkey: context.owner_keypair.pubkey(),
                compressible_config: context.compressible_config,
                rent_sponsor: compressible_data.rent_sponsor,
                pre_pay_num_epochs: compressible_data.num_prepaid_epochs,
                lamports_per_write: compressible_data.lamports_per_write,
                payer: payer_pubkey,
                compress_to_account_pubkey: None,
                token_account_version: compressible_data.account_version,
            },
        )
        .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(
            &[create_token_account_ix],
            &payer_pubkey,
            &[&context.payer, &context.token_account_keypair],
        )
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}
