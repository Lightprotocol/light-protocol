use light_zero_copy::num_trait::ZeroCopyNumTrait;

use crate::rent::{RentConfigTrait, SLOTS_PER_EPOCH};

/// Account state information needed for rent calculations
#[derive(Debug, Clone, Copy)]
pub struct AccountRentState {
    /// Size of the account in bytes
    pub num_bytes: u64,
    /// Current blockchain slot
    pub current_slot: u64,
    /// Current account balance in lamports
    pub current_lamports: u64,
    /// Slot when rent was last claimed
    pub last_claimed_slot: u64,
}

impl AccountRentState {
    /// Calculate the balance available for rent payments.
    ///
    /// The available balance is the current lamports minus:
    /// - `rent_exemption_lamports`: Solana's required minimum balance.
    /// - `compression_cost`: Reserved lamports for future compression operation (paid to forester)
    ///
    /// # Returns
    /// The lamports available for rent payments, or 0 if insufficient balance
    pub fn get_available_rent_balance(
        &self,
        rent_exemption_lamports: u64,
        compression_cost: u64,
    ) -> u64 {
        self.current_lamports
            .saturating_sub(rent_exemption_lamports)
            .saturating_sub(compression_cost)
    }

    /// Calculate the number of completed epochs between last claimed slot and current slot.
    /// This represents epochs for which rent can potentially be claimed.
    ///
    /// # Returns
    /// The number of complete epochs that have passed since rent was last claimed
    pub fn get_completed_epochs(&self) -> u64 {
        let last_claimed_epoch = slot_to_epoch(self.last_claimed_slot);
        let current_epoch = slot_to_epoch(self.current_slot);
        current_epoch.saturating_sub(last_claimed_epoch)
    }

    /// Calculate how many epochs of rent are required.
    ///
    /// # Type Parameters
    /// - `INCLUDE_NEXT_EPOCH`: If true, includes the next epoch (for compressibility checks)
    ///
    /// # Returns
    /// The number of epochs requiring rent payment
    pub fn get_required_epochs<const INCLUDE_NEXT_EPOCH: bool>(&self) -> u64 {
        let current_epoch = slot_to_epoch(self.current_slot);
        let last_claimed_epoch = slot_to_epoch(self.last_claimed_slot);

        let target_epoch = if INCLUDE_NEXT_EPOCH {
            current_epoch + 1
        } else {
            current_epoch
        };

        target_epoch.saturating_sub(last_claimed_epoch)
    }

