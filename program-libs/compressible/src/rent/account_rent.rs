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
    #[inline(always)]
    pub fn get_available_rent_balance(
        &self,
        rent_exemption_lamports: u64,
        compression_cost: u64,
    ) -> u64 {
        self.current_lamports
            .saturating_sub(rent_exemption_lamports)
            .saturating_sub(compression_cost)
    }

    /// The number of complete epochs that have passed since rent was last claimed.
    #[inline(always)]
    pub fn get_completed_epochs(&self) -> u64 {
        self.get_required_epochs::<false>()
    }

    /// Calculate how many epochs of rent are required.
    ///
    /// # Type Parameters
    /// - `INCLUDE_ONGOING_EPOCH`: If true, includes the next epoch (for compressibility checks)
    ///
    /// # Returns
    /// The number of epochs requiring rent payment
    #[inline(always)]
    pub fn get_required_epochs<const INCLUDE_ONGOING_EPOCH: bool>(&self) -> u64 {
        let last_completed_epoch = slot_to_epoch(self.current_slot);
        let last_claimed_epoch = slot_to_epoch(self.last_claimed_slot);

        let target_epoch = if INCLUDE_ONGOING_EPOCH {
            last_completed_epoch + 1
        } else {
            last_completed_epoch
        };

        target_epoch.saturating_sub(last_claimed_epoch)
    }

    /// Check if the account is compressible based on its rent status.
    /// An account becomes compressible when it lacks sufficient rent for the current epoch + 1.
    ///
    /// # Returns
    /// - `Some(deficit)`: The account is compressible, returns the deficit amount including compression costs
    /// - `None`: The account is not compressible
    #[inline(always)]
    pub fn is_compressible(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> Option<u64> {
        let available_balance =
            self.get_available_rent_balance(rent_exemption_lamports, config.compression_cost());
        let required_epochs = self.get_required_epochs::<true>(); // include next epoch for compressibility check
        let rent_per_epoch = config.rent_curve_per_epoch(self.num_bytes);
        // Use saturating_mul to prevent overflow - cheaper than checked_mul (no branching)
        let lamports_due = rent_per_epoch.saturating_mul(required_epochs);

        if available_balance < lamports_due {
            // Include compression cost in deficit so forester can execute
            let deficit =
                (lamports_due + config.compression_cost()).saturating_sub(available_balance);
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
        // Use saturating_mul to prevent overflow - cheaper than checked_mul (no branching)
        Some(self.get_completed_epochs().saturating_mul(rent_per_epoch))
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
        let unutilized_lamports = self.get_unused_lamports(config, rent_exemption_lamports);

        CloseDistribution {
            to_rent_sponsor: self.current_lamports - unutilized_lamports,
            to_user: unutilized_lamports,
        }
    }
    /// Calculate unused lamports after accounting for rent and compression costs.
    ///
    /// # Parameters
    /// - `config`: Rent configuration
    /// - `rent_exemption_lamports`: Solana's required minimum balance
    ///
    /// # Returns
    /// The amount of unused lamports
    pub fn get_unused_lamports(
        &self,
        config: &impl RentConfigTrait,
        rent_exemption_lamports: u64,
    ) -> u64 {
        let available_balance =
            self.get_available_rent_balance(rent_exemption_lamports, config.compression_cost());
        let required_epochs = self.get_required_epochs::<true>();
        let rent_per_epoch = config.rent_curve_per_epoch(self.num_bytes);
        // Use saturating_mul to prevent overflow - cheaper than checked_mul (no branching)
        let lamports_due = rent_per_epoch.saturating_mul(required_epochs);

        available_balance.saturating_sub(lamports_due)
    }
}

/// Distribution of lamports when closing an account
#[derive(Debug, Clone, Copy)]
pub struct CloseDistribution {
    /// Lamports going to rent sponsor (completed epochs)
    pub to_rent_sponsor: u64,
    /// Lamports returned to user (partial epoch)
    pub to_user: u64,
}

/// First epoch is 0.
#[inline(always)]
pub fn slot_to_epoch(slot: u64) -> u64 {
    slot / SLOTS_PER_EPOCH
}

/// Forester helper function to index when an account will become compressible.
#[inline(always)]
pub fn get_last_funded_epoch(
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

    let available_balance =
        state.get_available_rent_balance(rent_exemption_lamports, config.compression_cost());
    let rent_per_epoch = config.rent_curve_per_epoch(state.num_bytes);
    let epochs_funded = available_balance / rent_per_epoch;

    let last_claimed_epoch: u64 = slot_to_epoch(state.last_claimed_slot);

    // The last paid epoch is the last claimed epoch plus epochs paid minus 1
    // If no epochs are paid, the account is immediately compressible.
    // Epochs start at 0.
    if epochs_funded > 0 {
        last_claimed_epoch + epochs_funded - 1
    } else {
        // No rent paid, last paid epoch is before last claimed
        last_claimed_epoch.saturating_sub(1)
    }
}
