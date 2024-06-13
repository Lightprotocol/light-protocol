use crate::errors::RegistryError;
use crate::protocol_config::state::ProtocolConfig;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_be;

/// Simulate work and slots for testing purposes.
/// 1. Check eligibility of forester
pub fn simulate_work_and_slots_instruction(
    forester_epoch_pda: &mut ForesterEpochPda,
    num_work: u64,
    queue_pubkey: &Pubkey,
    current_slot: &u64,
) -> Result<()> {
    forester_epoch_pda
        .protocol_config
        .is_active_phase(*current_slot, forester_epoch_pda.epoch)?;
    forester_epoch_pda.check_eligibility(*current_slot, queue_pubkey)?;
    forester_epoch_pda.work_counter += num_work;
    Ok(())
}

pub struct MockCompressedTokenAccount {
    pub balance: u64,
}

pub struct MockSplTokenAccount {
    pub balance: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{signature::Keypair, signer::Signer};
    use std::collections::HashMap;

    /// Scenario:
    /// 1. Protocol config setup
    /// 2. User stakes 1000 tokens
    /// 3. Forester creates a stake account
    /// 4. User delegates 1000 tokens to forester
    /// 5. Forester registers for epoch
    /// 6. Forester performs some actions
    /// 7. Tally results
    /// 8. Forester withdraws rewards
    /// 9. User syncs stake account
    #[test]
    fn staking_scenario() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 20,
            registration_phase_length: 1,
            active_phase_length: 7,
            report_work_phase_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
            mint: Pubkey::new_unique(),
        };
        let mut current_solana_slot = protocol_config.genesis_slot;

        // ----------------------------------------------------------------------------------------
        // 2. User stakes 1000 tokens
        // - token transfer from compressed token account to compressed token staking account
        //   - the compressed token staking account stores all staked tokens
        // - staked tokens are not automatically delegated
        // - user_delegate_account stake_weight is increased by the added stake amount
        let mut user_delegate_account = DelegateAccount::default();
        let user_token_balance = 1000;
        let mut user_stake_token_account = MockCompressedTokenAccount { balance: 0 };
        let mut user_token_account = MockCompressedTokenAccount { balance: 1000 };
        stake_instruction(
            &mut user_delegate_account,
            user_token_balance,
            &mut user_token_account,
            &mut user_stake_token_account,
        )
        .unwrap();

        // ----------------------------------------------------------------------------------------
        // 3. Forester creates a stake and token accounts
        // - forester_token_pool_account is intermediate storage for staking rewards that have not been synced to the user token staking accounts yet
        //   - is owned by a pda derived from the forester pubkey
        // - forester_fee_token_account recipient for forester fees
        // - forester_pda
        let forester_pubkey = Pubkey::new_unique();
        let forester_delegate_account_pubkey = Pubkey::new_unique();
        let mut forester_pda = ForesterAccount {
            forester_config: ForesterConfig {
                forester_pubkey,
                fee: 10,
            },
            ..ForesterAccount::default()
        };
        // Forester fee rewards go to this compressed token account.
        let mut forester_fee_token_account = MockCompressedTokenAccount { balance: 0 };
        // Is an spl token account since it will need to be accessed by many parties.
        // Compressed token accounts are not well suited to be pool accounts.
        let mut forester_token_pool_account = MockSplTokenAccount { balance: 0 };

        // ----------------------------------------------------------------------------------------
        // 4. User delegates 1000 tokens to forester
        // - delegated stake is not active until the next epoch
        //   - this is enforced by adding the delegated stake to forester_pda  pending stake weight
        //   - forester_pda pending stake weight is synced to active stake weight once the epoch changes
        // - delegated stake is stored in the user_delegate_account delegated_stake_weight
        delegate_instruction(
            &protocol_config,
            &mut user_delegate_account,
            &forester_delegate_account_pubkey,
            &mut forester_pda,
            user_token_balance,
            current_solana_slot,
            true,
        )
        .unwrap();
        assert_eq!(
            forester_pda.pending_undelegated_stake_weight,
            user_delegate_account.delegated_stake_weight
        );
        assert_eq!(
            forester_delegate_account_pubkey,
            user_delegate_account
                .delegate_forester_delegate_account
                .unwrap()
        );
        // ----------------------------------------------------------------------------------------
        // Registration phase starts (epoch 1)
        // Active phase starts (epoch 1)
        // (We need to start in epoch 1, because nobody can register before epoch 0)
        current_solana_slot = 20 + protocol_config.active_phase_length;
        // ----------------------------------------------------------------------------------------
        // 5. Forester registers for epoch and initializes epoch account if
        //    needed
        // - epoch account is initialized if not already initialized
        // - forester epoch account is initialized with values from forester
        //   stake account and epoch account
        let mut epoch_pda = EpochPda::default();
        let mut forester_epoch_pda = ForesterEpochPda::default();
        register_for_epoch_instruction(
            &protocol_config,
            &mut forester_pda,
            &mut forester_epoch_pda,
            &mut epoch_pda,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(forester_pda.pending_undelegated_stake_weight, 0);
        assert_eq!(forester_epoch_pda.stake_weight, user_token_balance);
        assert_eq!(epoch_pda.registered_stake, user_token_balance);
        assert_eq!(forester_epoch_pda.epoch_start_slot, 28);
        assert_eq!(forester_epoch_pda.epoch, 1);
        // ----------------------------------------------------------------------------------------
        // Registration phase ends (epoch 1)
        // Active phase starts (epoch 1)
        current_solana_slot += protocol_config.registration_phase_length;
        assert!(protocol_config
            .is_registration_phase(current_solana_slot)
            .is_err());
        // ----------------------------------------------------------------------------------------
        // 6. Forester performs some actions until epoch ends
        set_total_registered_stake_instruction(&mut forester_epoch_pda, &epoch_pda);
        simulate_work_and_slots_instruction(
            &mut forester_epoch_pda,
            protocol_config.active_phase_length - 1,
            &Pubkey::new_unique(),
            &current_solana_slot,
        )
        .unwrap();

        // ----------------------------------------------------------------------------------------
        // Active phase ends (epoch 1)
        // Active phase starts (epoch 2)
        current_solana_slot += protocol_config.active_phase_length;
        assert!(protocol_config
            .is_active_phase(current_solana_slot, 1)
            .is_err());
        assert!(protocol_config
            .is_active_phase(current_solana_slot, 2)
            .is_ok());
        assert!(protocol_config
            .is_report_work_phase(current_solana_slot, 1)
            .is_ok());
        // ----------------------------------------------------------------------------------------
        // 7. Report work from active epoch phase
        report_work_instruction(&mut forester_epoch_pda, &mut epoch_pda, current_solana_slot)
            .unwrap();

        // ----------------------------------------------------------------------------------------
        // Report work phase ends (epoch 1)
        current_solana_slot += protocol_config.report_work_phase_length;
        assert!(protocol_config
            .is_report_work_phase(current_solana_slot, 1)
            .is_err());
        assert!(protocol_config
            .is_post_epoch(current_solana_slot, 1)
            .is_ok());
        // ----------------------------------------------------------------------------------------
        // 8. Forester claim rewards (post epoch)
        let compressed_forester_epoch_pda = forester_claim_rewards_instruction(
            &mut forester_fee_token_account,
            &mut forester_token_pool_account,
            &mut forester_pda,
            &mut forester_epoch_pda,
            &mut epoch_pda,
            current_solana_slot,
        )
        .unwrap();
        let forester_fee = 10_000;
        assert_eq!(forester_fee_token_account.balance, forester_fee);
        assert_eq!(
            forester_token_pool_account.balance,
            protocol_config.epoch_reward - forester_fee
        );

        // ----------------------------------------------------------------------------------------
        // 9. User syncs stake account and syncs stake token account
        let hashed_forester_pubkey = hash_to_bn254_field_size_be(&forester_pubkey.to_bytes())
            .unwrap()
            .0;
        let compressed_forester_epoch_pda_input_account = CompressedForesterEpochAccountInput {
            rewards_earned: compressed_forester_epoch_pda.rewards_earned,
            epoch: compressed_forester_epoch_pda.epoch,
            stake_weight: compressed_forester_epoch_pda.stake_weight,
        };
        sync_delegate_account_instruction(
            &mut user_delegate_account,
            vec![compressed_forester_epoch_pda_input_account],
            hashed_forester_pubkey,
            compressed_forester_epoch_pda.previous_hash,
        )
        .unwrap();

        assert_eq!(
            user_delegate_account.delegated_stake_weight,
            user_token_balance + compressed_forester_epoch_pda.rewards_earned
        );
        assert_eq!(user_delegate_account.pending_token_amount, 90_000);

        sync_token_account_instruction(
            &mut forester_token_pool_account,
            &mut user_stake_token_account,
            &mut user_delegate_account,
        );
        assert_eq!(user_stake_token_account.balance, 1000 + 90_000);
        assert_eq!(forester_token_pool_account.balance, 0);
        assert_eq!(user_delegate_account.pending_token_amount, 0);

        // ----------------------------------------------------------------------------------------
        // 10. User undelegates and unstakes
        let mut recipient_token_account = MockCompressedTokenAccount { balance: 0 };
        let unstake_amount = 100;
        undelegate_instruction(
            &protocol_config,
            &mut user_delegate_account,
            &mut forester_pda,
            unstake_amount,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(
            user_delegate_account.pending_undelegated_stake_weight,
            unstake_amount
        );
        assert_eq!(
            forester_pda.active_stake_weight,
            900 + protocol_config.epoch_reward - forester_fee
        );

        unstake_instruction(
            &mut user_delegate_account,
            &mut user_stake_token_account,
            &mut recipient_token_account,
            protocol_config,
            unstake_amount,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(user_delegate_account.delegated_stake_weight, 90900);
        assert_eq!(user_stake_token_account.balance, 90900);
        assert_eq!(recipient_token_account.balance, unstake_amount);
    }
}
