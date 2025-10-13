use bytemuck::{Pod, Zeroable};
use light_program_profiler::profile;
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};

use crate::{error::CompressibleError, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSION_COST: u16 = 10_000;
pub const COMPRESSION_INCENTIVE: u16 = 1000;

pub const MIN_RENT: u16 = 88;
pub const RENT_PER_BYTE: u8 = 1;
pub const SLOTS_PER_EPOCH: u64 = 36_000; // 4h
use aligned_sized::aligned_sized;

/// Rent function parameters,
/// used to calculate whether the account is compressible.
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
pub struct RentConfig {
    /// Base rent constant: rent = base_rent + num_bytes * lamports_per_byte_per_epoch
    pub base_rent: u16,
    pub compression_cost: u16,
    pub lamports_per_byte_per_epoch: u8,
    pub max_funded_epochs: u8, // once the account is funded for max_funded_epochs top up per write is not executed
    pub _padding: [u8; 2],
}

impl RentConfig {
    pub fn rent_curve_per_epoch(&self, num_bytes: u64) -> u64 {
        rent_curve_per_epoch(
            self.base_rent as u64,
            self.lamports_per_byte_per_epoch as u64,
            num_bytes,
        )
    }
    pub fn get_rent(&self, num_bytes: u64, epochs: u64) -> u64 {
        self.rent_curve_per_epoch(num_bytes) * epochs
    }
    pub fn get_rent_with_compression_cost(&self, num_bytes: u64, epochs: u64) -> u64 {
        self.rent_curve_per_epoch(num_bytes) * epochs + self.compression_cost as u64
    }
}

impl ZRentConfigMut<'_> {
    /// Sets all fields from a RentConfig instance, handling zero-copy type conversions
    pub fn set(&mut self, config: &RentConfig) {
        self.base_rent = config.base_rent.into();
        self.compression_cost = config.compression_cost.into();
        self.lamports_per_byte_per_epoch = config.lamports_per_byte_per_epoch;
        self.max_funded_epochs = config.max_funded_epochs;
        self._padding = config._padding;
    }
}

pub fn rent_curve_per_epoch(
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    num_bytes: u64,
) -> u64 {
    base_rent + num_bytes * lamports_per_byte_per_epoch
}

pub fn get_rent(
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    num_bytes: u64,
    epochs: u64,
) -> u64 {
    rent_curve_per_epoch(base_rent, lamports_per_byte_per_epoch, num_bytes) * epochs
}

#[profile]
pub fn get_rent_with_compression_cost(
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    num_bytes: u64,
    epochs: u64,
    compression_costs: u64,
) -> u64 {
    get_rent(base_rent, lamports_per_byte_per_epoch, num_bytes, epochs) + compression_costs
}

#[track_caller]
pub fn get_rent_exemption_lamports(_num_bytes: u64) -> Result<u64, CompressibleError> {
    #[cfg(target_os = "solana")]
    {
        use pinocchio::sysvars::Sysvar;
        return pinocchio::sysvars::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_num_bytes as usize))
            .map_err(|_| CompressibleError::FailedBorrowRentSysvar);
    }
    #[cfg(not(target_os = "solana"))]
    {
        #[cfg(test)]
        {
            // Standard rent-exempt balance for tests: 890880 + 6.96 * bytes
            // This matches Solana's rent calculation
            return Ok(890_880 + ((696 * _num_bytes + 99) / 100));
        }
        #[cfg(not(test))]
        unimplemented!(
            "get_rent_exemption_lamports is only implemented for target os solana and tests"
        )
    }
}

impl Default for RentConfig {
    fn default() -> Self {
        Self {
            base_rent: MIN_RENT,
            compression_cost: COMPRESSION_COST + COMPRESSION_INCENTIVE,
            lamports_per_byte_per_epoch: RENT_PER_BYTE,
            max_funded_epochs: 2, // once the account is funded for max_funded_epochs top up per write is not executed
            _padding: [0; 2],
        }
    }
}

