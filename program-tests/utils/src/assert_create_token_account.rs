use light_client::rpc::Rpc;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_program_test::LightProgramTest;
use light_token::instruction::get_associated_token_address;
use light_token_interface::{
    state::{
        extensions::CompressibleExtension, token::Token, AccountState, ExtensionStruct,
        PausableAccountExtension, PermanentDelegateAccountExtension, TransferFeeAccountExtension,
        TransferHookAccountExtension, ACCOUNT_TYPE_TOKEN_ACCOUNT,
    },
    BASE_TOKEN_ACCOUNT_SIZE,
};
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState, permanent_delegate::PermanentDelegate,
        transfer_fee::TransferFeeConfig, transfer_hook::TransferHook, BaseStateWithExtensions,
        ExtensionType, StateWithExtensions,
    },
    state::Mint,
};

#[derive(Debug, Clone)]
pub struct CompressibleData {
    pub compression_authority: Pubkey,
    pub rent_sponsor: Pubkey,
    pub num_prepaid_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_pubkey: bool,
    pub account_version: light_token_interface::state::TokenDataVersion,
    pub payer: Pubkey,
}

/// Derive expected Token-2022 extensions, state, and compression_only from the mint account
/// Returns (decimals, expected_state, expected_extensions, compression_only)
async fn get_expected_extensions_from_mint(
    rpc: &mut LightProgramTest,
    mint_pubkey: Pubkey,
) -> (Option<u8>, AccountState, Option<Vec<ExtensionStruct>>, bool) {
    let mint_account = match rpc.get_account(mint_pubkey).await {
        Ok(Some(account)) => account,
        _ => {
            // Mint account doesn't exist or can't be read - use defaults
            return (None, AccountState::Initialized, None, false);
        }
    };

    // Check if this is a Token-2022 mint (program owner)
    if mint_account.owner != spl_token_2022::ID {
        // Regular SPL Token mint - no extensions, not compression_only
        return (None, AccountState::Initialized, None, false);
    }

    // Parse mint with extensions
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data)
        .expect("Failed to unpack Token-2022 mint");

    let decimals = mint_state.base.decimals;

    // Determine expected account state from DefaultAccountState extension
    let expected_state = mint_state
        .get_extension::<DefaultAccountState>()
        .map(|ext| {
            let frozen_state: u8 = spl_token_2022::state::AccountState::Frozen.into();
            if ext.state == frozen_state {
                AccountState::Frozen
            } else {
                AccountState::Initialized
            }
        })
        .unwrap_or(AccountState::Initialized);

    // Build expected extensions based on mint extensions
    // Use ExtensionType checks for version compatibility
    let mut extensions = Vec::new();

    // Check for Pausable extension on mint -> PausableAccount on token
    // Use ExtensionType for compatibility with different spl-token-2022 versions
    let extension_types = mint_state.get_extension_types().unwrap_or_default();

    if extension_types.contains(&ExtensionType::Pausable) {
        extensions.push(ExtensionStruct::PausableAccount(PausableAccountExtension));
    }

    // Check for PermanentDelegate extension on mint -> PermanentDelegateAccount on token
    if mint_state.get_extension::<PermanentDelegate>().is_ok() {
        extensions.push(ExtensionStruct::PermanentDelegateAccount(
            PermanentDelegateAccountExtension,
        ));
    }

    // Check for TransferFee extension on mint -> TransferFeeAccount on token
    if mint_state.get_extension::<TransferFeeConfig>().is_ok() {
        extensions.push(ExtensionStruct::TransferFeeAccount(
            TransferFeeAccountExtension { withheld_amount: 0 },
        ));
    }

    // Check for TransferHook extension on mint -> TransferHookAccount on token
    if mint_state.get_extension::<TransferHook>().is_ok() {
        extensions.push(ExtensionStruct::TransferHookAccount(
            TransferHookAccountExtension { transferring: 0 },
        ));
    }

    // compression_only is true if the mint has any extensions that require it
    let compression_only = !extensions.is_empty();

    let expected_extensions = if extensions.is_empty() {
        None
    } else {
        Some(extensions)
    };

    (
        Some(decimals),
        expected_state,
        expected_extensions,
        compression_only,
    )
}

