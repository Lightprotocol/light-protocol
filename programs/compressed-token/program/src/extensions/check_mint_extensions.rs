use anchor_compressed_token::{ErrorCode, ALLOWED_EXTENSION_TYPES};
use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_ctoken_interface::state::ExtensionStructConfig;
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

/// Restricted extension types that require compression_only mode.
/// These extensions have special behaviors (pausable, permanent delegate, fees, hooks)
/// that are incompatible with standard compressed token transfers.
pub const RESTRICTED_EXTENSION_TYPES: [ExtensionType; 4] = [
    ExtensionType::Pausable,
    ExtensionType::PermanentDelegate,
    ExtensionType::TransferFeeConfig,
    ExtensionType::TransferHook,
];

/// Check if an extension type is a restricted extension.
#[inline(always)]
pub const fn is_restricted_extension(ext: &ExtensionType) -> bool {
    matches!(
        ext,
        ExtensionType::Pausable
            | ExtensionType::PermanentDelegate
            | ExtensionType::TransferFeeConfig
            | ExtensionType::TransferHook
    )
}

/// Result of checking mint extensions (runtime validation)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MintExtensionChecks {
    /// The permanent delegate pubkey if the mint has the PermanentDelegate extension and it's set
    pub permanent_delegate: Option<Pubkey>,
    /// Whether the mint has the TransferFeeConfig extension (non-zero fees are rejected)
    pub has_transfer_fee: bool,
    /// Whether the mint has restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook)
    /// Used to require CompressedOnly output when compressing tokens from restricted mints
    pub has_restricted_extensions: bool,
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

impl MintExtensionFlags {
    pub fn num_extensions(&self) -> usize {
        let mut count = 0;
        if self.has_pausable {
            count += 1;
        }
        if self.has_permanent_delegate {
            count += 1;
        }
        if self.has_transfer_fee {
            count += 1;
        }
        if self.has_transfer_hook {
            count += 1;
        }
        count
    }

    /// Calculate the ctoken account size based on extension flags.
    ///
    /// Calculate account size based on mint extensions.
    /// All ctoken accounts now have CompressionInfo embedded in base struct.
    ///
    /// # Returns
    /// * `Ok(u64)` - The account size in bytes
    /// * `Err(ProgramError)` - If extension size calculation fails
    pub fn calculate_account_size(&self) -> Result<u64, ProgramError> {
        // Use stack-allocated array to avoid heap allocation
        // Maximum 4 extensions: pausable, permanent_delegate, transfer_fee, transfer_hook
        let mut extensions: [ExtensionStructConfig; 4] = [
            ExtensionStructConfig::Placeholder0,
            ExtensionStructConfig::Placeholder0,
            ExtensionStructConfig::Placeholder0,
            ExtensionStructConfig::Placeholder0,
        ];
        let mut count = 0;

        if self.has_pausable {
            extensions[count] = ExtensionStructConfig::PausableAccount(());
            count += 1;
        }
        if self.has_permanent_delegate {
            extensions[count] = ExtensionStructConfig::PermanentDelegateAccount(());
            count += 1;
        }
        if self.has_transfer_fee {
            extensions[count] = ExtensionStructConfig::TransferFeeAccount(());
            count += 1;
        }
        if self.has_transfer_hook {
            extensions[count] = ExtensionStructConfig::TransferHookAccount(());
            count += 1;
        }

        let exts = if count == 0 {
            None
        } else {
            Some(&extensions[..count])
        };
        light_ctoken_interface::state::calculate_ctoken_account_size(exts)
            .map(|size| size as u64)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    /// Returns true if mint has any restricted extensions.
    /// Restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook)
    /// require compression_only mode when compressing tokens.
    pub const fn has_restricted_extensions(&self) -> bool {
        self.has_pausable
            || self.has_permanent_delegate
            || self.has_transfer_fee
            || self.has_transfer_hook
    }
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
    deny_restricted_extensions: bool,
) -> Result<MintExtensionChecks, ProgramError> {
    // Only Token-2022 mints can have extensions
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(MintExtensionChecks {
            permanent_delegate: None,
            has_transfer_fee: false,
            has_restricted_extensions: false,
        });
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Zero-copy parse mint with extensions using PodStateWithExtensions
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Always compute has_restricted_extensions (needed for CompressAndClose validation)
    let extension_types = mint_state.get_extension_types()?;
    let has_restricted_extensions = extension_types.iter().any(is_restricted_extension);

    // When there are output compressed accounts, mint must not contain restricted extensions.
    // Restricted extensions require compression_only mode (no compressed outputs).
    if deny_restricted_extensions && has_restricted_extensions {
        msg!("Mint has restricted extensions - compression_only mode required");
        return Err(ErrorCode::MintHasRestrictedExtensions.into());
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
    let has_transfer_fee =
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
            true
        } else {
            false
        };

    // Check transfer hook extension - only nil program_id supported
    if let Ok(transfer_hook) = mint_state.get_extension::<TransferHook>() {
        if Option::<solana_pubkey::Pubkey>::from(transfer_hook.program_id).is_some() {
            return Err(ErrorCode::TransferHookNotSupported.into());
        }
    }

    Ok(MintExtensionChecks {
        permanent_delegate,
        has_transfer_fee,
        has_restricted_extensions,
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
    let extension_types = mint_state.get_extension_types()?;

    // Check for unsupported extensions and collect flags in a single pass
    let mut has_pausable = false;
    let mut has_permanent_delegate = false;
    let mut has_transfer_fee = false;
    let mut has_transfer_hook = false;
    let mut has_default_account_state = false;

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
        default_state_frozen: default_account_state_frozen,
        has_transfer_fee,
        has_transfer_hook,
    })
}
