use crate::constants::{EPOCH_SEED, FORESTER_EPOCH_SEED};
use crate::errors::RegistryError;
use crate::forester::state::{ForesterAccount, ForesterConfig};
use crate::protocol_config::state::{ProtocolConfig, ProtocolConfigPda};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

//TODO: add mechanism to fund epoch account creation
/// Is used for tallying and rewards calculation
#[account]
#[aligned_sized(anchor)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EpochPda {
    pub epoch: u64,
    pub protocol_config: ProtocolConfig,
    pub total_work: u64,
    pub registered_stake: u64,
    pub claimed_stake: u64,
}

#[aligned_sized(anchor)]
#[account]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ForesterEpochPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub epoch: u64,
    pub stake_weight: u64,
    pub work_counter: u64,
    /// Work can be reported in an extra round to earn extra performance based
    /// rewards.
    pub has_reported_work: bool,
    /// Start index of the range that determines when the forester is eligible to perform work.
    /// End index is forester_start_index + stake_weight
    pub forester_index: u64,
    pub epoch_active_phase_start_slot: u64,
    /// Total epoch state weight is registered stake of the epoch account after
    /// registration is concluded and active epoch period starts.
    pub total_epoch_state_weight: Option<u64>,
    pub protocol_config: ProtocolConfig,
    /// Incremented every time finalize registration is called.
    pub finalize_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForesterSlot {
    pub slot: u64,
    pub start_solana_slot: u64,
    pub end_solana_slot: u64,
    pub forester_index: u64,
}

impl ForesterEpochPda {
    pub fn get_current_slot(&self, current_slot: u64) -> Result<u64> {
        if current_slot
            >= self.epoch_active_phase_start_slot + self.protocol_config.active_phase_length
        {
            return err!(RegistryError::EpochEnded);
        }

        let epoch_progres = match current_slot.checked_sub(self.epoch_active_phase_start_slot) {
            Some(epoch_progres) => epoch_progres,
            None => return err!(RegistryError::EpochEnded),
        };
        Ok(epoch_progres / self.protocol_config.slot_length)
    }

    pub fn get_eligible_forester_index(
        current_light_slot: u64,
        pubkey: &Pubkey,
        total_epoch_state_weight: u64,
    ) -> Result<u64> {
        // Domain separation using the pubkey and current_light_slot.
        let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
        hasher.hashv(&[
            pubkey.to_bytes().as_slice(),
            &current_light_slot.to_le_bytes(),
        ]);
        let hash_value = u64::from_be_bytes(hasher.result().to_bytes()[0..8].try_into().unwrap());
        let forester_slot = hash_value % total_epoch_state_weight;
        Ok(forester_slot)
    }

    pub fn is_eligible(&self, forester_slot: u64) -> bool {
        forester_slot >= self.forester_index
            && forester_slot < self.forester_index + self.stake_weight
    }

    /// Check forester account is:
    /// - of correct epoch
    /// - eligible to perform work in the current slot
    pub fn check_eligibility(&self, current_slot: u64, pubkey: &Pubkey) -> Result<()> {
        self.protocol_config
            .is_active_phase(current_slot, self.epoch)?;
        let current_light_slot = self.get_current_slot(current_slot)?;
        let forester_slot = Self::get_eligible_forester_index(
            current_light_slot,
            pubkey,
            self.total_epoch_state_weight.unwrap(),
        )?;
        if self.is_eligible(forester_slot) {
            Ok(())
        } else {
            err!(RegistryError::ForresterNotEligible)
        }
    }

    /// Checks forester:
    /// - signer
    /// - eligibility
    /// - increments work counter.
    pub fn check_forester(
        forester_epoch_pda: &mut ForesterEpochPda,
        authority: &Pubkey,
        queue_pubkey: &Pubkey,
        current_solana_slot: u64,
    ) -> Result<()> {
        if forester_epoch_pda.authority != *authority {
            #[cfg(target_os = "solana")]
            msg!(
                "Invalid forester: forester_epoch_pda authority {} != provided {}",
                forester_epoch_pda.authority,
                authority
            );
            return err!(RegistryError::InvalidForester);
        }
        forester_epoch_pda.check_eligibility(current_solana_slot, queue_pubkey)?;
        forester_epoch_pda.work_counter += 1;
        Ok(())
    }

    pub fn check_forester_in_program(
        forester_epoch_pda: &mut ForesterEpochPda,
        authority: &Pubkey,
        queue_pubkey: &Pubkey,
    ) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
        Self::check_forester(
            forester_epoch_pda,
            authority,
            queue_pubkey,
            current_solana_slot,
        )
    }
}

