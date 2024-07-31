use crate::errors::RegistryError;
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

#[aligned_sized(anchor)]
#[derive(Debug)]
#[account]
pub struct ProtocolConfigPda {
    pub authority: Pubkey,
    pub bump: u8,
    pub config: ProtocolConfig,
}

// TODO: replace epoch reward with inflation curve.
/// Epoch Phases:
/// 1. Registration
/// 2. Active
/// 3. Report Work
/// 4. Post (Epoch has ended, and rewards can be claimed.)
/// - There is always an active phase in progress, registration and report work
///   phases run in parallel to a currently active phase.
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct ProtocolConfig {
    /// Solana slot when the protocol starts operating.
    pub genesis_slot: u64,
    /// Total rewards per epoch.
    pub epoch_reward: u64,
    /// Base reward for foresters, the difference between epoch reward and base
    /// reward distributed based on performance.
    pub base_reward: u64,
    /// Minimum stake required for a forester to register to an epoch.
    pub min_stake: u64,
    /// Light protocol slot length. (Naming is confusing for Solana slot.)
    /// TODO: rename to epoch_length (registration + active phase length)
    pub slot_length: u64,
    /// Foresters can register for this phase.
    pub registration_phase_length: u64,
    /// Foresters can perform work in this phase.
    pub active_phase_length: u64,
    /// Foresters can report work to receive performance based rewards in this
    /// phase.
    /// TODO: enforce report work == registration phase length so that
    /// epoch in report work phase is registration epoch - 1
    pub report_work_phase_length: u64,
    pub mint: Pubkey,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            genesis_slot: 0,
            epoch_reward: 0,
            base_reward: 0,
            min_stake: 0,
            slot_length: 10,
            registration_phase_length: 100,
            active_phase_length: 1000,
            report_work_phase_length: 100,
            mint: Pubkey::default(),
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

impl ProtocolConfig {
    /// Current epoch including registration phase Only use to get registration
    /// phase.
    pub fn get_current_epoch(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) / self.active_phase_length
    }
    pub fn get_current_active_epoch(&self, slot: u64) -> Result<u64> {
        // msg!("slot: {}", slot);
        // msg!("genesis_slot: {}", self.genesis_slot);
        // msg!(
        //     "registration_phase_length: {}",
        //     self.registration_phase_length
        // );
        let slot = match slot.checked_sub(self.genesis_slot + self.registration_phase_length) {
            Some(slot) => slot,
            None => return err!(RegistryError::EpochEnded),
        };
        Ok(slot / self.active_phase_length)
    }

    pub fn get_current_epoch_progress(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) % self.active_phase_length
    }

    pub fn get_current_active_epoch_progress(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot + self.registration_phase_length))
            % self.active_phase_length
    }

    /// In the last part of the active phase the registration phase starts.
    pub fn is_registration_phase(&self, slot: u64) -> Result<u64> {
        let current_epoch = self.get_current_epoch(slot);
        let current_epoch_progress = self.get_current_epoch_progress(slot);
        if current_epoch_progress >= self.registration_phase_length {
            return err!(RegistryError::NotInRegistrationPeriod);
        }
        Ok((current_epoch) * self.active_phase_length
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
        if self.get_current_active_epoch(slot)? == epoch {
            return err!(RegistryError::InvalidEpoch);
        }
        Ok(())
    }

    /// Rewards:
    /// 1. foresters should be incentivezed to performe work
    /// 2. foresters contention is mitigated by giving time slots
    /// 2.1 foresters with more stake receive more timeslots to perform work
    /// 3. rewards
    /// 3.1 base rewards 50% of the total rewards - distributed based on relative stake
    /// 3.2 remainging 50% relative amount of work performed
    pub fn get_rewards(
        &self,
        total_stake_weight: u64,
        total_tally: u64,
        forester_stake_weight: u64,
        forester_tally: u64,
    ) -> u64 {
        let total_merit_reward = self.epoch_reward - self.base_reward;
        let merit_reward = total_merit_reward * forester_tally / total_tally;
        let stake_reward = self.base_reward * forester_stake_weight / total_stake_weight;
        merit_reward + stake_reward
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zero_values() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };

        assert_eq!(protocol_config.get_rewards(100_000, 20_000, 0, 0), 0);
        assert_eq!(
            protocol_config.get_rewards(100_000, 20_000, 10_000, 0),
            5_000
        );
        assert_eq!(
            protocol_config.get_rewards(100_000, 20_000, 0, 10_000),
            25_000
        );
    }

    #[test]
    fn test_equal_stake_and_tally() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };

        let total_stake_weight = 100_000;
        let total_tally = 20_000;

        assert_eq!(
            protocol_config.get_rewards(total_stake_weight, total_tally, 100_000, 20_000),
            100_000
        );
    }

    #[test]
    fn test_single_forester() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };

        let total_stake_weight = 10_000;
        let total_tally = 10_000;
        let forester_stake_weight = 10_000;
        let forester_tally = 10_000;

        let reward = protocol_config.get_rewards(
            total_stake_weight,
            total_tally,
            forester_stake_weight,
            forester_tally,
        );
        let expected_reward = 100_000;
        assert_eq!(reward, expected_reward);
    }

    #[test]
    fn test_proportional_distribution() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };

        let total_stake_weight = 100_000;
        let total_tally = 20_000;

        let forester_stake_weight = 10_000;
        let forester_tally = 5_000;

        let reward = protocol_config.get_rewards(
            total_stake_weight,
            total_tally,
            forester_stake_weight,
            forester_tally,
        );
        let expected_reward = 17_500;
        assert_eq!(reward, expected_reward);
    }

    #[test]
    fn reward_calculation() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };
        let total_stake_weight = 100_000;
        let total_tally = 20_000;
        {
            let forester_stake_weight = 10_000;
            let forester_tally = 10_000;

            let reward = protocol_config.get_rewards(
                total_stake_weight,
                total_tally,
                forester_stake_weight,
                forester_tally,
            );
            assert_eq!(reward, 30_000);
            let forester_stake_weight = 90_000;
            let forester_tally = 10_000;

            let reward = protocol_config.get_rewards(
                total_stake_weight,
                total_tally,
                forester_stake_weight,
                forester_tally,
            );
            assert_eq!(reward, 70_000);
        }
        // Forester performs max and receives max
        {
            let forester_stake_weight = 20_000;
            let forester_tally = 10_000;

            let reward = protocol_config.get_rewards(
                total_stake_weight,
                total_tally,
                forester_stake_weight,
                forester_tally,
            );
            assert_eq!(reward, 35_000);
            let forester_stake_weight = 80_000;
            let forester_tally = 10_000;

            let reward = protocol_config.get_rewards(
                total_stake_weight,
                total_tally,
                forester_stake_weight,
                forester_tally,
            );
            assert_eq!(reward, 65_000);
        }
        // forester performs less -> receives less
        {
            let forester_stake_weight = 20_000;
            let forester_tally = 0;

            let reward = protocol_config.get_rewards(
                total_stake_weight,
                total_tally,
                forester_stake_weight,
                forester_tally,
            );
            assert_eq!(reward, 10_000);
        }
    }
}
