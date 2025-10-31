use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_program_profiler::profile;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use pinocchio::pubkey::Pubkey;
use zerocopy::U64;

use crate::{
    config::CompressibleConfig,
    error::CompressibleError,
    rent::{
        get_last_funded_epoch, get_rent_exemption_lamports, AccountRentState, RentConfig,
        RentConfigTrait, SLOTS_PER_EPOCH,
    },
    AnchorDeserialize, AnchorSerialize,
};

/// Compressible extension for ctoken accounts.
#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    Pod,
    Zeroable,
)]
#[repr(C)]
#[aligned_sized]
pub struct CompressionInfo {
    pub config_account_version: u16, // config_account_version 0 is uninitialized, default is 1
    /// Compress to account pubkey instead of account owner.
    pub compress_to_pubkey: u8,
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The account_version specifies the hashing scheme.)
    pub account_version: u8,
    /// Lamports amount the account is topped up with at every write
    /// by the fee payer.
    pub lamports_per_write: u32,
    /// Authority that can compress and close the account.
    pub compression_authority: [u8; 32],
    /// Recipient for rent exemption lamports up on account closure.
    pub rent_sponsor: [u8; 32],
    /// Last slot rent was claimed from this account.
    pub last_claimed_slot: u64,
    /// Rent function parameters,
    /// used to calculate whether the account is compressible.
    pub rent_config: RentConfig,
}

// Unified macro for all compressible extension types
macro_rules! impl_is_compressible {
    ($struct_name:ty) => {
        impl $struct_name {
            /// current - last epoch = num epochs due
            /// rent_due
            /// available_balance = current_lamports - last_lamports
            ///     (we can never claim more lamports than rent is due)
            /// remaining_balance = available_balance - rent_due
            #[profile]
            pub fn is_compressible(
                &self,
                bytes: u64,
                current_slot: u64,
                current_lamports: u64,
            ) -> Result<Option<u64>, CompressibleError> {
                let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;
                Ok(crate::rent::AccountRentState {
                    num_bytes: bytes,
                    current_slot,
                    current_lamports,
                    last_claimed_slot: self.last_claimed_slot.into(),
                }
                .is_compressible(&self.rent_config, rent_exemption_lamports))
            }

            /// Converts the `compress_to_pubkey` field into a boolean.
            pub fn compress_to_pubkey(&self) -> bool {
                self.compress_to_pubkey != 0
            }

            /// Calculate the amount of lamports to top up during a write operation.
            /// Returns 0 if no top-up is needed (account is well-funded).
            /// Returns write_top_up + rent_deficit if account is compressible.
            /// Returns write_top_up if account needs more funding but isn't compressible yet.
            #[profile]
            pub fn calculate_top_up_lamports(
                &self,
                num_bytes: u64,
                current_slot: u64,
                current_lamports: u64,
                lamports_per_write: u32,
                rent_exemption_lamports: u64,
            ) -> Result<u64, CompressibleError> {
                // Calculate rent status using AccountRentState
                let state = crate::rent::AccountRentState {
                    num_bytes,
                    current_slot,
                    current_lamports,
                    last_claimed_slot: self.last_claimed_slot.into(),
                };
                let is_compressible =
                    state.is_compressible(&self.rent_config, rent_exemption_lamports);
                if let Some(rent_deficit) = is_compressible {
                    Ok(lamports_per_write as u64 + rent_deficit)
                } else {
                    // Calculate epochs funded ahead using available balance
                    let available_balance = state.get_available_rent_balance(
                        rent_exemption_lamports,
                        self.rent_config.compression_cost(),
                    );
                    let rent_per_epoch = self.rent_config.rent_curve_per_epoch(num_bytes);
                    let epochs_funded_ahead = available_balance / rent_per_epoch;
                    // Skip top-up if already funded for max_funded_epochs or more
                    if epochs_funded_ahead >= self.rent_config.max_funded_epochs as u64 {
                        Ok(0)
                    } else {
                        Ok(lamports_per_write as u64)
                    }
                }
            }
        }
    };
}
impl_is_compressible!(CompressionInfo);
impl_is_compressible!(ZCompressionInfo<'_>);
impl_is_compressible!(ZCompressionInfoMut<'_>);

// Unified macro to implement get_last_funded_epoch for all extension types
macro_rules! impl_get_last_paid_epoch {
    ($struct_name:ty) => {
        impl $struct_name {
            /// Get the last epoch that has been paid for.
            /// Returns the epoch number through which rent has been prepaid.
            pub fn get_last_funded_epoch(
                &self,
                num_bytes: u64,
                current_lamports: u64,
                rent_exemption_lamports: u64,
            ) -> Result<u64, CompressibleError> {
                Ok(get_last_funded_epoch(
                    num_bytes,
                    current_lamports,
                    self.last_claimed_slot,
                    &self.rent_config,
                    rent_exemption_lamports,
                ))
            }
        }
    };
}

impl_get_last_paid_epoch!(CompressionInfo);
impl_get_last_paid_epoch!(ZCompressionInfo<'_>);
impl_get_last_paid_epoch!(ZCompressionInfoMut<'_>);

pub struct ClaimAndUpdate<'a> {
    pub compression_authority: &'a Pubkey,
    pub rent_sponsor: &'a Pubkey,
    pub config_account: &'a CompressibleConfig,
    pub bytes: u64,
    pub current_slot: u64,
    pub current_lamports: u64,
}

impl ZCompressionInfoMut<'_> {
    /// Claim rent for past completed epochs and update the extension state.
    /// Returns the amount of lamports claimed, or None if account should be compressed.
    pub fn claim(
        &mut self,
        num_bytes: u64,
        current_slot: u64,
        current_lamports: u64,
        rent_exemption_lamports: u64,
    ) -> Result<Option<u64>, CompressibleError> {
        let state = AccountRentState {
            num_bytes,
            current_slot,
            current_lamports,
            last_claimed_slot: self.last_claimed_slot.into(),
        };
        let claimed = state.calculate_claimable_rent(&self.rent_config, rent_exemption_lamports);

        if let Some(claimed_amount) = claimed {
            if claimed_amount > 0 {
                let completed_epochs = state.get_completed_epochs();

                self.last_claimed_slot += U64::from(completed_epochs * SLOTS_PER_EPOCH);
                Ok(Some(claimed_amount))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn claim_and_update(
        &mut self,
        ClaimAndUpdate {
            compression_authority,
            rent_sponsor,
            config_account,
            bytes,
            current_slot,
            current_lamports,
        }: ClaimAndUpdate,
    ) -> Result<Option<u64>, CompressibleError> {
        if self.compression_authority != *compression_authority {
            #[cfg(feature = "solana")]
            solana_msg::msg!("Rent authority mismatch");
            return Ok(None);
        }
        if self.rent_sponsor != *rent_sponsor {
            #[cfg(feature = "solana")]
            solana_msg::msg!("Rent sponsor PDA does not match rent recipient");
            return Ok(None);
        }

        // Verify config version matches
        let account_version: u16 = self.config_account_version.into();
        let config_version = config_account.version;

        if account_version != config_version {
            #[cfg(feature = "solana")]
            solana_msg::msg!(
                "Config version mismatch: account has v{}, config is v{}",
                account_version,
                config_version
            );
            return Err(CompressibleError::InvalidVersion);
        }

        let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;

        let claim_result = self.claim(
            bytes,
            current_slot,
            current_lamports,
            rent_exemption_lamports,
        )?;

        // Update RentConfig after claim calculation (even if claim_result is None)
        self.rent_config.set(&config_account.rent_config);

        Ok(claim_result)
    }
}