/// This instruction needs to be executed once once the active period starts.
pub fn set_total_registered_stake_instruction(
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &EpochPda,
) {
    forester_epoch_pda.total_epoch_state_weight = Some(epoch_pda.registered_stake);
}

#[derive(Accounts)]
pub struct UpdateForesterEpochPda<'info> {
    #[account(address = forester_epoch_pda.authority)]
    pub signer: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
}

#[derive(Accounts)]
#[instruction(current_epoch: u64)]
pub struct RegisterForesterEpoch<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub protocol_config: Account<'info, ProtocolConfigPda>,
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterAccount>,
    /// CHECK:
    #[account(init, seeds = [FORESTER_EPOCH_SEED, forester_pda.key().to_bytes().as_slice(), current_epoch.to_le_bytes().as_slice()], bump, space =ForesterEpochPda::LEN , payer = authority)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    /// CHECK: TODO: check that this is the correct epoch account
    #[account(init_if_needed, seeds = [EPOCH_SEED, current_epoch.to_le_bytes().as_slice()], bump, space =EpochPda::LEN, payer = authority)]
    pub epoch_pda: Account<'info, EpochPda>,
    system_program: Program<'info, System>,
}

/// Register Forester for epoch:
/// 1. initialize epoch account if not initialized
/// 2. check that forester has enough stake
/// 3. check that forester has not already registered for the epoch
/// 4. check that we are in the registration period
/// 5. sync pending stake to active stake if stake hasn't been synced yet
/// 6. Initialize forester epoch account.
/// 7. Add forester active stake to epoch registered stake.
///
/// Epoch account:
/// - should only be created in epoch registration period
/// - should only be created once
/// - contains the protocol config to set the protocol config for that epoch
///   (changes to protocol config take effect with next epoch)
/// - collectes the active stake of registered foresters
///
/// Forester Epoch Account:
/// - should only be created in epoch registration period
/// - should only be created once per epoch per forester
#[inline(never)]
pub fn register_for_epoch_instruction(
    authority: &Pubkey,
    forester_pda: &mut ForesterAccount,
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &mut EpochPda,
    current_slot: u64,
) -> Result<()> {
    // Check whether we are in a epoch registration phase and which epoch we are in
    let current_epoch_start_slot = epoch_pda
        .protocol_config
        .is_registration_phase(current_slot, epoch_pda.epoch)?;
    // Sync pending stake to active stake if stake hasn't been synced yet.
    forester_pda.sync(current_slot, &epoch_pda.protocol_config)?;
    msg!("epoch_pda.protocol config: {:?}", epoch_pda.protocol_config);
    if forester_pda.active_stake_weight < epoch_pda.protocol_config.min_stake {
        return err!(RegistryError::StakeInsuffient);
    }
    if forester_pda.last_registered_epoch == epoch_pda.epoch && epoch_pda.epoch != 0 {
        msg!(
            "forester_pda.last_registered_epoch: {}",
            forester_pda.last_registered_epoch
        );
        msg!("epoch_pda.epoch: {}", epoch_pda.epoch);
        // With onchain implementation this error will not be necessary for the pda will be derived from the epoch
        // from the forester pubkey.
        return err!(RegistryError::ForesterAlreadyRegistered);
    }
    msg!("epoch_pda.epoch: {}", epoch_pda.epoch);

    forester_pda.last_registered_epoch = epoch_pda.epoch;
    // Add forester active stake to epoch registered stake.
    // Initialize forester epoch account.
    let initialized_forester_epoch_pda = ForesterEpochPda {
        authority: *authority,
        config: forester_pda.config,
        epoch: epoch_pda.epoch,
        stake_weight: forester_pda.active_stake_weight,
        work_counter: 0,
        has_reported_work: false,
        epoch_active_phase_start_slot: current_epoch_start_slot,
        forester_index: epoch_pda.registered_stake,
        total_epoch_state_weight: None,
        protocol_config: epoch_pda.protocol_config,
        finalize_counter: 0,
    };
    forester_epoch_pda.clone_from(&initialized_forester_epoch_pda);
    epoch_pda.registered_stake += forester_pda.active_stake_weight;

    Ok(())
}

#[cfg(test)]
mod test {
    use solana_sdk::signature::{Keypair, Signer};
    use std::collections::HashMap;

    use super::*;

    fn setup_forester_epoch_pda(
        forester_start_range: u64,
        forester_stake_weight: u64,
        active_phase_length: u64,
        slot_length: u64,
        epoch_active_phase_start_slot: u64,
        total_epoch_state_weight: u64,
    ) -> ForesterEpochPda {
        ForesterEpochPda {
            authority: Pubkey::new_unique(),
            config: ForesterConfig::default(),
            epoch: 0,
            stake_weight: forester_stake_weight,
            work_counter: 0,
            has_reported_work: false,
            forester_index: forester_start_range,
            epoch_active_phase_start_slot,
            total_epoch_state_weight: Some(total_epoch_state_weight),
            finalize_counter: 0,
            protocol_config: ProtocolConfig {
                genesis_slot: 0,
                registration_phase_length: 1,
                active_phase_length,
                report_work_phase_length: 2,
                epoch_reward: 100_000,
                base_reward: 50_000,
                min_stake: 0,
                slot_length,
                mint: Pubkey::new_unique(),
                forester_registration_guarded: false,
            },
        }
    }

