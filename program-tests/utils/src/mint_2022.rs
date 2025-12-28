//! Helper functions for creating Token 2022 mints with multiple extensions.
//!
//! This module provides utilities to create Token 2022 mints with various extensions
//! enabled for testing purposes.

use forester_utils::instructions::create_account::create_account_instruction;
use light_client::rpc::Rpc;
use light_ctoken_interface::RESTRICTED_EXTENSION_TYPES;
use light_ctoken_sdk::spl_interface::{find_spl_interface_pda, CreateSplInterfacePda};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token_2022::{
    extension::{
        confidential_transfer::{
            instruction::initialize_mint as initialize_confidential_transfer_mint,
            ConfidentialTransferMint,
        },
        confidential_transfer_fee::{
            instruction::initialize_confidential_transfer_fee_config, ConfidentialTransferFeeConfig,
        },
        default_account_state::{
            instruction::initialize_default_account_state, DefaultAccountState,
        },
        metadata_pointer::{
            instruction::initialize as initialize_metadata_pointer, MetadataPointer,
        },
        mint_close_authority::MintCloseAuthority,
        pausable::{instruction::initialize as initialize_pausable, PausableConfig},
        permanent_delegate::PermanentDelegate,
        transfer_fee::{instruction::initialize_transfer_fee_config, TransferFeeConfig},
        transfer_hook::{instruction::initialize as initialize_transfer_hook, TransferHook},
        BaseStateWithExtensions, ExtensionType, StateWithExtensions, StateWithExtensionsMut,
    },
    instruction::{
        initialize_mint, initialize_mint_close_authority, initialize_permanent_delegate,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    state::{AccountState, Mint},
};

/// Configuration returned after creating a Token 2022 mint with extensions.
/// Contains the mint pubkey and all the authorities for the various extensions.
#[derive(Debug, Clone)]
pub struct Token22ExtensionConfig {
    /// The mint pubkey
    pub mint: Pubkey,
    /// The token pool PDA for compressed tokens
    pub token_pool: Pubkey,
    /// Authority that can close the mint account
    pub close_authority: Pubkey,
    /// Authority that can update transfer fee configuration
    pub transfer_fee_config_authority: Pubkey,
    /// Authority that can withdraw withheld transfer fees
    pub withdraw_withheld_authority: Pubkey,
    /// Permanent delegate that can transfer/burn any tokens
    pub permanent_delegate: Pubkey,
    /// Authority that can update metadata
    pub metadata_update_authority: Pubkey,
    /// Authority that can pause/unpause the mint
    pub pause_authority: Pubkey,
    /// Authority for confidential transfer configuration
    pub confidential_transfer_authority: Pubkey,
    /// Authority for confidential transfer fee withdraw
    pub confidential_transfer_fee_authority: Pubkey,
    /// Whether the mint has DefaultAccountState set to Frozen
    pub default_account_state_frozen: bool,
}

/// All restricted extension types for Token 2022 mints.
/// These extensions restrict token transfers and require compression_only mode.
pub const RESTRICTED_EXTENSIONS: &[ExtensionType] = RESTRICTED_EXTENSION_TYPES.as_slice();

/// Non-restricted extension types for Token 2022 mints.
/// These extensions don't restrict transfers and work with normal compression.
pub const NON_RESTRICTED_EXTENSIONS: &[ExtensionType] = &[
    ExtensionType::MintCloseAuthority,
    ExtensionType::MetadataPointer,
    ExtensionType::ConfidentialTransferMint,
    ExtensionType::ConfidentialTransferFeeConfig,
];

/// All supported extension types (restricted + non-restricted).
pub const ALL_EXTENSIONS: &[ExtensionType] = &[
    // Non-restricted
    ExtensionType::MintCloseAuthority,
    ExtensionType::DefaultAccountState,
    ExtensionType::MetadataPointer,
    ExtensionType::ConfidentialTransferMint,
    ExtensionType::ConfidentialTransferFeeConfig,
    // Restricted
    ExtensionType::TransferFeeConfig,
    ExtensionType::PermanentDelegate,
    ExtensionType::TransferHook,
    ExtensionType::Pausable,
];

/// Creates a Token 2022 mint with all extensions initialized.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `payer` - Transaction fee payer and authority for all extensions
/// * `decimals` - Token decimals
///
/// # Returns
/// A tuple of (mint_keypair, extension_config)
pub async fn create_mint_22_with_extensions<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    decimals: u8,
) -> (Keypair, Token22ExtensionConfig) {
    create_mint_22_with_extension_types(rpc, payer, decimals, ALL_EXTENSIONS).await
}

