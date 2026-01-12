use light_zero_copy::errors::ZeroCopyError;
use spl_token_2022::extension::ExtensionType;

use crate::state::ExtensionStructConfig;

/// Restricted extension types that require compression_only mode.
/// These extensions have special behaviors (pausable, permanent delegate, fees, hooks,
/// default frozen state) that are incompatible with standard compressed token transfers.
pub const RESTRICTED_EXTENSION_TYPES: [ExtensionType; 5] = [
    ExtensionType::Pausable,
    ExtensionType::PermanentDelegate,
    ExtensionType::TransferFeeConfig,
    ExtensionType::TransferHook,
    ExtensionType::DefaultAccountState,
];

/// Allowed mint extension types for Token accounts.
/// Extensions not in this list will cause account creation to fail.
///
/// Runtime constraints enforced by check_mint_extensions():
/// - TransferFeeConfig: fees must be zero
/// - DefaultAccountState: any state allowed (Initialized or Frozen)
/// - TransferHook: program_id must be nil (no hook execution)
pub const ALLOWED_EXTENSION_TYPES: [ExtensionType; 16] = [
    // Metadata extensions
    ExtensionType::MetadataPointer,
    ExtensionType::TokenMetadata,
    // Group extensions
    ExtensionType::InterestBearingConfig,
    ExtensionType::GroupPointer,
    ExtensionType::GroupMemberPointer,
    ExtensionType::TokenGroup,
    ExtensionType::TokenGroupMember,
    // Token 2022 extensions with runtime constraints
    ExtensionType::MintCloseAuthority,
    ExtensionType::TransferFeeConfig,
    ExtensionType::DefaultAccountState,
    ExtensionType::PermanentDelegate,
    ExtensionType::TransferHook,
    ExtensionType::Pausable,
    ExtensionType::ConfidentialTransferMint,
    ExtensionType::ConfidentialTransferFeeConfig,
    ExtensionType::ConfidentialMintBurn,
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
            | ExtensionType::DefaultAccountState
    )
}

/// Flags for mint extensions that affect Token account initialization and transfers
#[derive(Debug, Default, Clone, Copy)]
pub struct MintExtensionFlags {
    /// Whether the mint has the PausableAccount extension
    pub has_pausable: bool,
    /// Whether the mint has the PermanentDelegate extension
    pub has_permanent_delegate: bool,
    /// Whether the mint has the DefaultAccountState extension (restricted regardless of state)
    pub has_default_account_state: bool,
    /// Whether DefaultAccountState is currently set to Frozen (for Token account creation)
    pub default_state_frozen: bool,
    /// Whether the mint has the TransferFeeConfig extension
    pub has_transfer_fee: bool,
    /// Whether the mint has the TransferHook extension (with nil program_id)
    pub has_transfer_hook: bool,
}

impl MintExtensionFlags {
    pub fn num_token_account_extensions(&self) -> usize {
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

    /// Calculate the token account size based on extension flags.
    ///
    /// # Arguments
    /// * `compressible` - If true, includes the Compressible extension in the size calculation
    ///
    /// # Returns
    /// * `Ok(u64)` - The account size in bytes
    /// * `Err(ZeroCopyError)` - If extension size calculation fails
    pub fn calculate_account_size(&self, compressible: bool) -> Result<u64, ZeroCopyError> {
        // Use stack-allocated array to avoid heap allocation
        // Maximum 5 extensions: pausable, permanent_delegate, transfer_fee, transfer_hook, compressible
        let mut extensions: [ExtensionStructConfig; 5] = [
            ExtensionStructConfig::Placeholder0,
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
        if compressible {
            extensions[count] =
                ExtensionStructConfig::Compressible(crate::state::CompressibleExtensionConfig {
                    info: crate::state::CompressionInfoConfig { rent_config: () },
                });
            count += 1;
        }

        let exts = if count == 0 {
            None
        } else {
            Some(&extensions[..count])
        };
        crate::state::calculate_token_account_size(exts).map(|size| size as u64)
    }

    /// Returns true if mint has any restricted extensions.
    /// Restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook,
    /// DefaultAccountState) require compression_only mode when compressing tokens.
    pub const fn has_restricted_extensions(&self) -> bool {
        self.has_pausable
            || self.has_permanent_delegate
            || self.has_transfer_fee
            || self.has_transfer_hook
            || self.has_default_account_state
    }
}
