//! Helper functions for creating Token 2022 mints with multiple extensions.
//!
//! This module provides utilities to create Token 2022 mints with various extensions
//! enabled for testing purposes.

use forester_utils::instructions::create_account::create_account_instruction;
use light_client::rpc::Rpc;
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
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
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

/// Creates a Token 2022 mint with all supported extensions initialized.
///
/// The following extensions are initialized:
/// - Mint close authority
/// - Transfer fees (set to zero)
/// - Default account state (set to Initialized)
/// - Permanent delegate
/// - Transfer hook (set to nil program id)
/// - Metadata pointer (points to mint itself)
/// - Pausable
/// - Confidential transfers (initialized, not enabled)
/// - Confidential transfer fee (set to zero)
///
/// Note: Confidential mint/burn requires additional setup after mint initialization
/// and is not included in this helper.
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
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let authority = payer.pubkey();

    // Define all extensions we want to initialize
    let extension_types = [
        ExtensionType::MintCloseAuthority,
        ExtensionType::TransferFeeConfig,
        ExtensionType::DefaultAccountState,
        ExtensionType::PermanentDelegate,
        ExtensionType::TransferHook,
        ExtensionType::MetadataPointer,
        ExtensionType::Pausable,
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::ConfidentialTransferFeeConfig,
    ];

    // Calculate the account size needed for all extensions
    let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&extension_types).unwrap();

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

    // Initialize extensions in the correct order (before initialize_mint)
    // Order matters - some extensions must be initialized before others

    // 1. Mint close authority
    let init_close_authority_ix =
        initialize_mint_close_authority(&spl_token_2022::ID, &mint_pubkey, Some(&authority))
            .unwrap();

    // 2. Transfer fee config (fees set to zero)
    let init_transfer_fee_ix = initialize_transfer_fee_config(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(&authority), // transfer_fee_config_authority
        Some(&authority), // withdraw_withheld_authority
        0,                // fee_basis_points (0 = no fee)
        0,                // max_fee (0 = no max)
    )
    .unwrap();

    // 3. Default account state (Initialized - not frozen by default)
    let init_default_state_ix = initialize_default_account_state(
        &spl_token_2022::ID,
        &mint_pubkey,
        &AccountState::Initialized,
    )
    .unwrap();

    // 4. Permanent delegate
    let init_permanent_delegate_ix =
        initialize_permanent_delegate(&spl_token_2022::ID, &mint_pubkey, &authority).unwrap();

    // 5. Transfer hook (nil program - no hook)
    let init_transfer_hook_ix = initialize_transfer_hook(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(authority),
        None, // No transfer hook program
    )
    .unwrap();

    // 6. Metadata pointer (points to mint itself for embedded metadata)
    let init_metadata_pointer_ix = initialize_metadata_pointer(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(authority),   // authority
        Some(mint_pubkey), // metadata address (self-referential)
    )
    .unwrap();

    // 7. Pausable
    let init_pausable_ix =
        initialize_pausable(&spl_token_2022::ID, &mint_pubkey, &authority).unwrap();

    // 8. Confidential transfer mint (initialized but not auto-approve, no auditor)
    let init_confidential_transfer_ix = initialize_confidential_transfer_mint(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(authority), // authority
        false,           // auto_approve_new_accounts
        None,            // auditor_elgamal_pubkey (none)
    )
    .unwrap();

    // 9. Confidential transfer fee config (fees set to zero, no authority)
    // Using zeroed ElGamal pubkey since we're not enabling confidential fees
    let init_confidential_fee_ix = initialize_confidential_transfer_fee_config(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(authority),              // authority
        &PodElGamalPubkey::default(), // zeroed withdraw_withheld_authority_elgamal_pubkey
    )
    .unwrap();

    // 10. Initialize mint (must come after extension inits)
    let init_mint_ix = initialize_mint(
        &spl_token_2022::ID,
        &mint_pubkey,
        &authority,       // mint_authority
        Some(&authority), // freeze_authority (required for DefaultAccountState)
        decimals,
    )
    .unwrap();

    // 11. Create token pool for compressed tokens (restricted=true for mints with restricted extensions)
    let (token_pool_pubkey, _) = find_spl_interface_pda(&mint_pubkey, true);
    let create_token_pool_ix =
        CreateSplInterfacePda::new(authority, mint_pubkey, spl_token_2022::ID, true).instruction();

    // Combine all instructions
    let instructions: Vec<Instruction> = vec![
        create_account_ix,
        init_close_authority_ix,
        init_transfer_fee_ix,
        init_default_state_ix,
        init_permanent_delegate_ix,
        init_transfer_hook_ix,
        init_metadata_pointer_ix,
        init_pausable_ix,
        init_confidential_transfer_ix,
        init_confidential_fee_ix,
        init_mint_ix,
        create_token_pool_ix,
    ];

    // Send transaction
    rpc.create_and_send_transaction(&instructions, &authority, &[payer, &mint_keypair])
        .await
        .unwrap();

    let config = Token22ExtensionConfig {
        mint: mint_pubkey,
        token_pool: token_pool_pubkey,
        close_authority: authority,
        transfer_fee_config_authority: authority,
        withdraw_withheld_authority: authority,
        permanent_delegate: authority,
        metadata_update_authority: authority,
        pause_authority: authority,
        confidential_transfer_authority: authority,
        confidential_transfer_fee_authority: authority,
        default_account_state_frozen: false,
    };

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