/// Creates a Token 2022 mint with the specified extension types.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `payer` - Transaction fee payer and authority for all extensions
/// * `decimals` - Token decimals
/// * `extensions` - Slice of extension types to initialize
///
/// # Returns
/// A tuple of (mint_keypair, extension_config)
pub async fn create_mint_22_with_extension_types<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    decimals: u8,
    extensions: &[ExtensionType],
) -> (Keypair, Token22ExtensionConfig) {
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let authority = payer.pubkey();

    // Calculate the account size needed for requested extensions
    let mint_len = ExtensionType::try_calculate_account_len::<Mint>(extensions).unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_len)
        .await
        .unwrap();

    // Create the mint account
    let create_account_ix = create_account_instruction(
        &authority,
        mint_len,
        rent,
        &spl_token_2022::ID,
        Some(&mint_keypair),
    );

    // Build instructions based on requested extensions
    let mut instructions: Vec<Instruction> = vec![create_account_ix];
    let mut config = Token22ExtensionConfig {
        mint: mint_pubkey,
        token_pool: Pubkey::default(),
        close_authority: Pubkey::default(),
        transfer_fee_config_authority: Pubkey::default(),
        withdraw_withheld_authority: Pubkey::default(),
        permanent_delegate: Pubkey::default(),
        metadata_update_authority: Pubkey::default(),
        pause_authority: Pubkey::default(),
        confidential_transfer_authority: Pubkey::default(),
        confidential_transfer_fee_authority: Pubkey::default(),
        default_account_state_frozen: false,
    };

    // Add extension init instructions in correct order
    for ext in extensions {
        match ext {
            ExtensionType::MintCloseAuthority => {
                instructions.push(
                    initialize_mint_close_authority(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(&authority),
                    )
                    .unwrap(),
                );
                config.close_authority = authority;
            }
            ExtensionType::TransferFeeConfig => {
                instructions.push(
                    initialize_transfer_fee_config(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(&authority),
                        Some(&authority),
                        0,
                        0,
                    )
                    .unwrap(),
                );
                config.transfer_fee_config_authority = authority;
                config.withdraw_withheld_authority = authority;
            }
            ExtensionType::DefaultAccountState => {
                instructions.push(
                    initialize_default_account_state(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        &AccountState::Initialized,
                    )
                    .unwrap(),
                );
            }
            ExtensionType::PermanentDelegate => {
                instructions.push(
                    initialize_permanent_delegate(&spl_token_2022::ID, &mint_pubkey, &authority)
                        .unwrap(),
                );
                config.permanent_delegate = authority;
            }
            ExtensionType::TransferHook => {
                instructions.push(
                    initialize_transfer_hook(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(authority),
                        None,
                    )
                    .unwrap(),
                );
            }
            ExtensionType::MetadataPointer => {
                instructions.push(
                    initialize_metadata_pointer(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(authority),
                        Some(mint_pubkey),
                    )
                    .unwrap(),
                );
                config.metadata_update_authority = authority;
            }
            ExtensionType::Pausable => {
                instructions.push(
                    initialize_pausable(&spl_token_2022::ID, &mint_pubkey, &authority).unwrap(),
                );
                config.pause_authority = authority;
            }
            ExtensionType::ConfidentialTransferMint => {
                instructions.push(
                    initialize_confidential_transfer_mint(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(authority),
                        false,
                        None,
                    )
                    .unwrap(),
                );
                config.confidential_transfer_authority = authority;
            }
            ExtensionType::ConfidentialTransferFeeConfig => {
                instructions.push(
                    initialize_confidential_transfer_fee_config(
                        &spl_token_2022::ID,
                        &mint_pubkey,
                        Some(authority),
                        &PodElGamalPubkey::default(),
                    )
                    .unwrap(),
                );
                config.confidential_transfer_fee_authority = authority;
            }
            _ => {} // Ignore unsupported extensions
        }
    }

    // Initialize mint (must come after extension inits)
    // freeze_authority required if DefaultAccountState is present
    let needs_freeze_authority = extensions.contains(&ExtensionType::DefaultAccountState);
    instructions.push(
        initialize_mint(
            &spl_token_2022::ID,
            &mint_pubkey,
            &authority,
            if needs_freeze_authority {
                Some(&authority)
            } else {
                None
            },
            decimals,
        )
        .unwrap(),
    );

    // Create token pool for compressed tokens (restricted=true if any restricted extension)
    let has_restricted = !extensions.is_empty();
    let (token_pool_pubkey, _) = find_spl_interface_pda(&mint_pubkey, has_restricted);
    instructions.push(
        CreateSplInterfacePda::new(authority, mint_pubkey, spl_token_2022::ID, has_restricted)
            .instruction(),
    );
    config.token_pool = token_pool_pubkey;

    // Send transaction
    rpc.create_and_send_transaction(&instructions, &authority, &[payer, &mint_keypair])
        .await
        .unwrap();

    (mint_keypair, config)
}

