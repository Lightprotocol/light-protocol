use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_be;

#[error_code]
pub enum ErrorCode {
    NotInReportWorkPhase,
    StakeAccountAlreadySynced,
    EpochEnded,
    ForresterNotEligible,
    NotInRegistrationPeriod,
    StakeInsuffient,
    ForesterAlreadyRegistered,
    InvalidEpochAccount,
    InvalidEpoch,
    EpochStillInProgress,
    NotInActivePhase,
}

// TODO: replace epoch reward with inflation curve.
/// Epoch Phases:
/// 1. Registration
/// 2. Active
/// 3. Report Work
/// 4. Post (Epoch has ended, and rewards can be claimed.)
/// - There is always an active phase in progress, registration and report work
///   phases run in parallel to a currently active phase.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
    pub slot_length: u64,
    /// Foresters can register for this phase.
    pub registration_phase_length: u64,
    /// Foresters can perform work in this phase.
    pub active_phase_length: u64,
    /// Foresters can report work to receive performance based rewards in this
    /// phase.
    pub report_work_phase_length: u64,
}

impl ProtocolConfig {
    pub fn get_current_epoch(&self, slot: u64) -> u64 {
        (slot.saturating_sub(self.genesis_slot)) / self.active_phase_length
    }
    pub fn get_current_active_epoch(&self, slot: u64) -> Result<u64> {
        let slot = match slot.checked_sub(self.genesis_slot + self.registration_phase_length) {
            Some(slot) => slot,
            None => return err!(ErrorCode::EpochEnded),
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
            return err!(ErrorCode::NotInRegistrationPeriod);
        }
        Ok((current_epoch) * self.active_phase_length
            + self.genesis_slot
            + self.registration_phase_length)
    }

    pub fn is_active_phase(&self, slot: u64, epoch: u64) -> Result<()> {
        if self.get_current_active_epoch(slot)? != epoch {
            return err!(ErrorCode::NotInActivePhase);
        }
        Ok(())
    }

    pub fn is_report_work_phase(&self, slot: u64, epoch: u64) -> Result<()> {
        self.is_active_phase(slot, epoch + 1)?;
        let current_epoch_progress = self.get_current_active_epoch_progress(slot);
        if current_epoch_progress >= self.report_work_phase_length {
            return err!(ErrorCode::NotInReportWorkPhase);
        }
        Ok(())
    }

