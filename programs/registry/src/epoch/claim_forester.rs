use crate::selection::forester::ForesterAccount;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_be;

use super::register_epoch::{EpochPda, ForesterEpochPda};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedForesterEpochAccount {
    rewards_earned: u64,
    epoch: u64,
    stake_weight: u64,
    previous_hash: [u8; 32],
    forester_pubkey: Pubkey,
}

impl CompressedForesterEpochAccount {
    pub fn hash(&self, hashed_forester_pubkey: [u8; 32]) -> Result<[u8; 32]> {
        let hash = light_hasher::poseidon::Poseidon::hashv(&[
            hashed_forester_pubkey.as_slice(),
            self.previous_hash.as_slice(),
            &self.rewards_earned.to_le_bytes(),
            &self.epoch.to_le_bytes(),
            &self.stake_weight.to_le_bytes(),
        ])
        .map_err(ProgramError::from)?;
        Ok(hash)
    }

    pub fn get_reward(&self, stake: u64) -> u64 {
        self.rewards_earned * stake / self.stake_weight
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedForesterEpochAccountInput {
    pub rewards_earned: u64,
    pub epoch: u64,
    pub stake_weight: u64,
}

impl CompressedForesterEpochAccountInput {
    pub fn into_compressed_forester_epoch_pda(
        self,
        previous_hash: [u8; 32],
        forester_pubkey: Pubkey,
    ) -> CompressedForesterEpochAccount {
        CompressedForesterEpochAccount {
            rewards_earned: self.rewards_earned,
            epoch: self.epoch,
            stake_weight: self.stake_weight,
            previous_hash,
            forester_pubkey,
        }
    }
}

// TODO: implement claim logic
// /// Forester claim rewards:
// /// 1. Transfer forester fees to foresters token account
// /// 2. Transfer rewards to foresters token account
// /// 3. compress forester epoch account
// /// 4. close forester epoch account
// /// 5. if all stake has claimed close epoch account
// pub fn forester_claim_rewards_instruction(
//     forester_fee_token_account: &mut AccountInfo,
//     forester_token_pool_account: &mut AccountInfo,
//     forester_pda: &mut ForesterAccount,
//     forester_epoch_pda: &mut ForesterEpochPda,
//     epoch_pda: &mut EpochPda,
//     current_slot: u64,
// ) -> Result<()> {
//     let (compressed_account, fee, net_reward) = forester_claim_rewards(
//         forester_pda,
//         forester_epoch_pda,
//         epoch_pda,
//         current_slot,
//     )?;
//     // Transfer fees to forester
//     // TODO: create compressed account
//     Ok(())
// }

pub fn forester_claim_rewards(
    forester_pda: &mut ForesterAccount,
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &mut EpochPda,
    current_slot: u64,
) -> Result<(CompressedForesterEpochAccount, u64, u64)> {
    epoch_pda
        .protocol_config
        .is_post_epoch(current_slot, forester_epoch_pda.epoch)?;

    let total_stake_weight = epoch_pda.registered_stake;
    let total_tally = epoch_pda.total_work;
    let forester_stake_weight = forester_epoch_pda.stake_weight;
    let forester_tally = forester_epoch_pda.work_counter;
    let reward = epoch_pda.protocol_config.get_rewards(
        total_stake_weight,
        total_tally,
        forester_stake_weight,
        forester_tally,
    );
    let fee = reward * forester_pda.config.fee / 100;
    // Transfer fees to forester
    // forester_fee_token_account.balance += fee;
    let net_reward = reward - fee;
    // Transfer net_reward to forester pool account
    // Stakers can claim from this account
    // forester_token_pool_account.balance += net_reward;
    // Increase the active deleagted stake weight by the net_reward
    forester_pda.active_stake_weight += net_reward;
    let compressed_account = CompressedForesterEpochAccount {
        rewards_earned: net_reward,
        epoch: forester_epoch_pda.epoch,
        stake_weight: forester_epoch_pda.stake_weight,
        previous_hash: forester_pda.last_compressed_forester_epoch_pda_hash,
        forester_pubkey: forester_epoch_pda.authority,
    };
    let hashed_forester_pubkey = hash_to_bn254_field_size_be(&forester_pda.authority.to_bytes())
        .unwrap()
        .0;
    let compressed_account_hash = compressed_account.hash(hashed_forester_pubkey)?;
    forester_pda.last_compressed_forester_epoch_pda_hash = compressed_account_hash;
    Ok((compressed_account, fee, net_reward))
}