/// Creates a Token 2022 mint with DefaultAccountState set to Frozen.
/// This creates a minimal mint with only the extensions needed for testing frozen default state.
///
/// Extensions initialized:
/// - DefaultAccountState (Frozen)
/// - PermanentDelegate (required for frozen accounts - allows transfers by delegate)
/// - Pausable (for testing pausable + frozen combination)
///
/// # Arguments
/// * `rpc` - RPC client
/// * `payer` - Transaction fee payer and authority for all extensions
/// * `decimals` - Token decimals
///
/// # Returns
/// A tuple of (mint_keypair, extension_config)
pub async fn create_mint_22_with_frozen_default_state<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    decimals: u8,
) -> (Keypair, Token22ExtensionConfig) {
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let authority = payer.pubkey();

    // Extensions for frozen default state testing
    let extension_types = [
        ExtensionType::DefaultAccountState,
        ExtensionType::PermanentDelegate,
        ExtensionType::Pausable,
    ];

    let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&extension_types).unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_len)
        .await
        .unwrap();

    let create_account_ix = create_account_instruction(
        &authority,
        mint_len,
        rent,
        &spl_token_2022::ID,
        Some(&mint_keypair),
    );

    // 1. Default account state (Frozen)
    let init_default_state_ix =
        initialize_default_account_state(&spl_token_2022::ID, &mint_pubkey, &AccountState::Frozen)
            .unwrap();

    // 2. Permanent delegate (useful for frozen accounts)
    let init_permanent_delegate_ix =
        initialize_permanent_delegate(&spl_token_2022::ID, &mint_pubkey, &authority).unwrap();

    // 3. Pausable
    let init_pausable_ix =
        initialize_pausable(&spl_token_2022::ID, &mint_pubkey, &authority).unwrap();

    // 4. Initialize mint (freeze_authority required for DefaultAccountState)
    let init_mint_ix = initialize_mint(
        &spl_token_2022::ID,
        &mint_pubkey,
        &authority,       // mint_authority
        Some(&authority), // freeze_authority (required for DefaultAccountState)
        decimals,
    )
    .unwrap();

    // 5. Create token pool for compressed tokens (restricted=true for mints with restricted extensions)
    let (token_pool_pubkey, _) = find_spl_interface_pda(&mint_pubkey, true);
    let create_token_pool_ix =
        CreateSplInterfacePda::new(authority, mint_pubkey, spl_token_2022::ID, true).instruction();

    let instructions: Vec<Instruction> = vec![
        create_account_ix,
        init_default_state_ix,
        init_permanent_delegate_ix,
        init_pausable_ix,
        init_mint_ix,
        create_token_pool_ix,
    ];

    rpc.create_and_send_transaction(&instructions, &authority, &[payer, &mint_keypair])
        .await
        .unwrap();

    let config = Token22ExtensionConfig {
        mint: mint_pubkey,
        token_pool: token_pool_pubkey,
        close_authority: Pubkey::default(),
        transfer_fee_config_authority: Pubkey::default(),
        withdraw_withheld_authority: Pubkey::default(),
        permanent_delegate: authority,
        metadata_update_authority: Pubkey::default(),
        pause_authority: authority,
        confidential_transfer_authority: Pubkey::default(),
        confidential_transfer_fee_authority: Pubkey::default(),
        default_account_state_frozen: true,
    };

    (mint_keypair, config)
}

