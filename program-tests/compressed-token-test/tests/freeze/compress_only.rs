//! Tests for freezing/thawing compressed-only token accounts while compressed.
//!
//! Verifies that compressed-only token accounts (with restricted T22 extensions)
//! can be frozen and thawed while in compressed state using the anchor freeze instruction.

use light_client::indexer::{CompressedTokenAccount, Indexer};
use light_compressed_token::freeze::sdk::{
    create_instruction, CreateInstructionInputs as FreezeInputs,
};
use light_ctoken_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::TokenDataVersion,
};
use light_ctoken_sdk::{
    compat::{AccountState, TokenDataWithMerkleContext},
    ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
    spl_interface::find_spl_interface_pda_with_index,
};
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    conversions::sdk_to_program_token_data,
    mint_2022::{
        create_mint_22_with_extension_types, create_token_22_account, mint_spl_tokens_22,
        Token22ExtensionConfig, RESTRICTED_EXTENSIONS,
    },
    Rpc, RpcError,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

/// Restricted extensions for testing
/// Note: DefaultAccountState is required to set freeze_authority on the mint
const TEST_EXTENSIONS: &[ExtensionType] = &[
    ExtensionType::PermanentDelegate,
    ExtensionType::TransferFeeConfig,
    ExtensionType::TransferHook,
    ExtensionType::Pausable,
    ExtensionType::DefaultAccountState,
];

/// Test context for freeze tests
struct FreezeTestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    mint_pubkey: Pubkey,
    _extension_config: Token22ExtensionConfig,
}

/// Set up test environment with a Token 2022 mint with restricted extensions
async fn setup_freeze_test(extensions: &[ExtensionType]) -> Result<FreezeTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with specified extensions
    let (mint_keypair, extension_config) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;

    let mint_pubkey = mint_keypair.pubkey();

    Ok(FreezeTestContext {
        rpc,
        payer,
        mint_pubkey,
        _extension_config: extension_config,
    })
}

/// Helper to append version byte to the inner inputs Vec of an Anchor instruction.
/// Anchor instruction format: [8 bytes discriminator][4 bytes Vec length][N bytes Vec content]
fn append_version_to_inputs(instruction: &mut solana_sdk::instruction::Instruction, version: u8) {
    // The Vec length is at bytes 8..12 (little endian u32)
    let len_bytes = &instruction.data[8..12];
    let current_len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]);

    // Increment the length
    let new_len = current_len + 1;
    instruction.data[8..12].copy_from_slice(&new_len.to_le_bytes());

    // Append the version byte to the data
    instruction.data.push(version);
}

/// Helper to create and send freeze or thaw instruction
async fn freeze_or_thaw_compressed<const FREEZE: bool>(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    compressed_accounts: Vec<TokenDataWithMerkleContext>,
    output_merkle_tree: &Pubkey,
) -> Result<(), RpcError> {
    // Get validity proofs for the compressed accounts
    let input_compressed_account_hashes = compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();

    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await?;

    let inputs = FreezeInputs {
        fee_payer: payer.pubkey(),
        authority: payer.pubkey(),
        input_merkle_contexts: compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: compressed_accounts
            .iter()
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: compressed_accounts
            .iter()
            .map(|x| x.compressed_account.compressed_account.clone())
            .collect::<Vec<_>>(),
        outputs_merkle_tree: *output_merkle_tree,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof: proof_rpc_result.value.proof.0.unwrap_or_default(),
    };

    let mut instruction = create_instruction::<FREEZE>(inputs).map_err(|e| {
        RpcError::CustomError(format!("Failed to create freeze instruction: {:?}", e))
    })?;

    // Append version byte (ShaFlat = 3) to the inner inputs Vec
    append_version_to_inputs(&mut instruction, TokenDataVersion::ShaFlat as u8);

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(())
}

