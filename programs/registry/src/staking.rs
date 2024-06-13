use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_be;

#[error_code]
pub enum ErrorCode {
    TallyPeriodNotStarted,
    StakeAccountAlreadySynced,
    EpochEnded,
    ForresterNotEligible,
    NotInRegistrationPeriod,
    NotInActivationPeriod,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolConfig {
    pub genesis_slot: u64,

    pub epoch_reward: u64,
    pub base_reward: u64,
    pub min_stake: u64,
    pub slot_length: u64,
    /// Epoch length in slots
    pub epoch_length: u64,
    /// Foresters can register for this epoch.
    pub registration_period_length: u64,
    /// Must be less than epoch_length
    pub tally_period_length: u64,
}

impl ProtocolConfig {
    pub fn get_current_epoch(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) / self.epoch_length
    }

    pub fn get_current_epoch_progress(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) % self.epoch_length
    }

    pub fn is_registration_period(&self, slot: u64) -> Result<u64> {
        let current_epoch_progress = self.get_current_epoch_progress(slot);
        if current_epoch_progress > self.registration_period_length {
            return err!(ErrorCode::NotInRegistrationPeriod);
        }
        Ok(self.get_current_epoch(slot) * self.epoch_length + self.genesis_slot)
    }

    pub fn is_in_tally_period(&self, slot: u64, tally_epoch: u64) -> bool {
        let current_epoch = self.get_current_epoch(slot);
        let current_epoch_progress = self.get_current_epoch_progress(slot);
        if current_epoch != tally_epoch + 1 && self.tally_period_length < current_epoch_progress {
            return false;
        }
        true
    }

    pub fn is_post_tally_period(&self, slot: u64, tally_epoch: u64) -> bool {
        let current_epoch = self.get_current_epoch(slot);
        let current_epoch_progress = self.get_current_epoch_progress(slot);
        if current_epoch != tally_epoch + 1 && self.tally_period_length >= current_epoch_progress {
            return false;
        }
        true
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

/// Is used for tallying and rewards calculation
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EpochAccount {
    pub epoch: u64,
    pub protocol_config: ProtocolConfig,
    pub last_epoch_total_stake_weight: u64,
    pub last_epoch_tally: u64,
    pub registered_stake: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StakeAccount {
    delegate_forester_stake_account: Option<Pubkey>,
    /// Stake weight that is delegated to a forester.
    /// Newly delegated stake is not active until the next epoch.
    delegated_stake_weight: u64,
    /// undelgated stake is stake that is not yet delegated to a forester
    stake_weight: u64,
    /// When undelegating stake is pending until the next epoch
    pending_stake_weight: u64,
    pending_epoch: u64,
    last_sync_epoch: u64,
    /// Pending token amount are rewards that are not yet claimed to the stake
    /// compressed token account.
    pending_token_amount: u64,
}

impl StakeAccount {
    pub fn sync_pending_stake_weight(&mut self, current_epoch: u64) {
        if current_epoch > self.last_sync_epoch {
            self.stake_weight += self.pending_stake_weight;
            self.pending_stake_weight = 0;
            self.pending_epoch = current_epoch;
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterStakeAccount {
    forester_config: ForesterConfig,
    active_stake_weight: u64,
    pending_stake_weight: u64,
    current_epoch: u64,
    protocol_config: ProtocolConfig,
    last_compressed_forester_epoch_account_hash: [u8; 32],
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterConfig {
    pub forester_pubkey: Pubkey,
    /// Fee in percentage points
    pub fee: u64,
}

impl ForesterStakeAccount {
    /// If current epoch changed, move pending stake to active stake and update
    /// current epoch field
    pub fn sync(&mut self, current_slot: u64) {
        let current_epoch = self.protocol_config.get_current_epoch(current_slot);
        if current_epoch > self.current_epoch {
            self.current_epoch = current_epoch;
            self.active_stake_weight += self.pending_stake_weight;
            self.pending_stake_weight = 0;
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterEpochAccount {
    forester_pubkey: Pubkey,
    epoch: u64,
    stake_weight: u64,
    work_counter: u64,
    has_tallied: bool,
    forester_index: u64,
    // epoch_length: u64, // remove
    epoch_start_slot: u64,
    slot_length: u64,
    total_epoch_state_weight: Option<u64>,
    protocol_config: ProtocolConfig,
}

impl ForesterEpochAccount {
    pub fn get_current_slot(&self, current_slot: u64) -> Result<u64> {
        if current_slot >= self.epoch_start_slot + self.protocol_config.epoch_length {
            return err!(ErrorCode::EpochEnded);
        }
        if current_slot < self.epoch_start_slot + self.protocol_config.registration_period_length {
            return err!(ErrorCode::NotInActivationPeriod);
        }
        let epoch_progres = match current_slot.checked_sub(self.epoch_start_slot) {
            Some(epoch_progres) => epoch_progres,
            None => return err!(ErrorCode::EpochEnded),
        };
        Ok(epoch_progres / self.protocol_config.slot_length)
    }

    // TODO: add function that returns all light slots with start and end solana slots for a given epoch
    pub fn get_eligible_forester_index(&self, current_slot: u64, pubkey: &Pubkey) -> Result<u64> {
        let current_light_slot = self.get_current_slot(current_slot)?;
        let total_epoch_state_weight = self.total_epoch_state_weight.unwrap();

        // Domain separation using the pubkey and current_light_slot
        let mut hasher = anchor_lang::solana_program::hash::Hasher::default();
        hasher.hashv(&[
            pubkey.to_bytes().as_slice(),
            &current_light_slot.to_le_bytes(),
        ]);
        let hash_value = u64::from_be_bytes(hasher.result().to_bytes()[0..8].try_into().unwrap());
        let forester_slot = hash_value % total_epoch_state_weight;
        Ok(forester_slot)
    }

    pub fn check_eligibility(&self, current_slot: u64, pubkey: &Pubkey) -> Result<()> {
        let forester_slot = self.get_eligible_forester_index(current_slot, pubkey)?;
        if forester_slot >= self.forester_index
            && forester_slot < self.forester_index + self.stake_weight
        {
            Ok(())
        } else {
            err!(ErrorCode::ForresterNotEligible)
        }
    }
}

/// This instruction needs to be executed once once the active period starts.
pub fn set_total_registered_stake(
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &EpochAccount,
) {
    forester_epoch_account.total_epoch_state_weight = Some(epoch_account.registered_stake);
}

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
    rewards_earned: u64,
    epoch: u64,
    stake_weight: u64,
}
impl CompressedForesterEpochAccountInput {
    pub fn into_compressed_forester_epoch_account(
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

/// 1. input a vector of compressed forester epoch accounts
/// 2. Check that epoch of first compressed forester epoch account is less than StakeAccount.last_sync_epoch
/// 3. iterate over all compressed forester epoch accounts, increase Account.stake_weight by rewards_earned in every step
/// 4. set StakeAccount.last_sync_epoch to the epoch of the last compressed forester epoch account
/// 5. prove inclusion of last hash in State merkle tree
pub fn sync_stake_account(
    stake_account: &mut StakeAccount,
    compressed_forester_epoch_accounts: Vec<CompressedForesterEpochAccountInput>,
    hashed_forester_pubkey: [u8; 32],
    mut previous_hash: [u8; 32],
    // last_account_merkle_tree_pubkey: Pubkey,
    // last_account_leaf_index: u64,
    // inclusion_proof: CompressedProof,
    // root_index: u16,
) -> Result<()> {
    if compressed_forester_epoch_accounts.is_empty() {
        return Ok(());
    }
    let last_sync_epoch = stake_account.last_sync_epoch;
    if compressed_forester_epoch_accounts[0].epoch <= last_sync_epoch {
        return err!(ErrorCode::StakeAccountAlreadySynced);
    }

    for compressed_forester_epoch_account in compressed_forester_epoch_accounts.iter() {
        // Forester pubkey is not hashed thus we use a random value and hash offchain
        let compressed_forester_epoch_account = compressed_forester_epoch_account
            .into_compressed_forester_epoch_account(previous_hash, crate::ID);
        previous_hash = compressed_forester_epoch_account.hash(hashed_forester_pubkey)?;
        let get_staker_epoch_reward =
            compressed_forester_epoch_account.get_reward(stake_account.delegated_stake_weight);
        stake_account.delegated_stake_weight += get_staker_epoch_reward;
        stake_account.pending_token_amount += get_staker_epoch_reward;
    }
    stake_account.last_sync_epoch = compressed_forester_epoch_accounts
        .iter()
        .last()
        .unwrap()
        .epoch;

    // included for completeness
    // let last_compressed_forester_account_hash = CompressedAccount {
    //     owner: crate::ID,
    //     lamports: 0,
    //     address: None,
    //     data: Some(CompressedAccountData {
    //         discriminator: [0, 0, 0, 0, 0, 0, 0, 1],
    //         data_hash: previous_hash,
    //         data: vec![],
    //     }),
    // }
    // .hash(&last_account_merkle_tree_pubkey, leaf_index)
    // .map_err(ProgramError::from)?;
    // let root = get_root(root_index);
    // verify_last_compressed_forester_account_hash zkp_inclusion_proof(root, last_compressed_forester_account_hash)?;
    Ok(())
}

pub fn stake(stake_account: &mut StakeAccount, num_tokens: u64) -> Result<()> {
    stake_account.stake_weight += num_tokens;
    Ok(())
}

pub fn delegate(
    stake_account: &mut StakeAccount,
    forester_stake_account_pubkey: &Pubkey,
    forester_stake_account: &mut ForesterStakeAccount,
    num_tokens: u64,
    current_slot: u64,
) -> Result<()> {
    forester_stake_account.sync(current_slot);
    // TODO: check that is not delegated to a different forester
    stake_account.delegate_forester_stake_account = Some(*forester_stake_account_pubkey);
    forester_stake_account.pending_stake_weight += num_tokens;
    stake_account.delegated_stake_weight += num_tokens;
    stake_account.stake_weight -= num_tokens;
    Ok(())
}

pub fn register_for_epoch(
    forester_stake_account: &mut ForesterStakeAccount,
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<()> {
    // Check whether we are in a epoch registration phase and which epoch we are in
    let current_epoch_start_slot = forester_stake_account
        .protocol_config
        .is_registration_period(current_slot)?;

    // Init epoch account if not initialized
    if *epoch_account == EpochAccount::default() {
        *epoch_account = EpochAccount {
            epoch: forester_stake_account.current_epoch,
            protocol_config: forester_stake_account.protocol_config,
            ..EpochAccount::default()
        };
    }

    forester_stake_account.sync(current_slot);
    forester_epoch_account.stake_weight += forester_stake_account.active_stake_weight;
    forester_epoch_account.epoch = forester_stake_account.current_epoch;
    forester_epoch_account.forester_index = epoch_account.registered_stake;
    epoch_account.registered_stake += forester_stake_account.active_stake_weight;
    forester_epoch_account.epoch_start_slot = current_epoch_start_slot;
    forester_epoch_account.protocol_config = forester_stake_account.protocol_config;

    Ok(())
}

pub fn simulate_work_and_slots(
    forester_epoch_account: &mut ForesterEpochAccount,
    current_slot: &mut u64,
    num_slots_and_work: u64,
    queue_pubkey: &Pubkey,
) -> Result<()> {
    forester_epoch_account.check_eligibility(*current_slot, queue_pubkey)?;
    forester_epoch_account.work_counter += num_slots_and_work;
    *current_slot += num_slots_and_work;
    Ok(())
}

pub fn tally_results(
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<()> {
    if !epoch_account
        .protocol_config
        .is_in_tally_period(current_slot, forester_epoch_account.epoch)
    {
        return err!(ErrorCode::TallyPeriodNotStarted);
    }

    forester_epoch_account.has_tallied = true;
    epoch_account.last_epoch_total_stake_weight += forester_epoch_account.stake_weight;
    epoch_account.last_epoch_tally += forester_epoch_account.work_counter;
    Ok(())
}

pub struct MockCompressedTokenAccount {
    pub balance: u64,
}

/// 1. Transfer forester fees to foresters token account
/// 2. Transfer rewards to foresters token account
/// 3. compress forester epoch account
/// 4. close forester epoch account
/// 5. if all stake has claimed close epoch account
pub fn forester_claim_rewards(
    forester_token_account: &mut MockCompressedTokenAccount,
    forester_token_pool_account: &mut MockCompressedTokenAccount,
    forester_stake_account: &mut ForesterStakeAccount,
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<CompressedForesterEpochAccount> {
    if !epoch_account
        .protocol_config
        .is_post_tally_period(current_slot, forester_epoch_account.epoch)
    {
        return err!(ErrorCode::TallyPeriodNotStarted);
    }

    let total_stake_weight = epoch_account.last_epoch_total_stake_weight;
    let total_tally = epoch_account.last_epoch_tally;
    let forester_stake_weight = forester_epoch_account.stake_weight;
    let forester_tally = forester_epoch_account.work_counter;
    let reward = epoch_account.protocol_config.get_rewards(
        total_stake_weight,
        total_tally,
        forester_stake_weight,
        forester_tally,
    );
    let fee = reward * forester_stake_account.forester_config.fee / 100;
    // Transfer fees to forester
    forester_token_account.balance += fee;
    let net_reward = reward - fee;
    // Transfer net_reward to forester pool account
    // Stakers can claim from this account
    forester_token_pool_account.balance += net_reward;
    // Increase the active deleagted stake weight by the net_reward
    forester_stake_account.active_stake_weight += net_reward;
    let compressed_account = CompressedForesterEpochAccount {
        rewards_earned: net_reward,
        epoch: forester_epoch_account.epoch,
        stake_weight: forester_epoch_account.stake_weight,
        previous_hash: forester_stake_account.last_compressed_forester_epoch_account_hash,
        forester_pubkey: forester_epoch_account.forester_pubkey,
    };
    let hashed_forester_pubkey = hash_to_bn254_field_size_be(
        &forester_stake_account
            .forester_config
            .forester_pubkey
            .to_bytes(),
    )
    .unwrap()
    .0;
    let compressed_account_hash = compressed_account.hash(hashed_forester_pubkey)?;
    forester_stake_account.last_compressed_forester_epoch_account_hash = compressed_account_hash;
    Ok(compressed_account)
}

pub fn undelegate(
    stake_account: &mut StakeAccount,
    forester_stake_account: &mut ForesterStakeAccount,
    amount: u64,
    current_slot: u64,
) -> Result<()> {
    forester_stake_account.sync(current_slot);
    forester_stake_account.active_stake_weight -= amount;
    stake_account.delegated_stake_weight -= amount;
    stake_account.pending_stake_weight += amount;
    stake_account.pending_epoch = forester_stake_account
        .protocol_config
        .get_current_epoch(current_slot);
    if stake_account.delegated_stake_weight == 0 {
        stake_account.delegate_forester_stake_account = None;
    }
    Ok(())
}

pub fn unstake(
    stake_account: &mut StakeAccount,
    stake_token_account: &mut MockCompressedTokenAccount,
    recipient_token_account: &mut MockCompressedTokenAccount,
    protocol_config: ProtocolConfig,
    amount: u64,
    current_slot: u64,
) -> Result<()> {
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    stake_account.sync_pending_stake_weight(current_epoch);
    // reduce stake weight
    // only non delegated stake can be unstaked
    stake_account.stake_weight -= amount;
    // transfer tokens
    stake_token_account.balance -= amount;
    recipient_token_account.balance += amount;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    /// Questions:
    ///
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
    // TODO: add unstake
    #[test]
    fn staking_scenario() {
        let epoch_length = 10;
        let protocol_config = ProtocolConfig {
            genesis_slot: 20,
            registration_period_length: 1,
            epoch_length,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
        };
        let mut current_slot = 0;

        // ----------------------------------------------------------------------------------------
        // 2. User stakes 1000 tokens
        let mut user_stake_account = StakeAccount::default();
        let user_token_balance = 1000;
        stake(&mut user_stake_account, user_token_balance).unwrap();

        // ----------------------------------------------------------------------------------------
        // 3. Forester creates a stake and token accounts
        let forester_pubkey = Pubkey::new_unique();
        let forester_stake_account_pubkey = Pubkey::new_unique();
        let mut forester_stake_account = ForesterStakeAccount {
            protocol_config,
            forester_config: ForesterConfig {
                forester_pubkey,
                fee: 10,
            },
            ..ForesterStakeAccount::default()
        };
        let mut forester_token_account = MockCompressedTokenAccount { balance: 0 };
        let mut forester_token_pool_account = MockCompressedTokenAccount { balance: 0 };

        // ----------------------------------------------------------------------------------------
        // 4. User delegates 1000 tokens to forester
        delegate(
            &mut user_stake_account,
            &forester_stake_account_pubkey,
            &mut forester_stake_account,
            user_token_balance,
            current_slot,
        )
        .unwrap();
        assert_eq!(
            forester_stake_account.pending_stake_weight,
            user_stake_account.delegated_stake_weight
        );
        assert_eq!(
            forester_stake_account_pubkey,
            user_stake_account.delegate_forester_stake_account.unwrap()
        );
        // We need to start in epoch 1, because nobody can register before epoch 0
        current_slot = 30;

        // ----------------------------------------------------------------------------------------
        // 5. Forester registers for epoch and initializes epoch account and
        let mut epoch_account = EpochAccount::default();
        let mut forester_epoch_account = ForesterEpochAccount::default();
        register_for_epoch(
            &mut forester_stake_account,
            &mut forester_epoch_account,
            &mut epoch_account,
            current_slot,
        )
        .unwrap();
        assert_eq!(forester_stake_account.pending_stake_weight, 0);
        assert_eq!(forester_epoch_account.stake_weight, user_token_balance);
        assert_eq!(epoch_account.registered_stake, user_token_balance);
        assert_eq!(forester_epoch_account.epoch_start_slot, 30);
        current_slot = 31;

        // ----------------------------------------------------------------------------------------
        // 6. Forester performs some actions until epoch ends
        set_total_registered_stake(&mut forester_epoch_account, &epoch_account);
        simulate_work_and_slots(
            &mut forester_epoch_account,
            &mut current_slot,
            epoch_length,
            &Pubkey::new_unique(),
        )
        .unwrap();

        // ----------------------------------------------------------------------------------------
        // 7. Tally results
        tally_results(
            &mut forester_epoch_account,
            &mut epoch_account,
            current_slot,
        )
        .unwrap();

        // ----------------------------------------------------------------------------------------
        // 8. Forester claim rewards
        let compressed_forester_epoch_account = forester_claim_rewards(
            &mut forester_token_account,
            &mut forester_token_pool_account,
            &mut forester_stake_account,
            &mut forester_epoch_account,
            &mut epoch_account,
            current_slot,
        )
        .unwrap();
        let forester_fee = 10_000;
        assert_eq!(forester_token_account.balance, forester_fee);
        assert_eq!(
            forester_token_pool_account.balance,
            protocol_config.epoch_reward - forester_fee
        );

        // ----------------------------------------------------------------------------------------
        // 9. User syncs stake account
        let hashed_forester_pubkey = hash_to_bn254_field_size_be(&forester_pubkey.to_bytes())
            .unwrap()
            .0;
        let compressed_forester_epoch_account_input_account = CompressedForesterEpochAccountInput {
            rewards_earned: compressed_forester_epoch_account.rewards_earned,
            epoch: compressed_forester_epoch_account.epoch,
            stake_weight: compressed_forester_epoch_account.stake_weight,
        };
        sync_stake_account(
            &mut user_stake_account,
            vec![compressed_forester_epoch_account_input_account],
            hashed_forester_pubkey,
            compressed_forester_epoch_account.previous_hash,
        )
        .unwrap();

        assert_eq!(
            user_stake_account.delegated_stake_weight,
            user_token_balance + compressed_forester_epoch_account.rewards_earned
        );
        println!(
            "compressed_forester_epoch_account.rewards_earned: {}",
            compressed_forester_epoch_account.rewards_earned
        );
        println!(
            "User d delegated_stake_weight: {}",
            user_stake_account.delegated_stake_weight
        );

        // ----------------------------------------------------------------------------------------
        // 10. User unstakes
        let mut user_stake_token_account = MockCompressedTokenAccount { balance: 1000 };
        let mut recipient_token_account = MockCompressedTokenAccount { balance: 0 };
        let unstake_amount = 100;
        undelegate(
            &mut user_stake_account,
            &mut forester_stake_account,
            unstake_amount,
            current_slot,
        )
        .unwrap();
        assert_eq!(user_stake_account.pending_stake_weight, unstake_amount);
        assert_eq!(
            forester_stake_account.active_stake_weight,
            900 + protocol_config.epoch_reward - forester_fee
        );
        unstake(
            &mut user_stake_account,
            &mut user_stake_token_account,
            &mut recipient_token_account,
            protocol_config,
            unstake_amount,
            current_slot,
        )
        .unwrap();
        assert_eq!(user_stake_account.delegated_stake_weight, 90900);
        assert_eq!(user_stake_token_account.balance, 900);
        assert_eq!(recipient_token_account.balance, unstake_amount);
    }

    #[test]
    fn test_zero_values() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            registration_period_length: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
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
            registration_period_length: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
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
            registration_period_length: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
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
            registration_period_length: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
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
            registration_period_length: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
            min_stake: 0,
            slot_length: 1,
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

    fn setup_forester_epoch_account(
        forester_start_range: u64,
        forester_stake_weight: u64,
        epoch_length: u64,
        slot_length: u64,
        epoch_start_slot: u64,
        total_epoch_state_weight: u64,
    ) -> ForesterEpochAccount {
        ForesterEpochAccount {
            forester_pubkey: Pubkey::default(),
            epoch: 0,
            stake_weight: forester_stake_weight,
            work_counter: 0,
            has_tallied: false,
            forester_index: forester_start_range,
            epoch_start_slot,
            slot_length,
            total_epoch_state_weight: Some(total_epoch_state_weight),
            protocol_config: ProtocolConfig {
                genesis_slot: 0,
                registration_period_length: 1,
                epoch_length,
                tally_period_length: 2,
                epoch_reward: 100_000,
                base_reward: 50_000,
                min_stake: 0,
                slot_length: 1,
            },
        }
    }

    // Instead of index I use stake weight to get the period
    #[test]
    fn test_eligibility_check_within_epoch() {
        let mut eligible = HashMap::<u8, (u64, u64)>::new();
        let slot_length = 20;
        let num_foresters = 5;
        let epoch_start_slot = 10;
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
            let account = setup_forester_epoch_account(
                current_total_stake_weight,
                forester_stake_weight,
                epoch_len,
                slot_length,
                epoch_start_slot,
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
            // assert_eq!(*num_eligible_slots, slots_per_forester);
        }

        let sum = eligible.values().map(|x| x.1).sum::<u64>();
        let total_slots: u64 = (epoch_len - epoch_start_slot) - 1;
        assert_eq!(sum, total_slots);
    }
}