    pub fn is_post_epoch(&self, slot: u64, epoch: u64) -> Result<()> {
        if self.get_current_active_epoch(slot)? == epoch {
            return err!(ErrorCode::InvalidEpoch);
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

/// Is used for tallying and rewards calculation
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EpochAccount {
    pub epoch: u64,
    pub protocol_config: ProtocolConfig,
    pub total_work: u64,
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
    /// Pending stake which will get active once the next epoch starts.
    pending_stake_weight: u64,
    current_epoch: u64,
    /// Link to previous compressed forester epoch account hash.
    last_compressed_forester_epoch_account_hash: [u8; 32],
    last_registered_epoch: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterConfig {
    pub forester_pubkey: Pubkey,
    /// Fee in percentage points.
    pub fee: u64,
}

impl ForesterStakeAccount {
    /// If current epoch changed, move pending stake to active stake and update
    /// current epoch field
    pub fn sync(&mut self, current_slot: u64, protocol_config: &ProtocolConfig) -> Result<()> {
        // get last registration epoch, stake sync treats the registration phase
        // of an epoch like the next active epoch
        let current_epoch = protocol_config.get_current_active_epoch(
            current_slot.saturating_sub(protocol_config.registration_phase_length),
        )?;
        // If the current epoch is greater than the last registered epoch, or next epoch is in registration phase
        if current_epoch > self.current_epoch
            || protocol_config.is_registration_phase(current_slot).is_ok()
        {
            self.current_epoch = current_epoch;
            self.active_stake_weight += self.pending_stake_weight;
            self.pending_stake_weight = 0;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ForesterEpochAccount {
    forester_config: ForesterConfig,
    epoch: u64,
    stake_weight: u64,
    work_counter: u64,
    /// Work can be reported in an extra round to earn extra performance based
    /// rewards. // TODO: make sure that performance based rewards can only be
    /// claimed if work has been reported
    has_reported_work: bool,
    /// Start index of the range that determines when the forester is eligible to perform work.
    /// End index is forester_start_index + stake_weight
    forester_index: u64,
    epoch_start_slot: u64,
    /// Total epoch state weight is registered stake of the epoch account after
    /// registration is concluded and active epoch period starts.
    total_epoch_state_weight: Option<u64>,
    protocol_config: ProtocolConfig,
}

impl ForesterEpochAccount {
    pub fn get_current_slot(&self, current_slot: u64) -> Result<u64> {
        if current_slot >= self.epoch_start_slot + self.protocol_config.active_phase_length {
            return err!(ErrorCode::EpochEnded);
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

    /// Check forester account is:
    /// - of correct epoch
    /// - eligible to perform work in the current slot
    pub fn check_eligibility(&self, current_slot: u64, pubkey: &Pubkey) -> Result<()> {
        self.protocol_config
            .is_active_phase(current_slot, self.epoch)?;

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
pub fn set_total_registered_stake_instruction(
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

/// Sync Stake Account:
/// - syncs the virtual balance of accumulated stake rewards to the stake
///   account
/// - it does not sync the token stake account balance
/// - the token stake account balance must be fully synced to perform any
///   actions that move delegated stake
/// 1. input a vector of compressed forester epoch accounts
/// 2. Check that epoch of first compressed forester epoch account is less than
///    StakeAccount.last_sync_epoch
/// 3. iterate over all compressed forester epoch accounts, increase
///    Account.stake_weight by rewards_earned in every step
/// 4. set StakeAccount.last_sync_epoch to the epoch of the last compressed
///    forester epoch account
/// 5. prove inclusion of last hash in State merkle tree
pub fn sync_stake_account_instruction(
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

/// Sync Token Account:
/// - syncs the user stake compressed token accounts with the pending token amount of
///   the stake account
pub fn sync_token_account_instruction(
    forester_token_pool_account: &mut MockSplTokenAccount,
    user_staking_compressed_token_account: &mut MockCompressedTokenAccount,
    user_stake_account: &mut StakeAccount,
) {
    forester_token_pool_account.balance -= user_stake_account.pending_token_amount;
    user_staking_compressed_token_account.balance += user_stake_account.pending_token_amount;
    user_stake_account.pending_token_amount = 0;
}

pub fn stake_instruction(
    stake_account: &mut StakeAccount,
    num_tokens: u64,
    token_account: &mut MockCompressedTokenAccount,
    stake_token_account: &mut MockCompressedTokenAccount,
) -> Result<()> {
    stake_account.stake_weight += num_tokens;

    stake_token_account.balance += num_tokens;
    token_account.balance -= num_tokens;
    Ok(())
}

pub fn delegate_instruction(
    protocol_config: &ProtocolConfig,
    stake_account: &mut StakeAccount,
    forester_stake_account_pubkey: &Pubkey,
    forester_stake_account: &mut ForesterStakeAccount,
    num_tokens: u64,
    current_slot: u64,
    no_sync: bool,
) -> Result<()> {
    if !no_sync {
        forester_stake_account.sync(current_slot, protocol_config)?;
    }
    // TODO: check that is not delegated to a different forester
    stake_account.delegate_forester_stake_account = Some(*forester_stake_account_pubkey);
    forester_stake_account.pending_stake_weight += num_tokens;
    stake_account.delegated_stake_weight += num_tokens;
    stake_account.stake_weight -= num_tokens;
    Ok(())
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
pub fn register_for_epoch_instruction(
    protocol_config: &ProtocolConfig,
    forester_stake_account: &mut ForesterStakeAccount,
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<()> {
    // Init epoch account if not initialized
    if *epoch_account == EpochAccount::default() {
        let current_epoch = protocol_config.get_current_active_epoch(current_slot)?;
        *epoch_account = EpochAccount {
            epoch: current_epoch + 1,
            protocol_config: *protocol_config,
            total_work: 0,
            registered_stake: 0,
        };
    }

    if forester_stake_account.active_stake_weight < epoch_account.protocol_config.min_stake {
        return err!(ErrorCode::StakeInsuffient);
    }
    if forester_stake_account.last_registered_epoch == epoch_account.epoch {
        // With onchain implementation this error will not be necessary for the pda will be derived from the epoch
        // from the forester pubkey.
        return err!(ErrorCode::ForesterAlreadyRegistered);
    }
    // Check whether we are in a epoch registration phase and which epoch we are in
    let current_epoch_start_slot = epoch_account
        .protocol_config
        .is_registration_phase(current_slot)?;

    // Sync pending stake to active stake if stake hasn't been synced yet.
    forester_stake_account.sync(current_slot, &epoch_account.protocol_config)?;
    forester_stake_account.last_registered_epoch = epoch_account.epoch;
    // Initialize forester epoch account.
    let initialized_forester_epoch_account = ForesterEpochAccount {
        forester_config: forester_stake_account.forester_config,
        epoch: epoch_account.epoch,
        stake_weight: forester_stake_account.active_stake_weight,
        work_counter: 0,
        has_reported_work: false,
        forester_index: epoch_account.registered_stake,
        epoch_start_slot: current_epoch_start_slot,
        total_epoch_state_weight: None,
        protocol_config: epoch_account.protocol_config,
    };
    forester_epoch_account.clone_from(&initialized_forester_epoch_account);

    // Add forester active stake to epoch registered stake.
    epoch_account.registered_stake += forester_stake_account.active_stake_weight;
    Ok(())
}

/// Simulate work and slots for testing purposes.
/// 1. Check eligibility of forester
pub fn simulate_work_and_slots_instruction(
    forester_epoch_account: &mut ForesterEpochAccount,
    num_work: u64,
    queue_pubkey: &Pubkey,
    current_slot: &u64,
) -> Result<()> {
    forester_epoch_account
        .protocol_config
        .is_active_phase(*current_slot, forester_epoch_account.epoch)?;
    forester_epoch_account.check_eligibility(*current_slot, queue_pubkey)?;
    forester_epoch_account.work_counter += num_work;
    Ok(())
}

/// Report work:
/// - work is reported so that performance based rewards can be calculated after
///   the report work phase ends
/// 1. Check that we are in the report work phase
/// 2. Check that forester has registered for the epoch
/// 3. Check that forester has not already reported work
/// 4. Add work to total work
///
/// Considerations:
/// - we could remove this phase:
///     -> con: we would have no performance based rewards
///     -> pro: reduced complexity
/// 1. Design possibilities even without a separate phase:
///   - we could introduce a separate reward just per work performed (uncapped,
///     for weighted cap we need this round, hardcoded cap would work without
///     this round)
///   - reward could be in sol, or light tokens
pub fn report_work_instruction(
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<()> {
    epoch_account
        .protocol_config
        .is_report_work_phase(current_slot, epoch_account.epoch)?;

    if forester_epoch_account.epoch != epoch_account.epoch {
        return err!(ErrorCode::InvalidEpochAccount);
    }
    if forester_epoch_account.has_reported_work {
        return err!(ErrorCode::ForesterAlreadyRegistered);
    }

    forester_epoch_account.has_reported_work = true;
    epoch_account.total_work += forester_epoch_account.work_counter;
    Ok(())
}

pub struct MockCompressedTokenAccount {
    pub balance: u64,
}

pub struct MockSplTokenAccount {
    pub balance: u64,
}

/// Forester claim rewards:
/// 1. Transfer forester fees to foresters token account
/// 2. Transfer rewards to foresters token account
/// 3. compress forester epoch account
/// 4. close forester epoch account
/// 5. if all stake has claimed close epoch account
pub fn forester_claim_rewards_instruction(
    forester_fee_token_account: &mut MockCompressedTokenAccount,
    forester_token_pool_account: &mut MockSplTokenAccount,
    forester_stake_account: &mut ForesterStakeAccount,
    forester_epoch_account: &mut ForesterEpochAccount,
    epoch_account: &mut EpochAccount,
    current_slot: u64,
) -> Result<CompressedForesterEpochAccount> {
    epoch_account
        .protocol_config
        .is_post_epoch(current_slot, forester_epoch_account.epoch)?;

    let total_stake_weight = epoch_account.registered_stake;
    let total_tally = epoch_account.total_work;
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
    forester_fee_token_account.balance += fee;
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
        forester_pubkey: forester_epoch_account.forester_config.forester_pubkey,
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

pub fn undelegate_instruction(
    protocol_config: &ProtocolConfig,
    stake_account: &mut StakeAccount,
    forester_stake_account: &mut ForesterStakeAccount,
    amount: u64,
    current_slot: u64,
) -> Result<()> {
    forester_stake_account.sync(current_slot, protocol_config)?;
    forester_stake_account.active_stake_weight -= amount;
    stake_account.delegated_stake_weight -= amount;
    stake_account.pending_stake_weight += amount;
    stake_account.pending_epoch = protocol_config.get_current_epoch(current_slot);
    if stake_account.delegated_stake_weight == 0 {
        stake_account.delegate_forester_stake_account = None;
    }
    Ok(())
}

pub fn unstake_instruction(
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
        };
        let mut current_solana_slot = protocol_config.genesis_slot;

        // ----------------------------------------------------------------------------------------
        // 2. User stakes 1000 tokens
        // - token transfer from compressed token account to compressed token staking account
        //   - the compressed token staking account stores all staked tokens
        // - staked tokens are not automatically delegated
        // - user_stake_account stake_weight is increased by the added stake amount
        let mut user_stake_account = StakeAccount::default();
        let user_token_balance = 1000;
        let mut user_stake_token_account = MockCompressedTokenAccount { balance: 0 };
        let mut user_token_account = MockCompressedTokenAccount { balance: 1000 };
        stake_instruction(
            &mut user_stake_account,
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
        // - forester_stake_account
        let forester_pubkey = Pubkey::new_unique();
        let forester_stake_account_pubkey = Pubkey::new_unique();
        let mut forester_stake_account = ForesterStakeAccount {
            forester_config: ForesterConfig {
                forester_pubkey,
                fee: 10,
            },
            ..ForesterStakeAccount::default()
        };
        // Forester fee rewards go to this compressed token account.
        let mut forester_fee_token_account = MockCompressedTokenAccount { balance: 0 };
        // Is an spl token account since it will need to be accessed by many parties.
        // Compressed token accounts are not well suited to be pool accounts.
        let mut forester_token_pool_account = MockSplTokenAccount { balance: 0 };

        // ----------------------------------------------------------------------------------------
        // 4. User delegates 1000 tokens to forester
        // - delegated stake is not active until the next epoch
        //   - this is enforced by adding the delegated stake to forester_stake_account  pending stake weight
        //   - forester_stake_account pending stake weight is synced to active stake weight once the epoch changes
        // - delegated stake is stored in the user_stake_account delegated_stake_weight
        delegate_instruction(
            &protocol_config,
            &mut user_stake_account,
            &forester_stake_account_pubkey,
            &mut forester_stake_account,
            user_token_balance,
            current_solana_slot,
            true,
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
        let mut epoch_account = EpochAccount::default();
        let mut forester_epoch_account = ForesterEpochAccount::default();
        register_for_epoch_instruction(
            &protocol_config,
            &mut forester_stake_account,
            &mut forester_epoch_account,
            &mut epoch_account,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(forester_stake_account.pending_stake_weight, 0);
        assert_eq!(forester_epoch_account.stake_weight, user_token_balance);
        assert_eq!(epoch_account.registered_stake, user_token_balance);
        assert_eq!(forester_epoch_account.epoch_start_slot, 28);
        assert_eq!(forester_epoch_account.epoch, 1);
        // ----------------------------------------------------------------------------------------
        // Registration phase ends (epoch 1)
        // Active phase starts (epoch 1)
        current_solana_slot += protocol_config.registration_phase_length;
        assert!(protocol_config
            .is_registration_phase(current_solana_slot)
            .is_err());
        // ----------------------------------------------------------------------------------------
        // 6. Forester performs some actions until epoch ends
        set_total_registered_stake_instruction(&mut forester_epoch_account, &epoch_account);
        simulate_work_and_slots_instruction(
            &mut forester_epoch_account,
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
        report_work_instruction(
            &mut forester_epoch_account,
            &mut epoch_account,
            current_solana_slot,
        )
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
        let compressed_forester_epoch_account = forester_claim_rewards_instruction(
            &mut forester_fee_token_account,
            &mut forester_token_pool_account,
            &mut forester_stake_account,
            &mut forester_epoch_account,
            &mut epoch_account,
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
        let compressed_forester_epoch_account_input_account = CompressedForesterEpochAccountInput {
            rewards_earned: compressed_forester_epoch_account.rewards_earned,
            epoch: compressed_forester_epoch_account.epoch,
            stake_weight: compressed_forester_epoch_account.stake_weight,
        };
        sync_stake_account_instruction(
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
        assert_eq!(user_stake_account.pending_token_amount, 90_000);

        sync_token_account_instruction(
            &mut forester_token_pool_account,
            &mut user_stake_token_account,
            &mut user_stake_account,
        );
        assert_eq!(user_stake_token_account.balance, 1000 + 90_000);
        assert_eq!(forester_token_pool_account.balance, 0);
        assert_eq!(user_stake_account.pending_token_amount, 0);

        // ----------------------------------------------------------------------------------------
        // 10. User undelegates and unstakes
        let mut recipient_token_account = MockCompressedTokenAccount { balance: 0 };
        let unstake_amount = 100;
        undelegate_instruction(
            &protocol_config,
            &mut user_stake_account,
            &mut forester_stake_account,
            unstake_amount,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(user_stake_account.pending_stake_weight, unstake_amount);
        assert_eq!(
            forester_stake_account.active_stake_weight,
            900 + protocol_config.epoch_reward - forester_fee
        );

        unstake_instruction(
            &mut user_stake_account,
            &mut user_stake_token_account,
            &mut recipient_token_account,
            protocol_config,
            unstake_amount,
            current_solana_slot,
        )
        .unwrap();
        assert_eq!(user_stake_account.delegated_stake_weight, 90900);
        assert_eq!(user_stake_token_account.balance, 90900);
        assert_eq!(recipient_token_account.balance, unstake_amount);
    }

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
        active_phase_length: u64,
        slot_length: u64,
        epoch_start_slot: u64,
        total_epoch_state_weight: u64,
    ) -> ForesterEpochAccount {
        ForesterEpochAccount {
            forester_config: ForesterConfig::default(),
            epoch: 0,
            stake_weight: forester_stake_weight,
            work_counter: 0,
            has_reported_work: false,
            forester_index: forester_start_range,
            epoch_start_slot,
            total_epoch_state_weight: Some(total_epoch_state_weight),
            protocol_config: ProtocolConfig {
                genesis_slot: 0,
                registration_phase_length: 1,
                active_phase_length,
                report_work_phase_length: 2,
                epoch_reward: 100_000,
                base_reward: 50_000,
                min_stake: 0,
                slot_length,
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
        }

        let sum = eligible.values().map(|x| x.1).sum::<u64>();
        let total_slots: u64 = epoch_len - epoch_start_slot;
        assert_eq!(sum, total_slots);
    }

    #[test]
    fn test_is_epoch() {
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