    // Instead of index I use stake weight to get the period
    #[test]
    fn test_eligibility_check_within_epoch() {
        let mut eligible = HashMap::<u8, (u64, u64)>::new();
        let slot_length = 20;
        let num_foresters = 5;
        let epoch_active_phase_start_slot = 10;
        let epoch_len = 2000;
        let queue_pubkey = Keypair::new().pubkey();
        let mut total_stake_weight = 0;
        for forester_index in 0..num_foresters {
            let forester_stake_weight = 10_000 * (forester_index + 1);
            total_stake_weight += forester_stake_weight;
        }
        let mut current_total_stake_weight = 0;
        for forester_index in 0..num_foresters {
            let forester_stake_weight = 10_000 * (forester_index + 1);
            let account = setup_forester_epoch_pda(
                current_total_stake_weight,
                forester_stake_weight,
                epoch_len,
                slot_length,
                epoch_active_phase_start_slot,
                total_stake_weight,
            );
            current_total_stake_weight += forester_stake_weight;

            // Check eligibility within and outside the epoch
            for i in 0..epoch_len {
                let index = account.check_eligibility(i, &queue_pubkey);
                if index.is_ok() {
                    match eligible.get_mut(&(forester_index as u8)) {
                        Some((_, count)) => {
                            *count += 1;
                        }
                        None => {
                            eligible.insert(forester_index as u8, (forester_stake_weight, 1));
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

    // TODO: add randomized test
    #[test]
    fn test_onchain_epoch() {
        let registration_phase_length = 1;
        let active_phase_length = 7;
        let report_work_phase_length = 2;
        let protocol_config = ProtocolConfig {
            genesis_slot: 20,
            registration_phase_length,
            active_phase_length,
            report_work_phase_length,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
            forester_registration_guarded: false,
        };

        // Diagram of epochs 0 and 1.
        // Registration 0 starts at genesis slot.
        // |---- Registration 0 ----|------------------ Active 0 ------|---- Report Work 0 ----|---- Post 0 ----
        //                                        |-- Registration 1 --|------------------ Active 1 -----------------

        let mut current_solana_slot = protocol_config.genesis_slot;
        for epoch in 0..1000 {
            if epoch == 0 {
                for _ in 0..protocol_config.registration_phase_length {
                    assert!(protocol_config
                        .is_registration_phase(current_solana_slot, epoch)
                        .is_ok());

                    assert!(protocol_config
                        .is_active_phase(current_solana_slot, epoch)
                        .is_err());
                    assert!(protocol_config
                        .is_post_epoch(current_solana_slot, epoch)
                        .is_err());

                    assert!(protocol_config
                        .is_report_work_phase(current_solana_slot, epoch)
                        .is_err());

                    current_solana_slot += 1;
                }
            }

            for i in 0..protocol_config.active_phase_length {
                assert!(protocol_config
                    .is_active_phase(current_solana_slot, epoch)
                    .is_ok());
                if protocol_config.active_phase_length.saturating_sub(i)
                    <= protocol_config.registration_phase_length
                {
                    assert!(protocol_config
                        .is_registration_phase(current_solana_slot, epoch + 1)
                        .is_ok());
                } else {
                    assert!(protocol_config
                        .is_registration_phase(current_solana_slot, epoch)
                        .is_err());
                }
                if epoch == 0 {
                    assert!(protocol_config
                        .is_post_epoch(current_solana_slot, epoch)
                        .is_err());
                } else {
                    assert!(protocol_config
                        .is_post_epoch(current_solana_slot, epoch - 1)
                        .is_ok());
                }
                if epoch == 0 {
                    assert!(protocol_config
                        .is_report_work_phase(current_solana_slot, epoch)
                        .is_err());
                } else if i < protocol_config.report_work_phase_length {
                    assert!(protocol_config
                        .is_report_work_phase(current_solana_slot, epoch - 1)
                        .is_ok());
                } else {
                    assert!(protocol_config
                        .is_report_work_phase(current_solana_slot, epoch - 1)
                        .is_err());
                }
                assert!(protocol_config
                    .is_report_work_phase(current_solana_slot, epoch)
                    .is_err());
                current_solana_slot += 1;
            }
        }
    }
}
