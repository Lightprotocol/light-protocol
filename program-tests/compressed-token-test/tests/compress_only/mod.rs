//! Shared helpers and test context for compress_only extension tests.
//!
//! This module contains utilities for testing the compress_only behavior
//! with Token-2022 mints that have restricted extensions.

use borsh::BorshDeserialize;
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
pub use light_test_utils::{mint_2022::ALL_EXTENSIONS, Rpc};
use light_test_utils::{
    mint_2022::{
        create_mint_22_with_extension_types, create_token_22_account, mint_spl_tokens_22,
        Token22ExtensionConfig, RESTRICTED_EXTENSIONS,
    },
    RpcError,
};
use light_token_interface::state::{AccountState, ExtensionStruct, Token};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
pub use spl_token_2022::extension::ExtensionType;

/// Test context for extension-related tests
pub struct ExtensionsTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub _mint_keypair: Keypair,
    pub mint_pubkey: Pubkey,
    pub extension_config: Token22ExtensionConfig,
}

/// Set up test environment with a Token 2022 mint with specified extensions
pub async fn setup_extensions_test(
    extensions: &[ExtensionType],
) -> Result<ExtensionsTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with specified extensions
    let (mint_keypair, extension_config) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;

    let mint_pubkey = mint_keypair.pubkey();

    Ok(ExtensionsTestContext {
        rpc,
        payer,
        _mint_keypair: mint_keypair,
        mint_pubkey,
        extension_config,
    })
}

/// Configuration for parameterized compress and close extension tests
pub struct CompressAndCloseTestConfig {
    /// Extensions to initialize on the mint
    pub extensions: &'static [ExtensionType],
    /// Delegate keypair and delegated_amount (delegate can sign)
    pub delegate_config: Option<(Keypair, u64)>,
    /// Set account state to frozen before compress
    pub is_frozen: bool,
    /// Use permanent delegate as authority for decompress (instead of owner)
    pub use_permanent_delegate_for_decompress: bool,
    /// Use regular delegate as authority for decompress (instead of owner)
    pub use_delegate_for_decompress: bool,
}

/// Helper to modify Light Token account state for testing using set_account
/// Only modifies the SPL token portion (first 165 bytes) - Light Token::deserialize reads from there
pub async fn set_ctoken_account_state(
    rpc: &mut LightProgramTest,
    account_pubkey: Pubkey,
    delegate: Option<Pubkey>,
    delegated_amount: u64,
    is_frozen: bool,
) -> Result<(), RpcError> {
    use anchor_spl::token_2022::spl_token_2022;
    use solana_sdk::{program_option::COption, program_pack::Pack};

    let mut account_info = rpc
        .get_account(account_pubkey)
        .await?
        .ok_or_else(|| RpcError::CustomError("Account not found".to_string()))?;

    // Update SPL token state (first 165 bytes)
    // Light Token::deserialize reads delegate/delegated_amount/state from the SPL portion
    let mut spl_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_info.data[..165])
            .map_err(|e| RpcError::CustomError(format!("Failed to unpack SPL account: {:?}", e)))?;

    spl_account.delegate = match delegate {
        Some(d) => COption::Some(d),
        None => COption::None,
    };
    spl_account.delegated_amount = delegated_amount;
    if is_frozen {
        spl_account.state = spl_token_2022::state::AccountState::Frozen;
    }

    spl_token_2022::state::Account::pack(spl_account, &mut account_info.data[..165])
        .map_err(|e| RpcError::CustomError(format!("Failed to pack SPL account: {:?}", e)))?;

    rpc.set_account(account_pubkey, account_info);
    Ok(())
}

