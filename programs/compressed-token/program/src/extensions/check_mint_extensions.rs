use anchor_compressed_token::{ErrorCode, ALLOWED_EXTENSION_TYPES};
use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};
use spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState, pausable::PausableConfig,
        permanent_delegate::PermanentDelegate, transfer_fee::TransferFeeConfig,
        transfer_hook::TransferHook, BaseStateWithExtensions, ExtensionType,
        PodStateWithExtensions,
    },
    pod::PodMint,
    state::AccountState,
};

const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// Transfer fee configuration extracted from mint for current epoch
#[derive(Debug, Clone, Copy)]
pub struct TransferFeeInfo {
    /// Fee in basis points (100 = 1%)
    pub transfer_fee_basis_points: u16,
    /// Maximum fee cap
    pub maximum_fee: u64,
}

impl TransferFeeInfo {
    /// Calculate transfer fee for a given amount.
    /// Uses ceiling division to ensure fees are not under-collected.
    /// Fee = min(ceil(amount * basis_points / 10000), maximum_fee)
    pub fn calculate_fee(&self, amount: u64) -> Option<u64> {
        if self.transfer_fee_basis_points == 0 || amount == 0 {
            return Some(0);
        }

        let basis_points = self.transfer_fee_basis_points as u128;
        let numerator = (amount as u128).checked_mul(basis_points)?;

        // Ceiling division: (numerator + 10000 - 1) / 10000
        let raw_fee = numerator.checked_add(10_000 - 1)?.checked_div(10_000)?;

        let raw_fee_u64: u64 = raw_fee.try_into().ok()?;
        Some(core::cmp::min(raw_fee_u64, self.maximum_fee))
    }
}

/// Result of checking mint extensions (runtime validation)
pub struct MintExtensionChecks {
    /// The permanent delegate pubkey if the mint has the PermanentDelegate extension and it's set
    pub permanent_delegate: Option<Pubkey>,
    /// Transfer fee configuration for the current epoch if the mint has TransferFeeConfig
    pub transfer_fee: Option<TransferFeeInfo>,
}

impl MintExtensionChecks {
    /// Calculate transfer fee for a given amount. Returns 0 if no transfer fee extension.
    pub fn calculate_fee(&self, amount: u64) -> u64 {
        self.transfer_fee
            .as_ref()
            .and_then(|fee| fee.calculate_fee(amount))
            .unwrap_or(0)
    }
}

/// Flags for mint extensions that affect CToken account initialization and transfers
#[derive(Default, Clone, Copy)]
pub struct MintExtensionFlags {
    /// Whether the mint has the PausableAccount extension
    pub has_pausable: bool,
    /// Whether the mint has the PermanentDelegate extension
    pub has_permanent_delegate: bool,
    /// Whether the mint has DefaultAccountState set to Frozen
    pub default_state_frozen: bool,
    /// Whether the mint has the TransferFeeConfig extension
    pub has_transfer_fee: bool,
    /// Whether the mint has the TransferHook extension (with nil program_id)
    pub has_transfer_hook: bool,
}

/// Check mint extensions in a single pass with zero-copy deserialization.
/// This function deserializes the mint once and checks both pausable and permanent delegate extensions.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
///
/// # Returns
/// * `Ok(MintExtensionChecks)` - Extension check results
/// * `Err(ErrorCode::MintPaused)` - If the mint is paused
/// * `Err(ProgramError)` - If there's an error parsing the mint account
pub fn check_mint_extensions(
    mint_account: &AccountInfo,
    hotpath: bool,
) -> Result<MintExtensionChecks, ProgramError> {
    // Only Token-2022 mints can have extensions
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(MintExtensionChecks {
            permanent_delegate: None,
            transfer_fee: None,
        });
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // When NOT on hot path, mint must not contain restricted extensions.
    // Restricted extensions require compression_only mode (hot path).
    if !hotpath {
        let extension_types = mint_state.get_extension_types().unwrap_or_default();
        let has_restricted_extensions = extension_types.iter().any(|ext| {
            matches!(
                ext,
                ExtensionType::Pausable
                    | ExtensionType::PermanentDelegate
                    | ExtensionType::TransferFeeConfig
                    | ExtensionType::TransferHook
            )
        });

        if has_restricted_extensions {
            msg!("Mint has restricted extensions - hot path required");
            return Err(ErrorCode::MintHasRestrictedExtensions.into());
        }
    }

    // Check pausable extension first (early return if paused)
    if let Ok(pausable_config) = mint_state.get_extension::<PausableConfig>() {
        if bool::from(pausable_config.paused) {
            return Err(ErrorCode::MintPaused.into());
        }
    }

    // Check permanent delegate extension
    let permanent_delegate =
        if let Ok(permanent_delegate_ext) = mint_state.get_extension::<PermanentDelegate>() {
            // Convert OptionalNonZeroPubkey to Option<Pubkey>
            Option::<solana_pubkey::Pubkey>::from(permanent_delegate_ext.delegate)
                .map(|delegate| Pubkey::from(delegate.to_bytes()))
        } else {
            None
        };

    // Check transfer fee extension - non-zero fees not supported
    let transfer_fee =
        if let Ok(transfer_fee_config) = mint_state.get_extension::<TransferFeeConfig>() {
            // Check both older and newer fee configs for non-zero values
            let older_fee = &transfer_fee_config.older_transfer_fee;
            let newer_fee = &transfer_fee_config.newer_transfer_fee;
            if u16::from(older_fee.transfer_fee_basis_points) != 0
                || u64::from(older_fee.maximum_fee) != 0
                || u16::from(newer_fee.transfer_fee_basis_points) != 0
                || u64::from(newer_fee.maximum_fee) != 0
            {
                return Err(ErrorCode::NonZeroTransferFeeNotSupported.into());
            }
            Some(TransferFeeInfo {
                transfer_fee_basis_points: 0,
                maximum_fee: 0,
            })
        } else {
            None
        };

    // Check transfer hook extension - only nil program_id supported
    if let Ok(transfer_hook) = mint_state.get_extension::<TransferHook>() {
        if Option::<solana_pubkey::Pubkey>::from(transfer_hook.program_id).is_some() {
            return Err(ErrorCode::TransferHookNotSupported.into());
        }
    }

    Ok(MintExtensionChecks {
        permanent_delegate,
        transfer_fee,
    })
}

