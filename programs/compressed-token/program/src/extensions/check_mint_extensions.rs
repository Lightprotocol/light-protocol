use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_token_interface::{is_restricted_extension, MintExtensionFlags, ALLOWED_EXTENSION_TYPES};
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

/// Result of checking mint extensions (runtime validation)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MintExtensionChecks {
    /// The permanent delegate pubkey if the mint has the PermanentDelegate extension and it's set
    pub permanent_delegate: Option<Pubkey>,
    /// Whether the mint has the TransferFeeConfig extension (non-zero fees are rejected)
    pub has_transfer_fee: bool,
    /// Whether the mint has restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook, DefaultAccountState)
    /// Used to require CompressedOnly output when compressing tokens from restricted mints
    pub has_restricted_extensions: bool,
    /// Whether the mint is paused (PausableConfig.paused == true)
    /// CompressAndClose bypasses this check
    pub is_paused: bool,
    /// Whether the mint has non-zero transfer fees
    /// CompressAndClose bypasses this check
    pub has_non_zero_transfer_fee: bool,
    /// Whether the mint has a non-nil transfer hook program_id
    /// CompressAndClose bypasses this check
    pub has_non_nil_transfer_hook: bool,
}

/// Parse mint extensions in a single pass with zero-copy deserialization.
/// This function deserializes the mint once and extracts extension information.
/// It does NOT throw errors for invalid extension states (paused, non-zero fees, non-nil hook).
/// Use `check_mint_extensions` wrapper to enforce state validation.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
///
/// # Returns
/// * `Ok(MintExtensionChecks)` - Extension check results including `has_invalid_extension_state`
/// * `Err(ProgramError)` - If there's an error parsing the mint account
pub fn parse_mint_extensions(
    mint_account: &AccountInfo,
) -> Result<MintExtensionChecks, ProgramError> {
    // Only Token-2022 mints can have extensions
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(MintExtensionChecks::default());
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Always compute has_restricted_extensions (needed for CompressAndClose validation)
    let extension_types = mint_state.get_extension_types()?;
    let has_restricted_extensions = extension_types.iter().any(is_restricted_extension);

    // Check pausable extension
    let is_paused = mint_state
        .get_extension::<PausableConfig>()
        .map(|pausable_config| bool::from(pausable_config.paused))
        .unwrap_or(false);

    // Check permanent delegate extension
    let permanent_delegate =
        if let Ok(permanent_delegate_ext) = mint_state.get_extension::<PermanentDelegate>() {
            // Convert OptionalNonZeroPubkey to Option<Pubkey>
            Option::<solana_pubkey::Pubkey>::from(permanent_delegate_ext.delegate)
                .map(|delegate| Pubkey::from(delegate.to_bytes()))
        } else {
            None
        };

    // Check transfer fee extension
    let (has_transfer_fee, has_non_zero_transfer_fee) =
        if let Ok(transfer_fee_config) = mint_state.get_extension::<TransferFeeConfig>() {
            // Check both older and newer fee configs for non-zero values
            let older_fee = &transfer_fee_config.older_transfer_fee;
            let newer_fee = &transfer_fee_config.newer_transfer_fee;
            let has_non_zero = u16::from(older_fee.transfer_fee_basis_points) != 0
                || u64::from(older_fee.maximum_fee) != 0
                || u16::from(newer_fee.transfer_fee_basis_points) != 0
                || u64::from(newer_fee.maximum_fee) != 0;
            (true, has_non_zero)
        } else {
            (false, false)
        };

    // Check transfer hook extension - only nil program_id supported
    let has_non_nil_transfer_hook = mint_state
        .get_extension::<TransferHook>()
        .map(|transfer_hook| {
            Option::<solana_pubkey::Pubkey>::from(transfer_hook.program_id).is_some()
        })
        .unwrap_or(false);

    Ok(MintExtensionChecks {
        permanent_delegate,
        has_transfer_fee,
        has_restricted_extensions,
        is_paused,
        has_non_zero_transfer_fee,
        has_non_nil_transfer_hook,
    })
}

/// Check mint extensions and enforce state validation.
/// Wrapper around `parse_mint_extensions` that throws errors for invalid states.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
/// * `deny_restricted_extensions` - If true, fail if mint has restricted extensions
///
/// # Returns
/// * `Ok(MintExtensionChecks)` - Extension check results
/// * `Err(ErrorCode::MintPaused)` - If the mint is paused
/// * `Err(ErrorCode::NonZeroTransferFeeNotSupported)` - If transfer fees are non-zero
/// * `Err(ErrorCode::TransferHookNotSupported)` - If transfer hook program_id is non-nil
/// * `Err(ErrorCode::MintHasRestrictedExtensions)` - If deny_restricted_extensions and has restricted
/// * `Err(ProgramError)` - If there's an error parsing the mint account
#[inline(always)]
pub fn check_mint_extensions(
    mint_account: &AccountInfo,
    deny_restricted_extensions: bool,
) -> Result<MintExtensionChecks, ProgramError> {
    let checks = parse_mint_extensions(mint_account)?;

    // When there are output compressed accounts, mint must not contain restricted extensions.
    // Restricted extensions require compression_only mode (no compressed outputs).
    if deny_restricted_extensions && checks.has_restricted_extensions {
        msg!("Mint has restricted extensions - compression_only mode required");
        return Err(ErrorCode::MintHasRestrictedExtensions.into());
    }

    // Check for invalid extension states - throw specific errors for each
    if checks.is_paused {
        return Err(ErrorCode::MintPaused.into());
    }
    if checks.has_non_zero_transfer_fee {
        return Err(ErrorCode::NonZeroTransferFeeNotSupported.into());
    }
    if checks.has_non_nil_transfer_hook {
        return Err(ErrorCode::TransferHookNotSupported.into());
    }

    Ok(checks)
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
#[inline(always)]
pub fn has_mint_extensions(mint_account: &AccountInfo) -> Result<MintExtensionFlags, ProgramError> {
    // Only Token-2022 mints can have extensions
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(MintExtensionFlags::default());
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Get all extension types in a single call
    let extension_types = mint_state.get_extension_types()?;

    // Check for unsupported extensions and collect flags in a single pass
    let mut has_pausable = false;
    let mut has_permanent_delegate = false;
    let mut has_transfer_fee = false;
    let mut has_transfer_hook = false;
    let mut has_default_account_state = false;
    let mut has_mint_close_authority = false;

    for ext in &extension_types {
        if !ALLOWED_EXTENSION_TYPES.contains(ext) {
            msg!("Unsupported mint extension: {:?}", ext);
            return Err(ErrorCode::MintWithInvalidExtension.into());
        }
        match ext {
            ExtensionType::Pausable => has_pausable = true,
            ExtensionType::PermanentDelegate => has_permanent_delegate = true,
            ExtensionType::TransferFeeConfig => has_transfer_fee = true,
            ExtensionType::TransferHook => has_transfer_hook = true,
            ExtensionType::DefaultAccountState => has_default_account_state = true,
            ExtensionType::MintCloseAuthority => has_mint_close_authority = true,
            _ => {}
        }
    }

    // Check if DefaultAccountState is set to Frozen
    // AccountState::Frozen as u8 = 2, ext.state is PodAccountState (u8)
    let default_account_state_frozen = if has_default_account_state {
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
        has_default_account_state,
        default_state_frozen: default_account_state_frozen,
        has_transfer_fee,
        has_transfer_hook,
        has_mint_close_authority,
    })
}
