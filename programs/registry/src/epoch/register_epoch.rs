use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    constants::FORESTER_EPOCH_SEED,
    errors::RegistryError,
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    selection::forester::{ForesterConfig, ForesterPda},
};

/// Is used for tallying and rewards calculation
#[account]
#[aligned_sized(anchor)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EpochPda {
    pub epoch: u64,
    pub protocol_config: ProtocolConfig,
    pub total_work: u64,
    pub registered_weight: u64,
}

#[aligned_sized(anchor)]
#[account]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ForesterEpochPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub epoch: u64,
    pub weight: u64,
    pub work_counter: u64,
    /// Work can be reported in an extra round to earn extra performance based
    /// rewards.
    pub has_reported_work: bool,
    /// Start index of the range that determines when the forester is eligible to perform work.
    /// End index is forester_start_index + weight
    pub forester_index: u64,
    pub epoch_active_phase_start_slot: u64,
    /// Total epoch weight is registered weight of the epoch account after
    /// registration is concluded and active epoch period starts.
    pub total_epoch_weight: Option<u64>,
    pub protocol_config: ProtocolConfig,
    /// Incremented every time finalize registration is called.
    pub finalize_counter: u64,
}

impl ForesterEpochPda {
    pub fn get_current_light_slot(&self, current_solana_slot: u64) -> Result<u64> {
        let epoch_progress =
            match current_solana_slot.checked_sub(self.epoch_active_phase_start_slot) {
                Some(epoch_progress) => epoch_progress,
                None => return err!(RegistryError::EpochEnded),
            };
        Ok(epoch_progress / self.protocol_config.slot_length)
    }

    /// Returns the forester index for the current slot. The forester whose
    /// weighted range [total_registered_weight_at_registration,
    /// total_registered_weight_at_registration + forester_weight] contains the
    /// forester index is eligible to perform work. If a forester has more
    /// weight the range is larger -> the forester is eligible for more slots.
    /// The forester index is a random number, derived from queue pubkey, epoch,
    /// and current light slot, between 0 and total_epoch_weight.
    pub fn get_eligible_forester_index(
        current_light_slot: u64,
        pubkey: &Pubkey,
        total_epoch_weight: u64,
        epoch: u64,
    ) -> Result<u64> {
        // Domain separation using the pubkey and current_light_slot.
        let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
        hasher.hashv(&[
            pubkey.to_bytes().as_slice(),
            &epoch.to_be_bytes(),
            &current_light_slot.to_be_bytes(),
        ]);
        let hash_value = u64::from_be_bytes(hasher.result().to_bytes()[0..8].try_into().unwrap());
        let forester_index = hash_value % total_epoch_weight;
        Ok(forester_index)
    }

    pub fn is_eligible(&self, forester_range_start: u64) -> bool {
        forester_range_start >= self.forester_index
            && forester_range_start < self.forester_index + self.weight
    }

    /// Check forester account is:
    /// - of correct epoch
    /// - eligible to perform work in the current slot
    pub fn check_eligibility(&self, current_slot: u64, pubkey: &Pubkey) -> Result<()> {
        self.protocol_config
            .is_active_phase(current_slot, self.epoch)?;
        let current_light_slot = self.get_current_light_slot(current_slot)?;
        let total_epoch_weight = self
            .total_epoch_weight
            .ok_or(RegistryError::RegistrationNotFinalized)?;
        let forester_slot = Self::get_eligible_forester_index(
            current_light_slot,
            pubkey,
            total_epoch_weight,
            self.epoch,
        )?;
        if self.is_eligible(forester_slot) {
            Ok(())
        } else {
            err!(RegistryError::ForesterNotEligible)
        }
    }

    /// Checks forester:
    /// - signer
    /// - eligibility
    /// - increments work counter
    pub fn check_forester(
        forester_epoch_pda: &mut ForesterEpochPda,
        authority: &Pubkey,
        queue_pubkey: &Pubkey,
        current_solana_slot: u64,
        num_work_items: u64,
    ) -> Result<()> {
        if forester_epoch_pda.authority != *authority {
            msg!(
                "Invalid forester: forester_epoch_pda authority {} != provided {}",
                forester_epoch_pda.authority,
                authority
            );
            return err!(RegistryError::InvalidForester);
        }
        forester_epoch_pda.check_eligibility(current_solana_slot, queue_pubkey)?;
        forester_epoch_pda.work_counter += num_work_items;
        Ok(())
    }