/// Asserts that a Token 2022 mint with all extensions is correctly configured.
///
/// Verifies:
/// - All extensions are present on the mint
/// - Token pool account exists
/// - All authorities match the expected payer
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint_pubkey` - The mint pubkey
/// * `extension_config` - The extension configuration to verify
/// * `expected_authority` - The expected authority for all extensions
pub async fn assert_mint_22_with_all_extensions<R: Rpc>(
    rpc: &mut R,
    mint_pubkey: &Pubkey,
    extension_config: &Token22ExtensionConfig,
    expected_authority: &Pubkey,
) {
    // Verify all extensions are present
    verify_mint_extensions(rpc, mint_pubkey).await.unwrap();

    // Verify the extension config has correct values
    assert_eq!(
        extension_config.mint, *mint_pubkey,
        "Extension config mint should match"
    );

    // Verify token pool was created
    let token_pool_account = rpc.get_account(extension_config.token_pool).await.unwrap();
    assert!(
        token_pool_account.is_some(),
        "Token pool account should exist"
    );

    // Verify all authorities match expected
    assert_eq!(
        extension_config.close_authority, *expected_authority,
        "Close authority mismatch"
    );
    assert_eq!(
        extension_config.transfer_fee_config_authority, *expected_authority,
        "Transfer fee config authority mismatch"
    );
    assert_eq!(
        extension_config.withdraw_withheld_authority, *expected_authority,
        "Withdraw withheld authority mismatch"
    );
    assert_eq!(
        extension_config.permanent_delegate, *expected_authority,
        "Permanent delegate mismatch"
    );
    assert_eq!(
        extension_config.metadata_update_authority, *expected_authority,
        "Metadata update authority mismatch"
    );
    assert_eq!(
        extension_config.pause_authority, *expected_authority,
        "Pause authority mismatch"
    );
    assert_eq!(
        extension_config.confidential_transfer_authority, *expected_authority,
        "Confidential transfer authority mismatch"
    );
    assert_eq!(
        extension_config.confidential_transfer_fee_authority, *expected_authority,
        "Confidential transfer fee authority mismatch"
    );
}

/// Verifies that a mint has all expected extensions by reading the account data.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The mint pubkey to verify
///
/// # Returns
/// Ok(()) if all extensions are present, or an error message describing what's missing
pub async fn verify_mint_extensions<R: Rpc>(rpc: &mut R, mint: &Pubkey) -> Result<(), String> {
    let account = rpc
        .get_account(*mint)
        .await
        .map_err(|e| format!("Failed to get mint account: {:?}", e))?
        .ok_or_else(|| "Mint account not found".to_string())?;

    let mint_state = StateWithExtensions::<Mint>::unpack(&account.data)
        .map_err(|e| format!("Failed to unpack mint: {:?}", e))?;

    // Verify each extension is present using concrete types
    let mut missing = Vec::new();

    if mint_state.get_extension::<MintCloseAuthority>().is_err() {
        missing.push("MintCloseAuthority");
    }
    if mint_state.get_extension::<TransferFeeConfig>().is_err() {
        missing.push("TransferFeeConfig");
    }
    if mint_state.get_extension::<DefaultAccountState>().is_err() {
        missing.push("DefaultAccountState");
    }
    if mint_state.get_extension::<PermanentDelegate>().is_err() {
        missing.push("PermanentDelegate");
    }
    if mint_state.get_extension::<TransferHook>().is_err() {
        missing.push("TransferHook");
    }
    if mint_state.get_extension::<MetadataPointer>().is_err() {
        missing.push("MetadataPointer");
    }
    if mint_state.get_extension::<PausableConfig>().is_err() {
        missing.push("PausableConfig");
    }
    if mint_state
        .get_extension::<ConfidentialTransferMint>()
        .is_err()
    {
        missing.push("ConfidentialTransferMint");
    }
    if mint_state
        .get_extension::<ConfidentialTransferFeeConfig>()
        .is_err()
    {
        missing.push("ConfidentialTransferFeeConfig");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("Missing extensions: {:?}", missing))
    }
}

/// Creates a Token 2022 token account for the given mint.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `payer` - Transaction fee payer
/// * `mint` - The mint pubkey
/// * `owner` - The owner of the new token account
///
/// # Returns
/// The pubkey of the created token account
pub async fn create_token_22_account<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    use solana_system_interface::instruction as system_instruction;

    let token_account = Keypair::new();

    // Get mint account to determine extensions needed for token account
    let mint_account = rpc.get_account(*mint).await.unwrap().unwrap();
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();
    let mint_extensions = mint_state.get_extension_types().unwrap();

    // Calculate token account size with required extensions
    let account_len = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(
        &mint_extensions,
    )
    .unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(account_len)
        .await
        .unwrap();

    // Create account instruction
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        account_len as u64,
        &spl_token_2022::ID,
    );

    // Initialize token account
    let init_account_ix = spl_token_2022::instruction::initialize_account3(
        &spl_token_2022::ID,
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_account_ix, init_account_ix],
        &payer.pubkey(),
        &[payer, &token_account],
    )
    .await
    .unwrap();

    token_account.pubkey()
}

/// Mints Token 2022 tokens to a token account.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint_authority` - The mint authority keypair (must sign)
/// * `mint` - The mint pubkey
/// * `token_account` - The destination token account
/// * `amount` - Amount to mint
pub async fn mint_spl_tokens_22<R: Rpc>(
    rpc: &mut R,
    mint_authority: &Keypair,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
) {
    let mint_to_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::ID,
        mint,
        token_account,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_ix], &mint_authority.pubkey(), &[mint_authority])
        .await
        .unwrap();
}

