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
use light_registry::compressible::compressed_token::CompressAndCloseIndices;
pub use light_test_utils::{
    assert_close_token_account::assert_close_token_account,
    assert_create_token_account::{
        assert_create_associated_token_account, assert_create_token_account, CompressibleData,
    },
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

/// Set up test environment with an already-created token account
/// If num_prepaid_epochs is Some, creates a compressible account with that many epochs
/// If num_prepaid_epochs is None, creates a non-compressible account
/// If use_payer_as_rent_sponsor is true, uses context.payer.pubkey() as rent_sponsor
pub async fn setup_account_test_with_created_account(
    num_prepaid_epochs: Option<(u8, bool)>,
) -> Result<AccountTestContext, RpcError> {
    let mut context = setup_account_test().await?;

    if let Some((epochs, use_payer_as_rent_sponsor)) = num_prepaid_epochs {
        // Create compressible token account with specified epochs
        let rent_sponsor = if use_payer_as_rent_sponsor {
            context.payer.pubkey()
        } else {
            context.rent_sponsor
        };

        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: epochs,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: context.payer.pubkey(),
        };
        create_and_assert_token_account(&mut context, compressible_data, "setup_account").await;
    } else {
        // Create non-compressible token account (165 bytes, no extension)
        create_non_compressible_token_account(&mut context, None).await;
    }

    Ok(context)
}

/// Create a non-compressible token account (165 bytes, no compressible extension)
pub async fn create_non_compressible_token_account(
    context: &mut AccountTestContext,
    token_keypair: Option<&Keypair>,
) {
    use anchor_lang::prelude::{borsh::BorshSerialize, AccountMeta};
    use light_ctoken_types::instructions::create_ctoken_account::CreateTokenAccountInstructionData;
    use solana_sdk::instruction::Instruction;
    let token_keypair = token_keypair.unwrap_or(&context.token_account_keypair);
    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = token_keypair.pubkey();

    // Create account via system program (165 bytes for non-compressible)
    let rent = context
        .rpc
        .get_minimum_balance_for_rent_exemption(165)
        .await
        .unwrap();

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rent,
        165,
        &light_compressed_token::ID,
    );

    context
        .rpc
        .create_and_send_transaction(
            &[create_account_ix],
            &payer_pubkey,
            &[&context.payer, token_keypair],
        )
        .await
        .unwrap();

    // Initialize the token account (non-compressible)
    let init_data = CreateTokenAccountInstructionData {
        owner: context.owner_keypair.pubkey().into(),
        compressible_config: None, // Non-compressible
    };
    let mut data = vec![18]; // CreateTokenAccount discriminator
    init_data.serialize(&mut data).unwrap();

    let init_ix = Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(token_account_pubkey, true),
            AccountMeta::new_readonly(context.mint_pubkey, false),
        ],
        data,
    };

    context
        .rpc
        .create_and_send_transaction(&[init_ix], &payer_pubkey, &[&context.payer, token_keypair])
        .await
        .unwrap();

    // Assert account was created correctly
    assert_create_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.mint_pubkey,
        context.owner_keypair.pubkey(),
        None, // Non-compressible
    )
    .await;
}