    pub fn check_forester_in_program(
        forester_epoch_pda: &mut ForesterEpochPda,
        authority: &Pubkey,
        queue_pubkey: &Pubkey,
        num_work_items: u64,
    ) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
        Self::check_forester(
            forester_epoch_pda,
            authority,
            queue_pubkey,
            current_solana_slot,
            num_work_items,
        )
    }
}

/// This instruction needs to be executed once the active period starts.
pub fn set_total_registered_weight_instruction(
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &EpochPda,
) {
    forester_epoch_pda.total_epoch_weight = Some(epoch_pda.registered_weight);
}

#[derive(Accounts)]
#[instruction(current_epoch: u64)]
pub struct RegisterForesterEpoch<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(has_one = authority)]
    pub forester_pda: Account<'info, ForesterPda>,
    /// Instruction checks that current_epoch is the current epoch and that
    /// the epoch is in registration phase.
    #[account(init, seeds = [FORESTER_EPOCH_SEED, forester_pda.key().to_bytes().as_slice(), current_epoch.to_le_bytes().as_slice()], bump, space =ForesterEpochPda::LEN , payer = fee_payer)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    pub protocol_config: Account<'info, ProtocolConfigPda>,
    #[account(init_if_needed, seeds = [current_epoch.to_le_bytes().as_slice()], bump, space =EpochPda::LEN, payer = fee_payer)]
    pub epoch_pda: Account<'info, EpochPda>,
    system_program: Program<'info, System>,
}