/// Test that compressed-only token accounts can be frozen and thawed while compressed.
///
/// Flow:
/// 1. Create mint with restricted extensions
/// 2. Create compress-only CToken account
/// 3. Transfer tokens to it
/// 4. Warp epoch to compress (compress and close)
/// 5. Freeze the compressed token account
/// 6. Verify frozen state
/// 7. Thaw the compressed token account
/// 8. Verify thawed state
#[tokio::test]
#[serial]
async fn test_freeze_thaw_compressed_only_account() {
    let result = run_freeze_thaw_compressed_only_test(TEST_EXTENSIONS).await;
    assert!(result.is_ok(), "Test failed: {:?}", result.err());
}

async fn run_freeze_thaw_compressed_only_test(
    extensions: &[ExtensionType],
) -> Result<(), RpcError> {
    let mut context = setup_freeze_test(extensions).await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // 1. Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // 2. Create CToken account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 3. Transfer tokens to CToken using hot path
    let has_restricted = extensions
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted);
    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create transfer instruction: {:?}", e))
    })?;

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 4. Warp epoch to trigger forester compression
    context.rpc.warp_epoch_forward(30).await?;

    // 5. Assert the account has been compressed (closed)
    let account_after = context.rpc.get_account(ctoken_account).await?;
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "CToken account should be closed after compression"
    );

    // 6. Get compressed accounts and verify state
    let compressed_accounts: Vec<TokenDataWithMerkleContext> = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Verify initial state is Initialized
    assert_eq!(
        compressed_accounts[0].token_data.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );

    let output_merkle_tree: Pubkey = compressed_accounts[0]
        .compressed_account
        .merkle_context
        .queue_pubkey
        .into();

    // 7. Freeze the compressed token account
    freeze_or_thaw_compressed::<true>(
        &mut context.rpc,
        &payer,
        compressed_accounts.clone(),
        &output_merkle_tree,
    )
    .await?;

    // 8. Get updated compressed accounts and verify frozen state
    let frozen_accounts: Vec<TokenDataWithMerkleContext> = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        frozen_accounts.len(),
        1,
        "Should still have exactly 1 compressed token account"
    );
    assert_eq!(
        frozen_accounts[0].token_data.state,
        AccountState::Frozen,
        "Token account should be frozen"
    );

    // 9. Thaw the compressed token account
    freeze_or_thaw_compressed::<false>(
        &mut context.rpc,
        &payer,
        frozen_accounts.clone(),
        &output_merkle_tree,
    )
    .await?;

    // 10. Verify thawed state
    let thawed_accounts: Vec<TokenDataWithMerkleContext> = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        thawed_accounts.len(),
        1,
        "Should still have exactly 1 compressed token account"
    );
    assert_eq!(
        thawed_accounts[0].token_data.state,
        AccountState::Initialized,
        "Token account should be thawed (Initialized)"
    );

    // 11. Create destination CToken account for decompress
    let dest_account_keypair = Keypair::new();
    let create_dest_ix =
        CreateCTokenAccount::new(payer.pubkey(), dest_account_keypair.pubkey(), mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_account_keypair])
        .await?;

    // 12. Build TLV data for decompress (CompressedOnly extension with is_ata=false)
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    // 13. Decompress to CToken account
    let compressed_account: CompressedTokenAccount = thawed_accounts[0].clone().try_into().unwrap();
    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_account],
            decompress_amount: mint_amount,
            solana_token_account: dest_account_keypair.pubkey(),
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .map_err(|e| RpcError::CustomError(format!("Failed to create decompress instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 14. Verify CToken account has tokens
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::CToken;
    let dest_account = context
        .rpc
        .get_account(dest_account_keypair.pubkey())
        .await?
        .unwrap();
    let dest_ctoken = CToken::deserialize(&mut &dest_account.data[..]).unwrap();
    assert_eq!(
        dest_ctoken.amount, mint_amount,
        "Decompressed amount should match"
    );

    println!("Successfully froze, thawed, and decompressed compressed-only token account");

    Ok(())
}