/// Hash which extensions a mint has in a single zero-copy deserialization.
/// This function is used during account creation to determine which marker extensions
/// should be added to the ctoken account.
///
/// Note: This function only checks which extensions exist, not their values.
/// For runtime validation (checking if paused, getting delegate pubkey), use `check_mint_extensions` instead.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
///
/// # Returns
/// * `Ok(MintExtensionFlags)` - Flags indicating which extensions the mint has
/// * `Err(ProgramError)` - If there's an error parsing the mint account
pub fn has_mint_extensions(mint_account: &AccountInfo) -> Result<MintExtensionFlags, ProgramError> {
    // Only Token-2022 mints can have extensions
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(MintExtensionFlags::default());
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Get all extension types in a single call
    let extension_types = mint_state.get_extension_types().unwrap_or_default();

    // Check for unsupported extensions
    for ext in &extension_types {
        if !ALLOWED_EXTENSION_TYPES.contains(ext) {
            msg!("Unsupported mint extension: {:?}", ext);
            return Err(ErrorCode::MintWithInvalidExtension.into());
        }
    }

    // Check which extensions exist using the extension_types list
    let has_pausable = extension_types.contains(&ExtensionType::Pausable);
    let has_permanent_delegate = extension_types.contains(&ExtensionType::PermanentDelegate);
    let has_transfer_fee = extension_types.contains(&ExtensionType::TransferFeeConfig);
    let has_transfer_hook = extension_types.contains(&ExtensionType::TransferHook);

    // Check if DefaultAccountState is set to Frozen
    // AccountState::Frozen as u8 = 2, ext.state is PodAccountState (u8)
    let default_account_state_frozen =
        if extension_types.contains(&ExtensionType::DefaultAccountState) {
            mint_state
                .get_extension::<DefaultAccountState>()
                .map(|ext| ext.state == AccountState::Frozen as u8)
                .unwrap_or(false)
        } else {
            false
        };

    Ok(MintExtensionFlags {
        has_pausable,
        has_permanent_delegate,
        default_state_frozen: default_account_state_frozen,
        has_transfer_fee,
        has_transfer_hook,
    })
}

/// Checks if an SPL Token 2022 mint has the Pausable extension.
/// Returns true if the mint has the Pausable extension, false otherwise.
///
/// Note: For account creation, use `has_mint_extensions` to check multiple extensions at once.
/// For runtime checks during transfers, use `check_mint_extensions` instead.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
pub fn mint_has_pausable_extension(mint_account: &AccountInfo) -> Result<bool, ProgramError> {
    let flags = has_mint_extensions(mint_account)?;
    Ok(flags.has_pausable)
}

/// Checks if an SPL Token 2022 mint has the PermanentDelegate extension.
/// Returns true if the mint has the extension, false otherwise.
///
/// Note: For account creation, use `has_mint_extensions` to check multiple extensions at once.
/// For runtime checks during transfers, use `check_mint_extensions` instead.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
pub fn mint_has_permanent_delegate_extension(
    mint_account: &AccountInfo,
) -> Result<bool, ProgramError> {
    let flags = has_mint_extensions(mint_account)?;
    Ok(flags.has_permanent_delegate)
}

/// Checks if an SPL Token 2022 mint has the Pausable extension and if it's currently paused.
/// Returns an error if the mint is paused, otherwise Ok(()).
///
/// This function should be called before any token operation (transfer, compress, decompress)
/// when the token account has the PausableAccount extension.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
///
/// # Errors
/// * `ErrorCode::MintPaused` - If the mint has PausableConfig and is currently paused
pub fn check_mint_not_paused(mint_account: &AccountInfo) -> Result<(), ProgramError> {
    // Only Token-2022 mints can have the Pausable extension
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(());
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Check if mint has PausableConfig extension (zero-copy)
    if let Ok(pausable_config) = mint_state.get_extension::<PausableConfig>() {
        // Check if paused
        if bool::from(pausable_config.paused) {
            return Err(ErrorCode::MintPaused.into());
        }
    }

    Ok(())
}