/// Close token account and assert success
pub async fn close_and_assert_token_account(
    context: &mut AccountTestContext,
    destination: Pubkey,
    name: &str,
) {
    println!("Account closure initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Get account info to determine if it has compressible extension
    let account_info = context
        .rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    let is_compressible = account_info.data.len() == COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;

    let close_ix = if is_compressible {
        // Read rent_sponsor from the account's compressible extension
        use light_ctoken_types::state::{CToken, ZExtensionStruct};
        use light_zero_copy::traits::ZeroCopyAt;

        let (ctoken, _) = CToken::zero_copy_at(&account_info.data).unwrap();
        let rent_sponsor = if let Some(extensions) = ctoken.extensions.as_ref() {
            extensions
                .iter()
                .find_map(|ext| match ext {
                    ZExtensionStruct::Compressible(comp) => Some(Pubkey::from(comp.rent_sponsor)),
                    _ => None,
                })
                .unwrap()
        } else {
            panic!("Compressible account must have compressible extension");
        };

        close_compressible_account(
            &light_compressed_token::ID,
            &token_account_pubkey,
            &destination,
            &context.owner_keypair.pubkey(),
            &rent_sponsor,
        )
    } else {
        close_account(
            &light_compressed_token::ID,
            &token_account_pubkey,
            &destination,
            &context.owner_keypair.pubkey(),
        )
    };

    context
        .rpc
        .create_and_send_transaction(
            &[close_ix],
            &payer_pubkey,
            &[&context.payer, &context.owner_keypair],
        )
        .await
        .unwrap();

    // Assert account was closed (should not exist or have 0 data length)
    assert_close_token_account(
        &mut context.rpc,
        token_account_pubkey,
        context.owner_keypair.pubkey(),
        destination,
    )
    .await;
}

/// Close token account expecting failure with specific error code
pub async fn close_and_assert_token_account_fails(
    context: &mut AccountTestContext,
    destination: Pubkey,
    authority: &Keypair,
    rent_sponsor: Option<Pubkey>,
    name: &str,
    expected_error_code: u32,
) {
    println!(
        "Account closure (expecting failure) initiated for: {}",
        name
    );

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    let close_ix = if let Some(sponsor) = rent_sponsor {
        close_compressible_account(
            &light_compressed_token::ID,
            &token_account_pubkey,
            &destination,
            &authority.pubkey(),
            &sponsor,
        )
    } else {
        close_account(
            &light_compressed_token::ID,
            &token_account_pubkey,
            &destination,
            &authority.pubkey(),
        )
    };

    let result = context
        .rpc
        .create_and_send_transaction(&[close_ix], &payer_pubkey, &[&context.payer, authority])
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}

/// Create associated token account and assert success
/// Returns the ATA pubkey
pub async fn create_and_assert_ata(
    context: &mut AccountTestContext,
    compressible_data: Option<CompressibleData>,
    idempotent: bool,
    name: &str,
) -> Pubkey {
    println!("ATA creation initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Derive ATA address
    let (ata_pubkey, _bump) = derive_ctoken_ata(&owner_pubkey, &context.mint_pubkey);

    // Build instruction based on whether it's compressible
    let create_ata_ix = if let Some(compressible) = compressible_data.as_ref() {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account
        };

        create_fn(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                owner: owner_pubkey,
                mint: context.mint_pubkey,
                compressible_config: context.compressible_config,
                rent_sponsor: compressible.rent_sponsor,
                pre_pay_num_epochs: compressible.num_prepaid_epochs,
                lamports_per_write: compressible.lamports_per_write,
                token_account_version: compressible.account_version,
            },
        )
        .unwrap()
    } else {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_associated_token_account_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_associated_token_account
        };

        create_fn(payer_pubkey, owner_pubkey, context.mint_pubkey).unwrap()
    };

    context
        .rpc
        .create_and_send_transaction(&[create_ata_ix], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    // Assert ATA was created correctly with address derivation check
    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        compressible_data,
    )
    .await;

    ata_pubkey
}

/// Create associated token account expecting failure with specific error code
pub async fn create_and_assert_ata_fails(
    context: &mut AccountTestContext,
    compressible_data: Option<CompressibleData>,
    idempotent: bool,
    name: &str,
    expected_error_code: u32,
) {
    println!("ATA creation (expecting failure) initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Build instruction based on whether it's compressible
    let create_ata_ix = if let Some(compressible) = compressible_data.as_ref() {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account
        };

        create_fn(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                owner: owner_pubkey,
                mint: context.mint_pubkey,
                compressible_config: context.compressible_config,
                rent_sponsor: compressible.rent_sponsor,
                pre_pay_num_epochs: compressible.num_prepaid_epochs,
                lamports_per_write: compressible.lamports_per_write,
                token_account_version: compressible.account_version,
            },
        )
        .unwrap()
    } else {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_associated_token_account_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_associated_token_account
        };

        create_fn(payer_pubkey, owner_pubkey, context.mint_pubkey).unwrap()
    };

    let result = context
        .rpc
        .create_and_send_transaction(&[create_ata_ix], &payer_pubkey, &[&context.payer])
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}

// ============================================================================
// Compress and Close Helper Functions
// ============================================================================

/// Setup context with account ready to compress and close
///
/// # Parameters
/// - `num_prepaid_epochs`: Number of epochs to prepay for rent (0 = immediately compressible)
/// - `with_balance`: Token balance to set on the account (0 = no balance)
/// - `warp_epochs`: Optional number of epochs to advance time (makes account compressible for rent authority)
/// - `use_custom_payer`: If true, uses context.payer as rent_sponsor (for custom payer tests)
///
/// # Returns
/// AccountTestContext with created token account ready for compress_and_close
pub async fn setup_compress_and_close_test(
    num_prepaid_epochs: u8,
    with_balance: u64,
    warp_epochs: Option<u64>,
    use_custom_payer: bool,
) -> Result<AccountTestContext, RpcError> {
    use anchor_spl::token_2022::spl_token_2022;
    use solana_sdk::program_pack::Pack;

    let mut context =
        setup_account_test_with_created_account(Some((num_prepaid_epochs, use_custom_payer)))
            .await?;

    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Set balance if needed
    if with_balance > 0 {
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await?
            .ok_or_else(|| RpcError::AssertRpcError("Token account not found".to_string()))?;

        // Deserialize and modify the token account (only use first 165 bytes for SPL compatibility)
        let mut spl_token_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).map_err(
                |e| RpcError::AssertRpcError(format!("Failed to unpack token account: {:?}", e)),
            )?;

        spl_token_account.amount = with_balance;

        spl_token_2022::state::Account::pack(spl_token_account, &mut token_account.data[..165])
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to pack token account: {:?}", e))
            })?;

        // Set the modified account
        context.rpc.set_account(token_account_pubkey, token_account);
    }

    // Warp time if needed (to make account compressible for rent authority)
    if let Some(epochs) = warp_epochs {
        context
            .rpc
            .warp_to_slot((SLOTS_PER_EPOCH * epochs) + 1)
            .unwrap();
    }

    Ok(context)
}