/// Calculate the last epoch that has been paid for.
/// Returns the epoch number through which rent has been prepaid.
///
/// # Returns
/// The last epoch number that is covered by rent payments.
/// This is calculated as: last_claimed_epoch + epochs_paid - 1
///
/// # Example
/// If an account was created in epoch 0 and paid for 3 epochs of rent,
/// the last paid epoch would be 2 (epochs 0, 1, and 2 are covered).
#[inline(always)]
pub fn get_last_paid_epoch(
    num_bytes: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> u64 {
    // Reuse the existing calculate_rent_inner function with INCLUDE_CURRENT=false
    // to get epochs_paid calculation
    let (_, _rent_per_epoch_calc, epochs_paid, _) = calculate_rent_inner::<false>(
        num_bytes,
        0, // current_slot not needed for epochs_paid calculation
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        base_rent,
        lamports_per_byte_per_epoch,
        compression_cost,
    );

    let last_claimed_epoch: u64 = last_claimed_slot.into() / SLOTS_PER_EPOCH;

    // The last paid epoch is the last claimed epoch plus epochs paid minus 1
    // If no epochs are paid, the account is immediately compressible
    if epochs_paid > 0 {
        last_claimed_epoch + epochs_paid - 1
    } else {
        // No rent paid, last paid epoch is before last claimed
        last_claimed_epoch.saturating_sub(1)
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn calculate_rent_and_balance(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (bool, u64) {
    let (required_epochs, rent_per_epoch, epochs_paid, unutilized_lamports) =
        calculate_rent_with_current_epoch(
            num_bytes,
            current_slot,
            current_lamports,
            last_claimed_slot,
            rent_exemption_lamports,
            base_rent,
            lamports_per_byte_per_epoch,
            compression_cost,
        );

    let is_compressible = epochs_paid < required_epochs;
    if is_compressible {
        let epochs_payable = required_epochs.saturating_sub(epochs_paid);
        let payable = epochs_payable * rent_per_epoch + compression_cost;
        // How many lamports do we need to fund rent for the current epoch.
        let net_payable = payable.saturating_sub(unutilized_lamports);
        (true, net_payable)
    } else {
        (false, 0)
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn calculate_rent_with_current_epoch(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (u64, u64, u64, u64) {
    calculate_rent_inner::<true>(
        num_bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        base_rent,
        lamports_per_byte_per_epoch,
        compression_cost,
    )
}

// TODO: return Result
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn calculate_rent_inner<const INCLUDE_CURRENT: bool>(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (u64, u64, u64, u64) {
    // TODO: return struct.
    let available_balance = current_lamports
        .checked_sub(rent_exemption_lamports + compression_cost)
        .unwrap();
    let current_epoch = if INCLUDE_CURRENT {
        current_slot / SLOTS_PER_EPOCH + 1
    } else {
        current_slot / SLOTS_PER_EPOCH
    };
    let last_claimed_epoch: u64 = last_claimed_slot.into() / SLOTS_PER_EPOCH;
    let required_epochs = current_epoch.saturating_sub(last_claimed_epoch);

    let rent_per_epoch = rent_curve_per_epoch(base_rent, lamports_per_byte_per_epoch, num_bytes);
    // Number of epochs the avaible balance can fund.
    let potentially_epochs_funded = available_balance / rent_per_epoch;
    // Lamports that are not blocked for unclaimed rent of past epochs.
    let unutilized_lamports = available_balance.saturating_sub(rent_per_epoch * required_epochs); // TODO: double check in test
    (
        required_epochs,
        rent_per_epoch,
        potentially_epochs_funded,
        unutilized_lamports,
    )
}
// lamports_to_rent_sponsor,  lamports_to_destination
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn calculate_close_lamports(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> (u64, u64) {
    let (_, _, _, unutilized_lamports) = calculate_rent_with_current_epoch(
        num_bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        base_rent,
        lamports_per_byte_per_epoch,
        compression_cost,
    );
    (current_lamports - unutilized_lamports, unutilized_lamports)
}

/// Calculate how many lamports can be claimed for past completed epochs.
/// Only rent for fully completed epochs can be claimed, not the current ongoing epoch.
/// Returns None if the account is compressible (should be compressed instead of claimed).
/// Compression costs are never claimable.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn claimable_lamports(
    num_bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    base_rent: u64,
    lamports_per_byte_per_epoch: u64,
    compression_cost: u64,
) -> Option<u64> {
    // First check if account is compressible
    let (is_compressible, _) = calculate_rent_and_balance(
        num_bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        base_rent,
        lamports_per_byte_per_epoch,
        compression_cost,
    );

    if is_compressible {
        // Account should be compressed, not claimed
        return None;
    }

    // Use calculate_rent_inner with INCLUDE_CURRENT=false to get only completed epochs
    let (completed_epochs, rent_per_epoch, _, _) = calculate_rent_inner::<false>(
        num_bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        base_rent,
        lamports_per_byte_per_epoch,
        compression_cost,
    );

    // Calculate how much rent we can claim for completed epochs
    Some(completed_epochs * rent_per_epoch)
}