/// Pause a Token 2022 mint by modifying the PausableConfig extension.
///
/// This function reads the mint account, locates the PausableConfig extension,
/// sets paused = true, and writes the modified data back using set_account.
///
/// # Arguments
/// * `rpc` - RPC client (must support set_account, e.g., LightProgramTest)
/// * `mint_pubkey` - The mint pubkey to pause
pub async fn pause_mint(rpc: &mut light_program_test::LightProgramTest, mint_pubkey: &Pubkey) {
    use spl_token_2022::extension::BaseStateWithExtensionsMut;

    // Read mint account
    let mut account = rpc.get_account(*mint_pubkey).await.unwrap().unwrap();

    // Parse mint and get extension offset
    {
        let mut mint_state = StateWithExtensionsMut::<Mint>::unpack(&mut account.data).unwrap();
        let pausable_config = mint_state.get_extension_mut::<PausableConfig>().unwrap();
        pausable_config.paused = true.into();
    }

    // Write back modified account
    rpc.context.set_account(*mint_pubkey, account).unwrap();
}

/// Modify the TransferFeeConfig extension on a Token 2022 mint.
///
/// This function modifies both older and newer transfer fee configs
/// to set non-zero fees for testing validation failures.
///
/// # Arguments
/// * `rpc` - RPC client (must support set_account, e.g., LightProgramTest)
/// * `mint_pubkey` - The mint pubkey to modify
/// * `basis_points` - Transfer fee basis points (e.g., 100 = 1%)
/// * `max_fee` - Maximum fee in token amount
pub async fn set_mint_transfer_fee(
    rpc: &mut light_program_test::LightProgramTest,
    mint_pubkey: &Pubkey,
    basis_points: u16,
    max_fee: u64,
) {
    use spl_token_2022::extension::BaseStateWithExtensionsMut;

    // Read mint account
    let mut account = rpc.get_account(*mint_pubkey).await.unwrap().unwrap();

    // Parse mint and modify extension
    {
        let mut mint_state = StateWithExtensionsMut::<Mint>::unpack(&mut account.data).unwrap();
        let transfer_fee_config = mint_state.get_extension_mut::<TransferFeeConfig>().unwrap();
        // Set newer_transfer_fee (active fee schedule)
        transfer_fee_config
            .newer_transfer_fee
            .transfer_fee_basis_points = basis_points.into();
        transfer_fee_config.newer_transfer_fee.maximum_fee = max_fee.into();
        // Also set older_transfer_fee for completeness
        transfer_fee_config
            .older_transfer_fee
            .transfer_fee_basis_points = basis_points.into();
        transfer_fee_config.older_transfer_fee.maximum_fee = max_fee.into();
    }

    // Write back modified account
    rpc.context.set_account(*mint_pubkey, account).unwrap();
}