/// Assert that a token account was created correctly.
/// If compressible_data is provided, validates compressible token account with extensions.
/// If compressible_data is None, validates basic SPL token account.
/// If is_ata is true, expects 1 signer (payer only), otherwise expects 2 signers (token_account_keypair + payer).
/// Automatically detects idempotent mode by checking if account existed before transaction.
/// If expected_extensions is provided, uses those; otherwise reads mint account to derive expected Token-2022 extensions.
pub async fn assert_create_token_account_internal(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
    is_ata: bool,
    expected_extensions: Option<Vec<ExtensionStruct>>,
) {
    // Get the token account data
    let account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .expect("Failed to get token account")
        .expect("Token account should exist");

    // Verify basic account properties
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert!(account_info.lamports > 0);
    assert!(!account_info.executable);

    match compressible_data {
        Some(compressible_info) => {
            // Validate compressible token account
            let account_size = account_info.data.len();

            // Calculate expected lamports balance
            let rent_exemption = rpc
                .get_minimum_balance_for_rent_exemption(account_size)
                .await
                .expect("Failed to get rent exemption");

            let rent_with_compression = RentConfig::default().get_rent_with_compression_cost(
                account_size as u64,
                compressible_info.num_prepaid_epochs as u64,
            );
            let expected_lamports = rent_exemption + rent_with_compression;

            assert_eq!(
                account_info.lamports, expected_lamports,
                "Account should have rent-exempt balance ({}) plus prepaid rent with compression cost ({}) = {} lamports, but has {}",
                rent_exemption, rent_with_compression, expected_lamports, account_info.lamports
            );

            // Use zero-copy deserialization for compressible account
            let (actual_token_account, _) = Token::zero_copy_at(&account_info.data)
                .expect("Failed to deserialize compressible token account with zero-copy");

            // Get current slot for validation (program sets this to current slot)
            let current_slot = rpc.get_slot().await.expect("Failed to get current slot");

            // Get expected extensions from mint account or use provided extensions
            let (decimals, expected_state, final_extensions, compression_only) =
                if let Some(provided_extensions) = expected_extensions {
                    // Use provided extensions - derive decimals and state from mint
                    let (decimals, expected_state, _, _) =
                        get_expected_extensions_from_mint(rpc, mint_pubkey).await;
                    let compression_only = !provided_extensions.is_empty();
                    (
                        decimals,
                        expected_state,
                        Some(provided_extensions),
                        compression_only,
                    )
                } else {
                    get_expected_extensions_from_mint(rpc, mint_pubkey).await
                };

            // Build the Compressible extension
            // ATAs are always compression_only regardless of mint extensions
            let compressible_ext = CompressibleExtension {
                decimals_option: if decimals.is_some() { 1 } else { 0 },
                decimals: decimals.unwrap_or(0),
                compression_only: is_ata || compression_only,
                is_ata: is_ata as u8,
                info: CompressionInfo {
                    config_account_version: 1,
                    last_claimed_slot: current_slot,
                    rent_exemption_paid: rent_exemption as u32,
                    _reserved: 0,
                    rent_config: RentConfig::default(),
                    lamports_per_write: compressible_info.lamports_per_write.unwrap_or(0),
                    compression_authority: compressible_info.compression_authority.to_bytes(),
                    rent_sponsor: compressible_info.rent_sponsor.to_bytes(),
                    compress_to_pubkey: compressible_info.compress_to_pubkey as u8,
                    account_version: compressible_info.account_version as u8,
                },
            };

            // Add Compressible extension to extensions list (at beginning, matching program order)
            let mut all_extensions = final_extensions.unwrap_or_default();
            all_extensions.insert(0, ExtensionStruct::Compressible(compressible_ext));

            // Create expected compressible token account with embedded compression info
            let expected_token_account = Token {
                mint: mint_pubkey.into(),
                owner: owner_pubkey.into(),
                amount: 0,
                delegate: None,
                state: expected_state,
                is_native: None,
                delegated_amount: 0,
                close_authority: None,
                account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
                extensions: Some(all_extensions),
            };

            assert_eq!(actual_token_account, expected_token_account);

            // Check if account existed before transaction (for idempotent mode)
            // Account "existed" only if it had data (was initialized), not just lamports
            let pre_tx_account = rpc.get_pre_transaction_account(&token_account_pubkey);
            let account_existed_before = pre_tx_account
                .as_ref()
                .map(|acc| !acc.data.is_empty())
                .unwrap_or(false);
            // Get pre-existing lamports (e.g., from attacker donation for DoS prevention test)
            let pre_existing_lamports = pre_tx_account.map(|acc| acc.lamports).unwrap_or(0);

            // Assert payer and rent sponsor balance changes
            let payer_balance_before = rpc
                .get_pre_transaction_account(&compressible_info.payer)
                .expect("Payer should exist in pre-transaction context")
                .lamports;

            let payer_balance_after = rpc
                .get_account(compressible_info.payer)
                .await
                .expect("Failed to get payer account")
                .expect("Payer should exist")
                .lamports;

            let rent_sponsor_balance_before = rpc
                .get_pre_transaction_account(&compressible_info.rent_sponsor)
                .expect("Rent sponsor should exist in pre-transaction context")
                .lamports;

            let rent_sponsor_balance_after = rpc
                .get_account(compressible_info.rent_sponsor)
                .await
                .expect("Failed to get rent sponsor account")
                .expect("Rent sponsor should exist")
                .lamports;

            // Transaction fee: 5000 lamports per signature
            // For ATA: 1 signer (payer only) = 5,000 lamports
            // For regular token account: 2 signers (token_account_keypair + payer) = 10,000 lamports
            let tx_fee = if is_ata { 5_000 } else { 10_000 };

            // If account existed before (idempotent mode), only tx fee is charged
            if account_existed_before {
                // In idempotent mode, account already existed, so only tx fee is paid
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    tx_fee,
                    "In idempotent mode (account already existed), payer should only pay tx fee ({} lamports), but paid {}",
                    tx_fee,
                    payer_balance_before - payer_balance_after
                );
                return;
            }

            // Check if payer is the rent sponsor (custom fee payer case)
            if compressible_info.payer == compressible_info.rent_sponsor {
                // Case 2: Custom fee payer - payer pays everything (rent_exemption + rent_with_compression + tx_fee)
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    rent_exemption + rent_with_compression + tx_fee,
                    "Custom fee payer should have paid {} lamports (rent exemption) + {} lamports (rent with compression cost) + {} lamports (tx fee) = {} total, but paid {}",
                    rent_exemption,
                    rent_with_compression,
                    tx_fee,
                    rent_exemption + rent_with_compression + tx_fee,
                    payer_balance_before - payer_balance_after
                );
            } else {
                // Case 1: With rent sponsor - split payment
                // Payer pays: rent_with_compression + tx_fee
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    rent_with_compression + tx_fee,
                    "Payer should have paid {} lamports (rent with compression cost) + {} lamports (tx fee) = {} total, but paid {}",
                    rent_with_compression,
                    tx_fee,
                    rent_with_compression + tx_fee,
                    payer_balance_before - payer_balance_after
                );

                // Rent sponsor pays: rent_exemption minus any pre-existing lamports
                // (pre-existing lamports from attacker donation are kept in the account)
                let expected_rent_sponsor_payment =
                    rent_exemption.saturating_sub(pre_existing_lamports);
                assert_eq!(
                    rent_sponsor_balance_before - rent_sponsor_balance_after,
                    expected_rent_sponsor_payment,
                    "Rent sponsor should have paid {} lamports (rent exemption {} - pre-existing {}), but paid {}",
                    expected_rent_sponsor_payment,
                    rent_exemption,
                    pre_existing_lamports,
                    rent_sponsor_balance_before - rent_sponsor_balance_after
                );
            }
        }
        None => {
            // Validate basic SPL token account
            assert_eq!(account_info.data.len(), BASE_TOKEN_ACCOUNT_SIZE as usize); // SPL token account size

            // Use SPL token Pack trait for basic account
            let actual_spl_token_account =
                spl_token_2022::state::Account::unpack(&account_info.data[..165])
                    .expect("Failed to unpack basic token account data");

            // Create expected SPL token account
            let expected_spl_token_account = spl_token_2022::state::Account {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: actual_spl_token_account.delegate, // Copy the actual COption value
                state: spl_token_2022::state::AccountState::Initialized,
                is_native: actual_spl_token_account.is_native, // Copy the actual COption value
                delegated_amount: 0,
                close_authority: actual_spl_token_account.close_authority, // Copy the actual COption value
            };

            assert_eq!(actual_spl_token_account, expected_spl_token_account);
            assert_eq!(account_info.data.len(), BASE_TOKEN_ACCOUNT_SIZE as usize);

            // Calculate expected lamports balance
            let rent_exemption = rpc
                .get_minimum_balance_for_rent_exemption(BASE_TOKEN_ACCOUNT_SIZE as usize)
                .await
                .expect("Failed to get rent exemption");
            assert_eq!(
                account_info.lamports, rent_exemption,
                "Account should have rent-exempt balance ({}) lamports, but has {}",
                rent_exemption, account_info.lamports
            );
        }
    }
}