/// Helper to set withheld_amount in TransferFeeAccount extension for testing
/// Finds the TransferFeeAccount extension in the Light Token and modifies the withheld_amount field
pub async fn set_ctoken_withheld_fee(
    rpc: &mut LightProgramTest,
    account_pubkey: Pubkey,
    withheld_amount: u64,
) -> Result<(), RpcError> {
    use light_token_interface::state::{ExtensionStruct, TransferFeeAccountExtension};

    let mut account_info = rpc
        .get_account(account_pubkey)
        .await?
        .ok_or_else(|| RpcError::CustomError("Account not found".to_string()))?;

    // Deserialize Token to find and modify TransferFeeAccount extension
    let mut ctoken = Token::deserialize(&mut &account_info.data[..])
        .map_err(|e| RpcError::CustomError(format!("Failed to deserialize Token: {:?}", e)))?;

    // Find and update TransferFeeAccount extension
    let mut found = false;
    if let Some(extensions) = ctoken.extensions.as_mut() {
        for ext in extensions.iter_mut() {
            if let ExtensionStruct::TransferFeeAccount(fee_ext) = ext {
                *fee_ext = TransferFeeAccountExtension { withheld_amount };
                found = true;
                break;
            }
        }
    }

    if !found {
        return Err(RpcError::CustomError(
            "TransferFeeAccount extension not found in Token".to_string(),
        ));
    }

    // Serialize the modified Token back
    use borsh::BorshSerialize;
    let serialized = ctoken
        .try_to_vec()
        .map_err(|e| RpcError::CustomError(format!("Failed to serialize Token: {:?}", e)))?;

    // Update account data
    account_info.data = serialized;
    rpc.set_account(account_pubkey, account_info);
    Ok(())
}