/// Modify the TransferHook extension on a Token 2022 mint.
///
/// This function sets the transfer hook program_id to a non-nil value
/// for testing validation failures.
///
/// # Arguments
/// * `rpc` - RPC client (must support set_account, e.g., LightProgramTest)
/// * `mint_pubkey` - The mint pubkey to modify
/// * `program_id` - The transfer hook program_id to set
pub async fn set_mint_transfer_hook(
    rpc: &mut light_program_test::LightProgramTest,
    mint_pubkey: &Pubkey,
    program_id: Pubkey,
) {
    use spl_token_2022::extension::BaseStateWithExtensionsMut;

    // Read mint account
    let mut account = rpc.get_account(*mint_pubkey).await.unwrap().unwrap();

    // Parse mint and modify extension
    {
        let mut mint_state = StateWithExtensionsMut::<Mint>::unpack(&mut account.data).unwrap();
        let transfer_hook = mint_state.get_extension_mut::<TransferHook>().unwrap();
        transfer_hook.program_id = Some(program_id).try_into().unwrap();
    }

    // Write back modified account
    rpc.context.set_account(*mint_pubkey, account).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_config_struct() {
        // Basic struct test
        let config = Token22ExtensionConfig {
            mint: Pubkey::new_unique(),
            token_pool: Pubkey::new_unique(),
            close_authority: Pubkey::new_unique(),
            transfer_fee_config_authority: Pubkey::new_unique(),
            withdraw_withheld_authority: Pubkey::new_unique(),
            permanent_delegate: Pubkey::new_unique(),
            metadata_update_authority: Pubkey::new_unique(),
            pause_authority: Pubkey::new_unique(),
            confidential_transfer_authority: Pubkey::new_unique(),
            confidential_transfer_fee_authority: Pubkey::new_unique(),
            default_account_state_frozen: false,
        };

        assert_ne!(config.mint, config.close_authority);
    }
}