/// Assert that a regular token account was created correctly.
/// Public wrapper for non-ATA token accounts (expects 2 signers).
/// If expected_extensions is provided, uses those; otherwise derives from mint.
pub async fn assert_create_token_account(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
    expected_extensions: Option<Vec<ExtensionStruct>>,
) {
    assert_create_token_account_internal(
        rpc,
        token_account_pubkey,
        mint_pubkey,
        owner_pubkey,
        compressible_data,
        false, // Not an ATA
        expected_extensions,
    )
    .await;
}

/// Assert that an associated token account was created correctly.
/// Automatically derives the ATA address from owner and mint.
/// If compressible_data is provided, validates compressible ATA with extensions.
/// If compressible_data is None, validates basic SPL ATA.
/// If expected_extensions is provided, uses those; otherwise derives from mint.
pub async fn assert_create_associated_token_account(
    rpc: &mut LightProgramTest,
    owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
    expected_extensions: Option<Vec<ExtensionStruct>>,
) {
    // Derive the associated token account address
    let ata_pubkey = get_associated_token_address(&owner_pubkey, &mint_pubkey);

    // Verify the account exists at the derived address
    let account = rpc
        .get_account(ata_pubkey)
        .await
        .expect("Failed to get ATA account");

    assert!(
        account.is_some(),
        "ATA should exist at derived address {} for owner {} and mint {}",
        ata_pubkey,
        owner_pubkey,
        mint_pubkey
    );

    // Use the internal assertion function with is_ata=true (expects 1 signer)
    assert_create_token_account_internal(
        rpc,
        ata_pubkey,
        mint_pubkey,
        owner_pubkey,
        compressible_data,
        true, // Is an ATA
        expected_extensions,
    )
    .await;
}
