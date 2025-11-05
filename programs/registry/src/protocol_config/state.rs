use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_batched_merkle_tree::constants::DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2;

use crate::errors::RegistryError;

#[aligned_sized(anchor)]
#[derive(Debug)]
#[account]
pub struct ProtocolConfigPda {
    pub authority: Pubkey,
    pub bump: u8,
    pub config: ProtocolConfig,
}

/// Epoch Phases:
/// 1. Registration
/// 2. Active
/// 3. Report Work
/// 4. Post (Epoch has ended, and rewards can be claimed.)
/// - There is always an active phase in progress, registration and report work
///   phases run in parallel to a currently active phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct ProtocolConfig {
    /// Solana slot when the protocol starts operating.
    pub genesis_slot: u64,
    /// Minimum weight required for a forester to register to an epoch.
    pub min_weight: u64,
    /// Light protocol slot length
    pub slot_length: u64,
    /// Foresters can register for this phase.
    pub registration_phase_length: u64,
    /// Foresters can perform work in this phase.
    pub active_phase_length: u64,
    /// Foresters can report work to receive performance based rewards in this
    /// phase.
    pub report_work_phase_length: u64,
    pub network_fee: u64,
    pub cpi_context_size: u64,
    pub finalize_counter_limit: u64,
    /// Placeholder for future protocol updates.
    pub place_holder: Pubkey,
    pub address_network_fee: u64,
    pub place_holder_b: u64,
    pub place_holder_c: u64,
    pub place_holder_d: u64,
    pub place_holder_e: u64,
    pub place_holder_f: u64,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            genesis_slot: 0,
            min_weight: 1,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            network_fee: 5000,
            cpi_context_size: DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2,
            finalize_counter_limit: 100,
            place_holder: Pubkey::default(),
            address_network_fee: 10000,
            place_holder_b: 0,
            place_holder_c: 0,
            place_holder_d: 0,
            place_holder_e: 0,
            place_holder_f: 0,
        }
    }
}

impl ProtocolConfig {
    pub fn testnet_default() -> Self {
        Self {
            genesis_slot: 0,
            min_weight: 1,
            slot_length: 60,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            network_fee: 5000,
            cpi_context_size: DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2,
            finalize_counter_limit: 100,
            place_holder: Pubkey::default(),
            address_network_fee: 10000,
            place_holder_b: 0,
            place_holder_c: 0,
            place_holder_d: 0,
            place_holder_e: 0,
            place_holder_f: 0,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum EpochState {
    Registration,
    Active,
    ReportWork,
    Post,
    #[default]
    Pre,
}

/// Light Epoch Example:
///
///  Diagram of epochs 0 and 1.
/// Registration 0 starts at genesis slot.
/// |---- Registration 0 ----|------------------ Active 0 ------|---- Report Work 0 ----|---- Post 0 ----
///                                        |-- Registration 1 --|------------------ Active 1 -----------------
/// (Post epoch does not end unlike the other phases.)
///
/// let genesis = 0;
/// let registration_phase_length = 100;
/// let active_phase_length = 1000;
/// let report_work_phase_length = 100;
/// let slot = 10;
///
/// To get the latest registry epoch:
/// - slot = 0;
///   let current_registry_epoch = (slot - genesis) / active_phase_length;
///   current_registry_epoch =  (0 - 0) / 1000 = 0;
///   first active phase starts at genesis + registration_phase_length
///   = 0 + 100 = 100;
///
/// To get the current active epoch:
/// - slot = 100;
///   let current_active_epoch =
///   (slot - genesis - registration_phase_length) / active_phase_length;
///   current_active_epoch = (100 - 0 - 100) / 1000 = 0;
///
/// Epoch 0:
/// - Registration 0: 0 - 100
/// - Active 0: 100 - 1100
/// - Report Work 0: 1100 - 1200
/// - Post 0: 1200 - inf
///
/// Epoch 1:
/// - Registration 1: 1000 - 1100
/// - Active 1: 1100 - 2100
/// - Report Work 1: 2100 - 2200
/// - Post 1: 2200 - inf
///
/// Epoch 2:
/// - Registration 2: 2000 - 2100
/// - Active 2: 2100 - 3100
/// - Report Work 2: 3100 - 3200
/// - Post 2: 3200 - inf
///
impl ProtocolConfig {
    /// Current epoch including registration phase.
    pub fn get_latest_register_epoch(&self, slot: u64) -> Result<u64> {
        let slot = slot
            .checked_sub(self.genesis_slot)
            .ok_or(RegistryError::GetLatestRegisterEpochFailed)?;
        Ok(slot / self.active_phase_length)
    }

    pub fn get_current_epoch(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) / self.active_phase_length
    }

    pub fn get_current_active_epoch(&self, slot: u64) -> Result<u64> {
        let slot = slot
            .checked_sub(self.genesis_slot + self.registration_phase_length)
            .ok_or(RegistryError::GetCurrentActiveEpochFailed)?;
        Ok(slot / self.active_phase_length)
    }

    pub fn get_latest_register_epoch_progress(&self, slot: u64) -> Result<u64> {
        Ok(slot
            .checked_sub(self.genesis_slot)
            .ok_or(RegistryError::ArithmeticUnderflow)?
            % self.active_phase_length)
    }

    pub fn get_current_active_epoch_progress(&self, slot: u64) -> u64 {
        slot.checked_sub(self.genesis_slot + self.registration_phase_length)
            .map(|s| s % self.active_phase_length)
            .unwrap_or(0)
    }

    /// In the last part of the active phase the registration phase starts.
    /// Returns end slot of the registration phase/start slot of the next active phase.
    pub fn is_registration_phase(&self, slot: u64) -> Result<u64> {
        let latest_register_epoch = self.get_latest_register_epoch(slot)?;
        let latest_register_epoch_progress = self.get_latest_register_epoch_progress(slot)?;
        if latest_register_epoch_progress >= self.registration_phase_length {
            return err!(RegistryError::NotInRegistrationPeriod);
        }
        Ok((latest_register_epoch) * self.active_phase_length
            + self.genesis_slot
            + self.registration_phase_length)
    }

    pub fn is_active_phase(&self, slot: u64, epoch: u64) -> Result<()> {
        if self.get_current_active_epoch(slot)? != epoch {
            return err!(RegistryError::NotInActivePhase);
        }
        Ok(())
    }

    pub fn is_report_work_phase(&self, slot: u64, epoch: u64) -> Result<()> {
        self.is_active_phase(slot, epoch + 1)?;
        let current_epoch_progress = self.get_current_active_epoch_progress(slot);
        if current_epoch_progress >= self.report_work_phase_length {
            return err!(RegistryError::NotInReportWorkPhase);
        }
        Ok(())
    }

    pub fn is_post_epoch(&self, slot: u64, epoch: u64) -> Result<()> {
        if self.get_current_active_epoch(slot)? <= epoch {
            return err!(RegistryError::InvalidEpoch);
        }
        Ok(())
    }
}
