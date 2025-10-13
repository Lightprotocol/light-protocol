use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use pinocchio::pubkey::Pubkey;
use solana_msg::msg;
use zerocopy::U64;

use crate::{
    config::CompressibleConfig,
    error::CompressibleError,
    rent::{
        calculate_rent_and_balance, calculate_rent_inner, claimable_lamports, get_last_paid_epoch,
        get_rent_exemption_lamports, RentConfig, SLOTS_PER_EPOCH,
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
            pub fn is_compressible(
                &self,
                bytes: u64,
                current_slot: u64,
                current_lamports: u64,
            ) -> Result<(bool, u64), CompressibleError> {
                let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;
                let base_rent: u64 = self.rent_config.base_rent.into();
                let lamports_per_byte_per_epoch: u64 =
                    self.rent_config.lamports_per_byte_per_epoch.into();
                let compression_cost: u64 = self.rent_config.compression_cost.into();

                Ok(calculate_rent_and_balance(
                    bytes,
                    current_slot,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    base_rent,
                    lamports_per_byte_per_epoch,
                    compression_cost,
                ))
            }

            /// Converts the `compress_to_pubkey` field into a boolean.
            pub fn compress_to_pubkey(&self) -> bool {
                self.compress_to_pubkey != 0
            }

            /// Calculate the amount of lamports to top up during a write operation.
            /// Returns 0 if no top-up is needed (account is well-funded).
            /// Returns write_top_up + rent_deficit if account is compressible.
            /// Returns write_top_up if account needs more funding but isn't compressible yet.
            pub fn calculate_top_up_lamports(
                &self,
                bytes: u64,
                current_slot: u64,
                current_lamports: u64,
                lamports_per_write: u32,
            ) -> Result<u64, CompressibleError> {
                // TODO: unify with calculate_rent_and_balance
                let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;
                let base_rent: u64 = self.rent_config.base_rent.into();
                let lamports_per_byte_per_epoch: u64 =
                    self.rent_config.lamports_per_byte_per_epoch.into();
                let compression_cost: u64 = self.rent_config.compression_cost.into();

                // Calculate rent status using the internal function to avoid duplication
                let (required_epochs, rent_per_epoch, epochs_paid, unutilized_lamports) =
                    calculate_rent_inner::<true>(
                        bytes,
                        current_slot,
                        current_lamports,
                        self.last_claimed_slot,
                        rent_exemption_lamports,
                        base_rent,
                        lamports_per_byte_per_epoch,
                        compression_cost,
                    );

                let is_compressible = epochs_paid < required_epochs;

                if is_compressible {
                    // Account is compressible, return write_top_up + rent deficit
                    let epochs_payable = required_epochs.saturating_sub(epochs_paid);
                    let payable = epochs_payable * rent_per_epoch + compression_cost;
                    let rent_deficit = payable.saturating_sub(unutilized_lamports);
                    Ok(lamports_per_write as u64 + rent_deficit)
                } else {
                    // Account is not compressible, check if we should still top up
                    let epochs_funded_ahead = epochs_paid.saturating_sub(required_epochs);

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

// Unified macro to implement get_last_paid_epoch for all extension types
macro_rules! impl_get_last_paid_epoch {
    ($struct_name:ty) => {
        impl $struct_name {
            /// Get the last epoch that has been paid for.
            /// Returns the epoch number through which rent has been prepaid.
            pub fn get_last_paid_epoch(
                &self,
                bytes: u64,
                current_lamports: u64,
                rent_exemption_lamports: u64,
            ) -> Result<u64, CompressibleError> {
                let base_rent: u64 = self.rent_config.base_rent.into();
                let lamports_per_byte_per_epoch: u64 =
                    self.rent_config.lamports_per_byte_per_epoch.into();
                let compression_cost: u64 = self.rent_config.compression_cost.into();

                Ok(get_last_paid_epoch(
                    bytes,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    base_rent,
                    lamports_per_byte_per_epoch,
                    compression_cost,
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
        bytes: u64,
        current_slot: u64,
        current_lamports: u64,
        rent_exemption_lamports: u64,
    ) -> Result<Option<u64>, CompressibleError> {
        let base_rent: u64 = self.rent_config.base_rent.into();
        let lamports_per_byte_per_epoch: u64 = self.rent_config.lamports_per_byte_per_epoch.into();
        let compression_cost: u64 = self.rent_config.compression_cost.into();

        // Calculate claimable amount
        let claimed = claimable_lamports(
            bytes,
            current_slot,
            current_lamports,
            self.last_claimed_slot,
            rent_exemption_lamports,
            base_rent,
            lamports_per_byte_per_epoch,
            compression_cost,
        );

        if let Some(claimed_amount) = claimed {
            if claimed_amount > 0 {
                let (completed_epochs, _, _, _) = calculate_rent_inner::<false>(
                    bytes,
                    current_slot,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    base_rent,
                    lamports_per_byte_per_epoch,
                    compression_cost,
                );

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
            msg!("Rent authority mismatch");
            return Ok(None);
        }
        if self.rent_sponsor != *rent_sponsor {
            msg!("Rent sponsor PDA does not match rent recipient");
            return Ok(None);
        }

        // Verify config version matches
        let account_version: u16 = self.config_account_version.into();
        let config_version = config_account.version;

        if account_version != config_version {
            msg!(
                "Config version mismatch: account has v{}, config is v{}",
                account_version,
                config_version
            );
            return Err(CompressibleError::InvalidVersion);
        }

        let rent_exemption_lamports = get_rent_exemption_lamports(bytes).unwrap();

        let claim_result = self.claim(
            bytes,
            current_slot,
            current_lamports,
            rent_exemption_lamports,
        )?;

        // // Calculate claim with current RentConfig
        // // let claim_result = self.claim(bytes, current_slot, current_lamports, base_lamports)?;
        // let base_rent: u64 = self.rent_config.base_rent.into();
        // let lamports_per_byte_per_epoch: u64 = self.rent_config.lamports_per_byte_per_epoch.into();
        // let compression_cost: u64 = self.rent_config.compression_cost.into();
        // // Calculate claimable amount
        // let claimed = claimable_lamports(
        //     bytes,
        //     current_slot,
        //     current_lamports,
        //     self.last_claimed_slot,
        //     rent_exemption_lamports,
        //     base_rent,
        //     lamports_per_byte_per_epoch,
        //     compression_cost,
        // );

        // let claim_result = if let Some(claimed_amount) = claimed {
        //     if claimed_amount > 0 {
        //         let (completed_epochs, _, _, _) = calculate_rent_inner::<false>(
        //             bytes,
        //             current_slot,
        //             current_lamports,
        //             self.last_claimed_slot,
        //             rent_exemption_lamports,
        //             base_rent,
        //             lamports_per_byte_per_epoch,
        //             compression_cost,
        //         );

        //         self.last_claimed_slot += U64::from(completed_epochs * SLOTS_PER_EPOCH);
        //         Some(claimed_amount)
        //     } else {
        //         None
        //     }
        // } else {
        //     None
        // };

        // Update RentConfig after claim calculation (even if claim_result is None)
        self.rent_config.set(&config_account.rent_config);

        Ok(claim_result)
    }
}