/// Compress and close account as owner and assert success
///
/// # Parameters
/// - `context`: Test context with RPC and account info
/// - `destination`: Optional destination for user funds (defaults to owner)
/// - `name`: Test name for debugging
pub async fn compress_and_close_owner_and_assert(
    context: &mut AccountTestContext,
    destination: Option<Pubkey>,
    name: &str,
) {
    use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
    use light_test_utils::assert_transfer2::assert_transfer2_compress_and_close;
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, CompressAndCloseInput, Transfer2InstructionType,
    };

    println!("Compress and close (owner) initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    // Check if account is compressible by checking size
    let account_info = context
        .rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();
    let is_compressible = account_info.data.len() == COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;

    // Get output queue for compression
    let output_queue = context
        .rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Create compress_and_close instruction as owner
    let compress_and_close_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::CompressAndClose(
            CompressAndCloseInput {
                solana_ctoken_account: token_account_pubkey,
                authority: owner_pubkey,
                output_queue,
                destination,
                is_compressible,
            },
        )],
        payer_pubkey,
        false,
    )
    .await
    .unwrap();

    // Execute transaction
    context
        .rpc
        .create_and_send_transaction(
            &[compress_and_close_ix],
            &payer_pubkey,
            &[&context.payer, &context.owner_keypair],
        )
        .await
        .unwrap();

    // Assert compress and close succeeded
    assert_transfer2_compress_and_close(
        &mut context.rpc,
        CompressAndCloseInput {
            solana_ctoken_account: token_account_pubkey,
            authority: owner_pubkey,
            output_queue,
            destination,
            is_compressible,
        },
    )
    .await;
}

/// Compress and close account expecting failure with custom authority
///
/// # Parameters
/// - `context`: Test context with RPC and account info
/// - `authority`: Authority keypair to use for the operation (can be owner, wrong authority, etc.)
/// - `destination`: Optional destination for user funds
/// - `name`: Test name for debugging
/// - `expected_error_code`: Expected error code
pub async fn compress_and_close_and_assert_fails(
    context: &mut AccountTestContext,
    authority: &Keypair,
    destination: Option<Pubkey>,
    name: &str,
    expected_error_code: u32,
) {
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, CompressAndCloseInput, Transfer2InstructionType,
    };

    println!(
        "Compress and close (expecting failure) initiated for: {}",
        name
    );

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Get output queue for compression
    let output_queue = context
        .rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Create compress_and_close instruction with specified authority
    let compress_and_close_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::CompressAndClose(
            CompressAndCloseInput {
                solana_ctoken_account: token_account_pubkey,
                authority: authority.pubkey(),
                output_queue,
                destination,
                is_compressible: true,
            },
        )],
        payer_pubkey,
        false,
    )
    .await
    .unwrap();

    // Execute transaction expecting failure with the authority as signer
    let result = context
        .rpc
        .create_and_send_transaction(
            &[compress_and_close_ix],
            &payer_pubkey,
            &[&context.payer, authority],
        )
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}

/// Enum specifying which validation should fail in compress_and_close
#[derive(Debug, Clone, Copy)]
pub enum CompressAndCloseValidationError {
    /// Owner mismatch when compress_to_pubkey=false
    OwnerMismatch(Pubkey),
    /// Owner != account pubkey when compress_to_pubkey=true
    OwnerNotAccountPubkey(Pubkey),
}