    /// Check if the account is compressible based on its rent status.
    /// An account becomes compressible when it lacks sufficient rent for the current epoch + 1.
    ///
    /// # Returns
    /// - `Some(deficit)`: The account is compressible, returns the deficit amount including compression costs
    /// - `None`: The account is not compressible
    pub fn is_compressible(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> Option<u64> {
        let available_balance =
            self.get_available_rent_balance(rent_exemption_lamports, config.compression_cost());

        let required_epochs = self.get_required_epochs::<true>(); // include next epoch for compressibility check

        let rent_per_epoch = config.rent_curve_per_epoch(self.num_bytes);

        let total_required = rent_per_epoch * required_epochs;
        let is_compressible = available_balance < total_required;

        if is_compressible {
            // Include compression cost in deficit so forester can execute
            let deficit =
                total_required.saturating_sub(available_balance) + config.compression_cost();
            Some(deficit)
        } else {
            None
        }
    }

    /// Calculate rent that can be claimed for completed epochs.
    ///
    /// Rent can only be claimed for fully completed epochs, not the current ongoing epoch.
    /// If the account is compressible, returns None (should compress instead of claim).
    ///
    /// # Returns
    /// - `Some(amount)`: Claimable rent for completed epochs
    /// - `None`: Account is compressible and should be compressed instead
    pub fn calculate_claimable_rent(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> Option<u64> {
        // First check if account is compressible
        if self
            .is_compressible(config, rent_exemption_lamports)
            .is_some()
        {
            return None; // Should compress, not claim
        }
        let rent_per_epoch = config.rent_curve_per_epoch(self.num_bytes);
        Some(self.get_completed_epochs() * rent_per_epoch)
    }

    /// Calculate how lamports are distributed when closing an account.
    ///
    /// When a compressible account is closed:
    /// - Completed epoch rent goes to the rent sponsor
    /// - Partial epoch rent (unutilized) is returned to the user
    ///
    /// # Returns
    /// A `CloseDistribution` specifying how lamports are split
    pub fn calculate_close_distribution(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> CloseDistribution {
        let details = self.calculate_rent_details::<true>(config, rent_exemption_lamports);
        CloseDistribution {
            to_rent_sponsor: self.current_lamports - details.unutilized_lamports,
            to_user: details.unutilized_lamports,
        }
    }

    /// Get detailed rent calculation for an account.
    ///
    /// # Parameters
    /// - `config`: Rent configuration
    /// - `rent_exemption_lamports`: Solana's required minimum balance
    ///
    /// # Type Parameters
    /// - `INCLUDE_CURRENT_EPOCH`: Whether to include the current epoch in required epochs
    ///
    /// # Returns
    /// Detailed rent calculation including required epochs, funding status, and unutilized lamports
    pub fn calculate_rent_details<const INCLUDE_CURRENT_EPOCH: bool>(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> RentCalculation {
        let available_balance =
            self.get_available_rent_balance(rent_exemption_lamports, config.compression_cost());

        let required_epochs = self.get_required_epochs::<INCLUDE_CURRENT_EPOCH>();

        let rent_per_epoch = config.rent_curve_per_epoch(self.num_bytes);

        let epochs_funded = available_balance / rent_per_epoch;
        let unutilized_lamports =
            available_balance.saturating_sub(rent_per_epoch * required_epochs);

        RentCalculation {
            required_epochs,
            rent_per_epoch,
            epochs_funded,
            unutilized_lamports,
        }
    }
}

/// Result of rent calculation
#[derive(Debug, Clone, Copy)]
pub struct RentCalculation {
    /// Number of epochs requiring rent payment
    pub required_epochs: u64,
    /// Rent cost per epoch
    pub rent_per_epoch: u64,
    /// Number of epochs the account can fund
    pub epochs_funded: u64,
    /// Lamports not utilized for complete epochs
    pub unutilized_lamports: u64,
}

/// Distribution of lamports when closing an account
#[derive(Debug, Clone, Copy)]
pub struct CloseDistribution {
    /// Lamports going to rent sponsor (completed epochs)
    pub to_rent_sponsor: u64,
    /// Lamports returned to user (partial epoch)
    pub to_user: u64,
}

// ============================================================================
// Core Helper Functions
// ============================================================================

/// Convert a slot number to its epoch in Light Protocol's rent system.
///
/// Light Protocol uses 6,300 slots per epoch (~1.75 hours) for rent calculations,
/// which is different from Solana's standard epoch length of 432,000 slots.
#[inline(always)]
pub fn slot_to_epoch(slot: u64) -> u64 {
    slot / SLOTS_PER_EPOCH
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
    config: &impl RentConfigTrait,
    rent_exemption_lamports: u64,
) -> u64 {
    // Calculate epochs_paid using AccountRentState
    let state = AccountRentState {
        num_bytes,
        current_slot: 0, // current_slot not needed for epochs_paid calculation
        current_lamports,
        last_claimed_slot: last_claimed_slot.into(),
    };

    let epochs_paid = state
        .calculate_rent_details::<false>(config, rent_exemption_lamports)
        .epochs_funded;

    let last_claimed_epoch: u64 = slot_to_epoch(state.last_claimed_slot);

    // The last paid epoch is the last claimed epoch plus epochs paid minus 1
    // If no epochs are paid, the account is immediately compressible
    if epochs_paid > 0 {
        last_claimed_epoch + epochs_paid - 1
    } else {
        // No rent paid, last paid epoch is before last claimed
        last_claimed_epoch.saturating_sub(1)
    }
}