/// Core parameterized test function for compress -> decompress cycle with configurable state
pub async fn run_compress_and_close_extension_test(
    config: CompressAndCloseTestConfig,
) -> Result<(), RpcError> {
    use light_client::indexer::Indexer;
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    };
    use light_token_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::{
            CompressedOnlyExtension, CompressedTokenAccountState, TokenData, TokenDataVersion,
        },
    };
    use light_token_sdk::{
        spl_interface::find_spl_interface_pda_with_index,
        token::{CompressibleParams, CreateTokenAccount, TransferFromSpl},
    };

    let mut context = setup_extensions_test(config.extensions).await?;
    let has_restricted_extensions = config
        .extensions
        .iter()
        .any(|ext| RESTRICTED_EXTENSIONS.contains(ext));
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let _permanent_delegate = context.extension_config.permanent_delegate;

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

    // 2. Create Light Token account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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
                compression_only: has_restricted_extensions,
            })
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 3. Transfer tokens to Light Token using hot path
    // Determine if mint has restricted extensions for pool derivation

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, has_restricted_extensions);
    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ctoken_account,
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

    // 4. Modify Light Token state based on config BEFORE warp
    let delegate_pubkey = config.delegate_config.as_ref().map(|(kp, _)| kp.pubkey());
    let delegated_amount = config
        .delegate_config
        .as_ref()
        .map(|(_, a)| *a)
        .unwrap_or(0);

    if config.delegate_config.is_some() || config.is_frozen {
        set_ctoken_account_state(
            &mut context.rpc,
            ctoken_account,
            delegate_pubkey,
            delegated_amount,
            config.is_frozen,
        )
        .await?;
    }

    // 5. Warp epoch to trigger forester compression
    context.rpc.warp_epoch_forward(30).await?;

    // 6. Assert the account has been compressed (closed)
    let account_after = context.rpc.get_account(ctoken_account).await?;
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "Light Token account should be closed after compression"
    );

    // 7. Get compressed accounts and verify state
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Build expected TokenData based on config
    let expected_state = if config.is_frozen {
        CompressedTokenAccountState::Frozen as u8
    } else {
        CompressedTokenAccountState::Initialized as u8
    };

    let expected_token_data = TokenData {
        mint: mint_pubkey.into(),
        owner: owner.pubkey().into(),
        amount: mint_amount,
        delegate: delegate_pubkey.map(|d| d.into()),
        state: expected_state,
        tlv: Some(vec![ExtensionStruct::CompressedOnly(
            CompressedOnlyExtension {
                delegated_amount,
                withheld_transfer_fee: 0,
                is_ata: 0, // Non-ATA regular account
            },
        )]),
    };

    assert_eq!(
        compressed_accounts[0].token,
        expected_token_data.into(),
        "Compressed token account should match expected TokenData"
    );

    // 8. Create destination Light Token account for decompress
    let decompress_dest_keypair = Keypair::new();
    let decompress_dest_account = decompress_dest_keypair.pubkey();

    let create_dest_ix = CreateTokenAccount::new(
        payer.pubkey(),
        decompress_dest_account,
        mint_pubkey,
        owner.pubkey(),
    )
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
        compression_only: has_restricted_extensions,
    })
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create dest instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &payer.pubkey(),
            &[&payer, &decompress_dest_keypair],
        )
        .await?;

    // 9. Decompress with correct in_tlv including is_frozen
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount,
            withheld_transfer_fee: 0,
            is_frozen: config.is_frozen,
            compression_index: 0,
            is_ata: false, // Non-ATA regular account
            bump: 0,
            owner_index: 0,
        },
    )]];

    let mut decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: decompress_dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create decompress instruction: {:?}", e))
    })?;

    // 10. Sign with owner, permanent delegate, or regular delegate based on config
    let signers: Vec<&Keypair> = if config.use_permanent_delegate_for_decompress {
        // Permanent delegate is the payer in this test setup.
        // Find owner in account metas and set is_signer = false since permanent delegate acts on behalf.
        let owner_pubkey = owner.pubkey();
        for account_meta in decompress_ix.accounts.iter_mut() {
            if account_meta.pubkey == owner_pubkey {
                account_meta.is_signer = false;
            }
        }
        vec![&payer]
    } else if config.use_delegate_for_decompress {
        // Regular delegate signs instead of owner
        let delegate_kp = &config
            .delegate_config
            .as_ref()
            .expect("delegate_config required when use_delegate_for_decompress is true")
            .0;
        let delegate_pubkey = delegate_kp.pubkey();

        // Add delegate as signer account (it's not in the instruction by default)
        decompress_ix
            .accounts
            .push(solana_sdk::instruction::AccountMeta {
                pubkey: delegate_pubkey,
                is_signer: true,
                is_writable: false,
            });

        // Remove owner as signer
        let owner_pubkey = owner.pubkey();
        for account_meta in decompress_ix.accounts.iter_mut() {
            if account_meta.pubkey == owner_pubkey {
                account_meta.is_signer = false;
            }
        }
        vec![&payer, delegate_kp]
    } else {
        vec![&payer, &owner]
    };

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &signers)
        .await?;

    // 11. Verify decompressed Light Token state
    let dest_account_data = context
        .rpc
        .get_account(decompress_dest_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("Dest account not found".to_string()))?;

    let dest_ctoken = Token::deserialize(&mut &dest_account_data.data[..])
        .map_err(|e| RpcError::CustomError(format!("Failed to deserialize Token: {:?}", e)))?;

    // Verify state matches config
    let expected_ctoken_state = if config.is_frozen {
        AccountState::Frozen
    } else {
        AccountState::Initialized
    };

    assert_eq!(
        dest_ctoken.state, expected_ctoken_state,
        "Decompressed Light Token state should match config"
    );

    assert_eq!(
        dest_ctoken.delegated_amount, delegated_amount,
        "Decompressed Light Token delegated_amount should match"
    );

    if let Some((delegate_kp, _)) = &config.delegate_config {
        assert_eq!(
            dest_ctoken.delegate,
            Some(delegate_kp.pubkey().to_bytes().into()),
            "Decompressed Light Token delegate should match"
        );
    } else {
        assert!(
            dest_ctoken.delegate.is_none(),
            "Decompressed Light Token should have no delegate"
        );
    }

    // 12. Verify no more compressed accounts
    let remaining_compressed = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    assert_eq!(
        remaining_compressed.len(),
        0,
        "Should have no more compressed token accounts after decompress"
    );

    println!("Successfully completed compress-and-close -> decompress cycle");

    Ok(())
}