/// Register Forester for epoch:
/// 1. initialize epoch account if not initialized
/// 2. check that forester has enough weight
/// 3. check that forester has not already registered for the epoch
/// 4. check that we are in the registration period
/// 5. sync pending weight to active weight if weight hasn't been synced yet
/// 6. Initialize forester epoch account.
/// 7. Add forester active weight to epoch registered weight.
///
/// Epoch account:
/// - should only be created in epoch registration period
/// - should only be created once
/// - contains the protocol config to set the protocol config for that epoch
///   (changes to protocol config take effect with next epoch)
/// - collects the active weight of registered foresters
///
/// Forester Epoch Account:
/// - should only be created in epoch registration period
/// - should only be created once per epoch per forester
#[inline(never)]
pub fn process_register_for_epoch(
    authority: &Pubkey,
    forester_pda: &mut ForesterPda,
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &mut EpochPda,
    current_slot: u64,
) -> Result<()> {
    if forester_pda.active_weight < epoch_pda.protocol_config.min_weight {
        return err!(RegistryError::WeightInsuffient);
    }

    // Check whether we are in an epoch registration phase and which epoch we are in
    let current_epoch_start_slot = epoch_pda
        .protocol_config
        .is_registration_phase(current_slot)?;

    forester_pda.last_registered_epoch = epoch_pda.epoch;

    let initialized_forester_epoch_pda = ForesterEpochPda {
        authority: *authority,
        config: forester_pda.config,
        epoch: epoch_pda.epoch,
        weight: forester_pda.active_weight,
        work_counter: 0,
        has_reported_work: false,
        epoch_active_phase_start_slot: current_epoch_start_slot,
        forester_index: epoch_pda.registered_weight,
        total_epoch_weight: None,
        protocol_config: epoch_pda.protocol_config,
        finalize_counter: 0,
    };
    forester_epoch_pda.clone_from(&initialized_forester_epoch_pda);
    epoch_pda.registered_weight += forester_pda.active_weight;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use solana_sdk::signature::{Keypair, Signer};

    use super::*;

    fn setup_forester_epoch_pda(
        forester_start_range: u64,
        forester_weight: u64,
        active_phase_length: u64,
        slot_length: u64,
        epoch_active_phase_start_slot: u64,
        total_epoch_weight: u64,
    ) -> ForesterEpochPda {
        ForesterEpochPda {
            authority: Pubkey::new_unique(),
            config: ForesterConfig::default(),
            epoch: 0,
            weight: forester_weight,
            work_counter: 0,
            has_reported_work: false,
            forester_index: forester_start_range,
            epoch_active_phase_start_slot,
            total_epoch_weight: Some(total_epoch_weight),
            finalize_counter: 0,
            protocol_config: ProtocolConfig {
                genesis_slot: 0,
                registration_phase_length: 1,
                active_phase_length,
                report_work_phase_length: 2,
                min_weight: 0,
                slot_length,
                network_fee: 5000,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_eligibility_check_within_epoch() {
        let mut eligible = HashMap::<u8, (u64, u64)>::new();
        let slot_length = 20;
        let num_foresters = 5;
        let epoch_active_phase_start_slot = 10;
        let epoch_len = 2000;
        let queue_pubkey = Keypair::new().pubkey();
        let mut total_weight = 0;
        for forester_index in 0..num_foresters {
            let forester_weight = 10_000 * (forester_index + 1);
            total_weight += forester_weight;
        }
        let mut current_total_weight = 0;
        for forester_index in 0..num_foresters {
            let forester_weight = 10_000 * (forester_index + 1);
            let account = setup_forester_epoch_pda(
                current_total_weight,
                forester_weight,
                epoch_len,
                slot_length,
                epoch_active_phase_start_slot,
                total_weight,
            );
            current_total_weight += forester_weight;

            // Check eligibility within and outside the epoch
            for i in 0..epoch_len {
                let index = account.check_eligibility(i, &queue_pubkey);
                if index.is_ok() {
                    match eligible.get_mut(&(forester_index as u8)) {
                        Some((_, count)) => {
                            *count += 1;
                        }
                        None => {
                            eligible.insert(forester_index as u8, (forester_weight, 1));
                        }
                    };
                }
            }
        }
        println!("stats --------------------------------");
        for (forester_index, num_eligible_slots) in eligible.iter() {
            println!("forester_index = {:?}", forester_index);
            println!("num_eligible_slots = {:?}", num_eligible_slots);
        }

        let sum = eligible.values().map(|x| x.1).sum::<u64>();
        let total_slots: u64 = epoch_len - epoch_active_phase_start_slot;
        assert_eq!(sum, total_slots);
    }

    #[test]
    fn test_epoch_phases() {
        let registration_phase_length = 1;
        let active_phase_length = 7;
        let report_work_phase_length = 2;
        let protocol_config = ProtocolConfig {
            genesis_slot: 20,
            registration_phase_length,
            active_phase_length,
            report_work_phase_length,
            min_weight: 0,
            slot_length: 1,
            network_fee: 5000,
            ..Default::default()
        };
        // Diagram of epochs 0 and 1.
        // Registration 0 starts at genesis slot.
        // |---- Registration 0 ----|------------------ Active 0 ------|---- Report Work 0 ----|---- Post 0 ----
        //                                        |-- Registration 1 --|------------------ Active 1 -----------------

        let mut current_slot = protocol_config.genesis_slot;
        for epoch in 0..1000 {
            if epoch == 0 {
                for _ in 0..protocol_config.registration_phase_length {
                    assert!(protocol_config.is_registration_phase(current_slot).is_ok());

                    assert!(protocol_config
                        .is_active_phase(current_slot, epoch)
                        .is_err());
                    assert!(protocol_config.is_post_epoch(current_slot, epoch).is_err());

                    assert!(protocol_config
                        .is_report_work_phase(current_slot, epoch)
                        .is_err());

                    current_slot += 1;
                }
            }

            for i in 0..protocol_config.active_phase_length {
                assert!(protocol_config.is_active_phase(current_slot, epoch).is_ok());
                if protocol_config.active_phase_length.saturating_sub(i)
                    <= protocol_config.registration_phase_length
                {
                    assert!(protocol_config.is_registration_phase(current_slot).is_ok());
                } else {
                    assert!(protocol_config.is_registration_phase(current_slot).is_err());
                }
                if epoch == 0 {
                    assert!(protocol_config.is_post_epoch(current_slot, epoch).is_err());
                } else {
                    assert!(protocol_config
                        .is_post_epoch(current_slot, epoch - 1)
                        .is_ok());
                }
                if epoch == 0 {
                    assert!(protocol_config
                        .is_report_work_phase(current_slot, epoch)
                        .is_err());
                } else if i < protocol_config.report_work_phase_length {
                    assert!(protocol_config
                        .is_report_work_phase(current_slot, epoch - 1)
                        .is_ok());
                } else {
                    assert!(protocol_config
                        .is_report_work_phase(current_slot, epoch - 1)
                        .is_err());
                }
                assert!(protocol_config
                    .is_report_work_phase(current_slot, epoch)
                    .is_err());
                current_slot += 1;
            }
        }
    }
}