/// Compress and close account with intentionally invalid output validation data
///
/// This helper manually builds a registry compress_and_close instruction with custom (potentially wrong) values
/// to test the output validation logic in compress_and_close.
///
/// # Parameters
/// - `context`: Test context with RPC and account info
/// - `validation_error`: Specifies which validation should fail and the incorrect value
/// - `destination`: Optional destination for user funds
/// - `expected_error_code`: Expected error code
pub async fn compress_and_close_forester_with_invalid_output(
    context: &mut AccountTestContext,
    validation_error: CompressAndCloseValidationError,
    destination: Option<Pubkey>,
    expected_error_code: u32,
) {
    use std::str::FromStr;

    use anchor_lang::{InstructionData, ToAccountMetas};
    use light_compressible::config::CompressibleConfig;
    use light_ctoken_types::state::{CToken, ZExtensionStruct};
    use light_registry::{
        accounts::CompressAndCloseContext as CompressAndCloseAccounts,
        instruction::CompressAndClose, utils::get_forester_epoch_pda_from_authority,
    };
    use light_sdk::instruction::PackedAccounts;
    use light_zero_copy::traits::ZeroCopyAt;
    use solana_sdk::instruction::Instruction;

    println!(
        "Compress and close (forester, invalid output: {:?}) initiated",
        validation_error
    );

    let payer_pubkey = context.payer.pubkey();
    let token_account_pubkey = context.token_account_keypair.pubkey();

    // Get forester keypair and setup registry accounts
    let forester_keypair = context.rpc.test_accounts.protocol.forester.insecure_clone();
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressed_token_program_id =
        Pubkey::from_str("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m").unwrap();
    let current_epoch = 0;
    let (registered_forester_pda, _) =
        get_forester_epoch_pda_from_authority(&forester_keypair.pubkey(), current_epoch);
    let config = CompressibleConfig::ctoken_v1(Pubkey::default(), Pubkey::default());
    let compressible_config = CompressibleConfig::derive_v1_config_pda(&registry_program_id).0;
    let compression_authority = config.compression_authority;

    // Read token account to get current state
    let token_account_info = context
        .rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    let (ctoken, _) = CToken::zero_copy_at(&token_account_info.data).unwrap();
    let mint_pubkey = Pubkey::from(ctoken.mint.to_bytes());

    // Extract compressible extension data
    let extensions = ctoken.extensions.as_ref().unwrap();
    let compressible_ext = extensions
        .iter()
        .find_map(|ext| match ext {
            ZExtensionStruct::Compressible(comp) => Some(comp),
            _ => None,
        })
        .unwrap();

    let rent_sponsor = Pubkey::from(compressible_ext.rent_sponsor);

    // Get output queue for compression
    let output_queue = context
        .rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // Build PackedAccounts
    let mut packed_accounts = PackedAccounts::default();

    packed_accounts.insert_or_get(output_queue);
    let source_index = packed_accounts.insert_or_get(token_account_pubkey);
    let mint_index = packed_accounts.insert_or_get(mint_pubkey);

    // Determine owner based on validation_error
    let compressed_token_owner = match validation_error {
        CompressAndCloseValidationError::OwnerMismatch(wrong_owner) => wrong_owner,
        CompressAndCloseValidationError::OwnerNotAccountPubkey(wrong_owner) => wrong_owner,
    };

    let owner_index = packed_accounts.insert_or_get(compressed_token_owner);
    let rent_sponsor_index = packed_accounts.insert_or_get(rent_sponsor);
    let authority_index = packed_accounts.insert_or_get_config(compression_authority, false, true);
    let destination_pubkey = destination.unwrap_or(payer_pubkey);
    let destination_index = packed_accounts.insert_or_get_config(destination_pubkey, false, true);

    let indices = CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        rent_sponsor_index,
    };

    // Add system accounts
    use light_compressed_token_sdk::instructions::compress_and_close::CompressAndCloseAccounts as CTokenCompressAndCloseAccounts;
    let config = CTokenCompressAndCloseAccounts {
        compressed_token_program: compressed_token_program_id,
        cpi_authority_pda: Pubkey::find_program_address(
            &[b"cpi_authority"],
            &compressed_token_program_id,
        )
        .0,
        cpi_context: None,
        self_program: None,
    };
    packed_accounts.add_custom_system_accounts(config).unwrap();

    let (remaining_account_metas, _, _) = packed_accounts.to_account_metas();

    // Build registry accounts
    let compress_and_close_accounts = CompressAndCloseAccounts {
        authority: forester_keypair.pubkey(),
        registered_forester_pda,
        compression_authority,
        compressible_config,
    };

    let mut accounts = compress_and_close_accounts.to_account_metas(Some(true));
    accounts.extend(remaining_account_metas);

    let instruction = CompressAndClose {
        authority_index,
        destination_index,
        indices: vec![indices],
    };
    let instruction_data = instruction.data();

    let compress_and_close_ix = Instruction {
        program_id: registry_program_id,
        accounts,
        data: instruction_data,
    };

    // Execute transaction expecting failure
    let result = context
        .rpc
        .create_and_send_transaction(
            &[compress_and_close_ix],
            &payer_pubkey,
            &[&context.payer, &forester_keypair],
        )
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}
