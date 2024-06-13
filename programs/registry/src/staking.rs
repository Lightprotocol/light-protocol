use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_be;

#[error_code]
pub enum ErrorCode {
    TallyPeriodNotStarted,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolConfig {
    pub genesis_slot: u64,
    /// In epochs
    pub stake_activation_delay: u64,
    /// Epoch length in slots
    pub epoch_length: u64,
    /// Must be less than epoch_length
    pub tally_period_length: u64,
    pub epoch_reward: u64,
    pub base_reward: u64,
}

impl ProtocolConfig {
    pub fn get_current_epoch(&self, slot: u64) -> u64 {
        (slot - self.genesis_slot) / self.epoch_length
    }

    pub fn get_current_epoch_progress(&self, slot: u64) -> u64 {
        (slot - self.genesis_slot) % self.epoch_length
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
}

// TODO: check how Solana handle the delegation and undelegation process (the way it is now we cannot top up a delegated account)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StakeAccount {
    stake_weight: u64,
    delegate_forester_stake_account: Pubkey,
    activation_epoch: u64,
    deactivation_epoch: u64,
    last_sync_epoch: u64,
    pending_token_amount: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterStakeAccount {
    forester_config: ForesterConfig,
    active_stake_weight: u64,
    pending_stake_weight: u64,
    pending_deactivate_stake_weight: u64,
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
    total_stake_weight: u64,
    counter: u64,
    has_tallied: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedForesterEpochAccount {
    rewards_earned: u64,
    epoch: u64,
    total_stake_weight: u64,
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
            &self.total_stake_weight.to_le_bytes(),
        ])
        .map_err(ProgramError::from)?;
        Ok(hash)
    }

    pub fn get_reward(&self, stake: u64) -> u64 {
        self.rewards_earned * stake / self.total_stake_weight
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedForesterEpochAccountInput {
    rewards_earned: u64,
    epoch: u64,
    total_stake_weight: u64,
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
            total_stake_weight: self.total_stake_weight,
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
pub fn sync_staker_account(
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
        return err!(ErrorCode::TallyPeriodNotStarted);
    }

    for compressed_forester_epoch_account in compressed_forester_epoch_accounts.iter() {
        // Forester pubkey is not hashed thus we use a random value and hash offchain
        let compressed_forester_epoch_account = compressed_forester_epoch_account
            .into_compressed_forester_epoch_account(previous_hash, crate::ID);
        previous_hash = compressed_forester_epoch_account.hash(hashed_forester_pubkey)?;
        let get_staker_epoch_reward =
            compressed_forester_epoch_account.get_reward(stake_account.stake_weight);
        stake_account.stake_weight += get_staker_epoch_reward;
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

// It is easier to make stake active immediately.
// It is easy to enforce a cool off period for unstaking. // TODO: research whats the science behind cool off periods
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
    stake_account.delegate_forester_stake_account = *forester_stake_account_pubkey;
    forester_stake_account.pending_stake_weight += num_tokens;
    Ok(())
}

pub fn register_for_epoch(
    forester_stake_account: &mut ForesterStakeAccount,
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<()> {
    // Init epoch account if not initialized
    if *epoch_account == EpochAccount::default() {
        *epoch_account = EpochAccount {
            epoch: forester_stake_account.current_epoch,
            protocol_config: forester_stake_account.protocol_config,
            ..EpochAccount::default()
        };
    }

    forester_stake_account.sync(current_slot);
    forester_epoch_account.total_stake_weight += forester_stake_account.active_stake_weight;
    forester_epoch_account.epoch = forester_stake_account.current_epoch;
    // forester_epoch_account.slot_updated = current_slot;

    Ok(())
}

pub fn simulate_work_and_slots(
    forester_epoch_account: &mut ForesterEpochAccount,
    current_slot: &mut u64,
    num_slots_and_work: u64,
) -> Result<()> {
    forester_epoch_account.counter += num_slots_and_work;
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
    epoch_account.last_epoch_total_stake_weight += forester_epoch_account.total_stake_weight;
    epoch_account.last_epoch_tally += forester_epoch_account.counter;
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
    let forester_stake_weight = forester_epoch_account.total_stake_weight;
    let forester_tally = forester_epoch_account.counter;
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
        rewards_earned: reward,
        epoch: forester_epoch_account.epoch,
        total_stake_weight: forester_epoch_account.total_stake_weight,
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

pub fn staker_sync_account(
    stake_account: &mut StakeAccount,
    protocol_config: &ProtocolConfig,
    current_slot: u64,
) -> Result<()> {
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    if current_epoch > stake_account.last_sync_epoch {
        stake_account.last_sync_epoch = current_epoch;
        stake_account.stake_weight -= stake_account.deactivation_epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn advance_epoch(slot: &mut u64) {
        *slot += 11;
    }

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
    // TODO: add contention prevention
    // TODO: add unstake
    #[test]
    fn staking_scenario() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
            user_stake_account.stake_weight
        );
        advance_epoch(&mut current_slot);

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
        assert_eq!(
            forester_epoch_account.total_stake_weight,
            user_token_balance
        );

        // ----------------------------------------------------------------------------------------
        // 6. Forester performs some actions until epoch ends
        simulate_work_and_slots(&mut forester_epoch_account, &mut current_slot, 10).unwrap();

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

        // ----------------------------------------------------------------------------------------
        // 9. User syncs stake account
        let hashed_forester_pubkey = hash_to_bn254_field_size_be(&forester_pubkey.to_bytes())
            .unwrap()
            .0;
        let compressed_forester_epoch_account_input_account = CompressedForesterEpochAccountInput {
            rewards_earned: compressed_forester_epoch_account.rewards_earned,
            epoch: compressed_forester_epoch_account.epoch,
            total_stake_weight: compressed_forester_epoch_account.total_stake_weight,
        };
        sync_staker_account(
            &mut user_stake_account,
            vec![compressed_forester_epoch_account_input_account],
            hashed_forester_pubkey,
            compressed_forester_epoch_account.previous_hash,
        )
        .unwrap();

        assert_eq!(
            user_stake_account.stake_weight,
            user_token_balance + compressed_forester_epoch_account.rewards_earned
        );
    }

    #[test]
    fn test_zero_values() {
        let protocol_config = ProtocolConfig {
            genesis_slot: 0,
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
            stake_activation_delay: 1,
            epoch_length: 10,
            tally_period_length: 2,
            epoch_reward: 100_000,
            base_reward: 50_000,
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
