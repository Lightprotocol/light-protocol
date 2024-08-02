use crate::{
    delegate::{
        delegate_account::CompressedAccountTrait,
        process_cpi::{
            cpi_compressed_token_mint_to, cpi_light_system_program, mint_spl_to_pool_pda,
        },
        FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
    },
    errors::RegistryError,
    forester::state::ForesterAccount,
};
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_compressed_token::process_transfer::get_cpi_signer_seeds;
use light_hasher::Hasher;
use light_system_program::{
    sdk::compressed_account::{CompressedAccount, CompressedAccountData},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use super::{
    claim_forester_instruction::ClaimForesterInstruction,
    register_epoch::{EpochPda, ForesterEpochPda},
};

// TODO: make sure that performance based rewards can only be claimed if work has been reported
// TODO: add reimbursement for opening the epoch account (close an one epoch account to open a new one of X epochs ago)
/// Forester claim rewards:
/// 1. Transfer forester fees to foresters compressed token account
/// 2. Transfer rewards to foresters token account
/// 3. compress forester epoch account
/// 4. close forester epoch account (in instruction struct)
/// 5. (skipped) if all stake has claimed close epoch account
pub fn process_forester_claim_rewards<'info>(
    ctx: Context<'_, '_, '_, 'info, ClaimForesterInstruction<'info>>,
) -> Result<()> {
    let forester_pda_pubkey = ctx.accounts.forester_pda.key();

    let current_slot = Clock::get()?.slot;
    let (epoch_results_compressed_account, fee, net_reward) = forester_claim_rewards(
        &mut ctx.accounts.forester_pda,
        &ctx.accounts.forester_epoch_pda,
        &ctx.accounts.epoch_pda,
        current_slot,
        &forester_pda_pubkey,
        0,
    )?;
    // Mint netrewards to forester pool. These rewards can be claimed by the
    // delegates.
    mint_spl_to_pool_pda(
        &ctx,
        net_reward,
        ctx.accounts.forester_token_pool.to_account_info(),
        get_cpi_signer_seeds(),
    )?;

    cpi_light_system_program(
        &ctx,
        None,
        None,
        None,
        epoch_results_compressed_account,
        vec![ctx.accounts.output_merkle_tree.to_account_info()],
    )?;
    // Mint forester fee
    cpi_compressed_token_mint_to(
        &ctx,
        vec![ctx.accounts.forester_pda.config.fee_recipient],
        vec![fee],
        get_cpi_signer_seeds(),
        ctx.accounts.output_merkle_tree.to_account_info(),
    )
}

#[aligned_sized]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedForesterEpochAccount {
    pub rewards_earned: u64,
    pub epoch: u64,
    pub stake_weight: u64,
    pub previous_hash: [u8; 32],
    pub forester_pda_pubkey: Pubkey,
}
impl CompressedAccountTrait for CompressedForesterEpochAccount {
    fn get_owner(&self) -> Pubkey {
        self.forester_pda_pubkey
    }
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

    pub fn get_reward(&self, stake: u64) -> Result<u64> {
        Ok(self
            .rewards_earned
            .checked_mul(stake)
            .ok_or(RegistryError::ArithmeticOverflow)?
            .checked_div(self.stake_weight)
            .ok_or(RegistryError::ArithmeticUnderflow)?)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedForesterEpochAccountInput {
    pub rewards_earned: u64,
    pub epoch: u64,
    pub stake_weight: u64,
}

impl CompressedForesterEpochAccountInput {
    pub fn into_compressed_forester_epoch_pda(
        self,
        previous_hash: [u8; 32],
        forester_pda_pubkey: Pubkey,
    ) -> CompressedForesterEpochAccount {
        CompressedForesterEpochAccount {
            rewards_earned: self.rewards_earned,
            epoch: self.epoch,
            stake_weight: self.stake_weight,
            previous_hash,
            forester_pda_pubkey,
        }
    }
}

pub fn serialize_compressed_forester_epoch_account(
    epoch_results_compressed_account: CompressedForesterEpochAccount,
    merkle_tree_index: u8,
    hashed_forester_pubkey: [u8; 32],
) -> Result<OutputCompressedAccountWithPackedContext> {
    let data_hash = epoch_results_compressed_account.hash(hashed_forester_pubkey)?;
    let mut data = Vec::with_capacity(CompressedForesterEpochAccount::LEN);
    epoch_results_compressed_account.serialize(&mut data)?;
    let compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0,
        address: None,
        data: Some(CompressedAccountData {
            discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
            data_hash,
            data,
        }),
    };
    let epoch_results_compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index,
    };
    Ok(epoch_results_compressed_account)
}

/// Instruction checks that:
/// 1. epoch and forester epoch are in the same epoch
/// 2. forester account and forester epoch pda are related
pub fn forester_claim_rewards(
    forester_pda: &mut ForesterAccount,
    forester_epoch_pda: &ForesterEpochPda,
    epoch_pda: &EpochPda,
    current_slot: u64,
    forester_pda_pubkey: &Pubkey,
    merkle_tree_index: u8,
) -> Result<(OutputCompressedAccountWithPackedContext, u64, u64)> {
    forester_pda.sync(current_slot, &epoch_pda.protocol_config)?;
    epoch_pda
        .protocol_config
        .is_post_epoch(current_slot, forester_epoch_pda.epoch)?;

    let total_stake_weight = epoch_pda.registered_stake;
    let total_tally = epoch_pda.total_work;
    let forester_stake_weight = forester_epoch_pda.stake_weight;
    let forester_tally = forester_epoch_pda.work_counter;
    msg!("epoch_pda: {:?}", epoch_pda);
    msg!("forester_epoch_pda: {:?}", forester_epoch_pda);
    let reward = epoch_pda.protocol_config.get_rewards(
        total_stake_weight,
        total_tally,
        forester_stake_weight,
        forester_tally,
    );
    msg!("reward: {}", reward);
    let fee = reward
        .checked_mul(forester_pda.config.fee)
        .ok_or(RegistryError::ArithmeticOverflow)?
        .checked_div(100)
        .ok_or(RegistryError::ArithmeticUnderflow)?;
    msg!("fee: {}", fee);
    let net_reward = reward
        .checked_sub(fee)
        .ok_or(RegistryError::ArithmeticUnderflow)?;
    msg!("net_reward: {}", net_reward);
    // Increase the active deleagted stake weight by the net_reward
    forester_pda.active_stake_weight = forester_pda
        .active_stake_weight
        .checked_add(net_reward)
        .ok_or(RegistryError::ArithmeticOverflow)?;
    let epoch_results_compressed_account = CompressedForesterEpochAccount {
        rewards_earned: net_reward,
        epoch: forester_epoch_pda.epoch,
        stake_weight: forester_epoch_pda.stake_weight,
        previous_hash: forester_pda.last_compressed_forester_epoch_pda_hash,
        forester_pda_pubkey: *forester_pda_pubkey,
    };
    let hashed_forester_pda_pubkey = hash_to_bn254_field_size_be(forester_pda_pubkey.as_ref())
        .unwrap()
        .0;
    let epoch_results_compressed_account = serialize_compressed_forester_epoch_account(
        epoch_results_compressed_account,
        merkle_tree_index,
        hashed_forester_pda_pubkey,
    )?;
    forester_pda.last_compressed_forester_epoch_pda_hash = epoch_results_compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data_hash;
    forester_pda.last_claimed_epoch = forester_epoch_pda.epoch;
    Ok((epoch_results_compressed_account, fee, net_reward))
}

#[cfg(test)]
mod tests {
    use crate::{protocol_config::state::ProtocolConfig, ForesterConfig};

    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;

    fn get_test_data() -> (CompressedForesterEpochAccount, u64, u8, [u8; 32]) {
        let test_epoch_account = CompressedForesterEpochAccount {
            rewards_earned: 1000,
            epoch: 1,
            stake_weight: 100,
            previous_hash: [0; 32],
            forester_pda_pubkey: Pubkey::default(),
        };
        let fee = 10;
        let merkle_tree_index = 1;
        let hashed_forester_pubkey = [0; 32];

        (
            test_epoch_account,
            fee,
            merkle_tree_index,
            hashed_forester_pubkey,
        )
    }

    #[test]
    fn test_serialize_compressed_forester_epoch_account() {
        let (epoch_account, _fee, merkle_tree_index, hashed_forester_pubkey) = get_test_data();

        let result = serialize_compressed_forester_epoch_account(
            epoch_account,
            merkle_tree_index,
            hashed_forester_pubkey,
        );

        assert!(result.is_ok());
        let serialized_account = result.unwrap();

        let expected_compressed_account = CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: None,
            data: Some(CompressedAccountData {
                discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
                data_hash: epoch_account.hash(hashed_forester_pubkey).unwrap(),
                data: epoch_account.try_to_vec().unwrap(),
            }),
        };

        let expected_output_compressed_account_with_packed_context =
            OutputCompressedAccountWithPackedContext {
                compressed_account: expected_compressed_account,
                merkle_tree_index,
            };

        assert_eq!(
            expected_output_compressed_account_with_packed_context,
            serialized_account
        );
    }
    fn get_test_forester_claim_rewards_test_data(
    ) -> (ForesterAccount, ForesterEpochPda, EpochPda, u64, Pubkey, u8) {
        let active_stake = 100;
        let forester_pda = ForesterAccount {
            active_stake_weight: active_stake,
            config: ForesterConfig {
                fee: 5,
                fee_recipient: Pubkey::default(),
            },
            last_compressed_forester_epoch_pda_hash: [2; 32],
            ..Default::default()
        };

        let epoch_pda = EpochPda {
            registered_stake: active_stake,
            total_work: 100,
            protocol_config: ProtocolConfig {
                active_phase_length: 40,
                genesis_slot: 0,
                registration_phase_length: 10,
                report_work_phase_length: 10,
                base_reward: 20,
                slot_length: 1,
                epoch_reward: 100,
                ..Default::default()
            },
            epoch: 1,
            ..Default::default()
        };
        let forester_epoch_pda = ForesterEpochPda {
            epoch: 1,
            stake_weight: active_stake,
            work_counter: 100,
            protocol_config: epoch_pda.protocol_config,
            ..Default::default()
        };

        let current_slot = 100;
        let forester_pda_pubkey = Pubkey::default();
        let merkle_tree_index = 1;

        (
            forester_pda,
            forester_epoch_pda,
            epoch_pda,
            current_slot,
            forester_pda_pubkey,
            merkle_tree_index,
        )
    }
    #[test]
    fn test_forester_claim_rewards_failing() {
        let (
            mut forester_pda,
            forester_epoch_pda,
            epoch_pda,
            _,
            forester_pda_pubkey,
            merkle_tree_index,
        ) = get_test_forester_claim_rewards_test_data();
        // Set current slot so that the epoch is still ongoing
        let current_slot = epoch_pda.protocol_config.genesis_slot
            + epoch_pda.protocol_config.registration_phase_length
            + epoch_pda.protocol_config.active_phase_length
            + 1;
        let result = forester_claim_rewards(
            &mut forester_pda,
            &forester_epoch_pda,
            &epoch_pda,
            current_slot,
            &forester_pda_pubkey,
            merkle_tree_index,
        );
        assert!(matches!(result, Err(error) if error == RegistryError::InvalidEpoch.into()));
    }

    #[test]
    fn test_forester_claim_rewards() {
        let (
            mut forester_pda,
            forester_epoch_pda,
            epoch_pda,
            current_slot,
            forester_pda_pubkey,
            merkle_tree_index,
        ) = get_test_forester_claim_rewards_test_data();
        let mut pre_forester_pda = forester_pda.clone();
        let result = forester_claim_rewards(
            &mut forester_pda,
            &forester_epoch_pda,
            &epoch_pda,
            current_slot,
            &forester_pda_pubkey,
            merkle_tree_index,
        );

        assert!(result.is_ok());
        let (compressed_account, fee, net_reward) = result.unwrap();

        let expected_compressed_account = CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: None,
            data: Some(CompressedAccountData {
                discriminator: FORESTER_EPOCH_RESULT_ACCOUNT_DISCRIMINATOR,
                data_hash: forester_pda.last_compressed_forester_epoch_pda_hash,
                data: CompressedForesterEpochAccount {
                    rewards_earned: net_reward,
                    epoch: forester_epoch_pda.epoch,
                    stake_weight: forester_epoch_pda.stake_weight,
                    previous_hash: pre_forester_pda.last_compressed_forester_epoch_pda_hash,
                    forester_pda_pubkey,
                }
                .try_to_vec()
                .unwrap(),
            }),
        };

        let expected_output_compressed_account_with_packed_context =
            OutputCompressedAccountWithPackedContext {
                compressed_account: expected_compressed_account,
                merkle_tree_index,
            };

        assert_eq!(
            compressed_account,
            expected_output_compressed_account_with_packed_context,
        );
        pre_forester_pda.active_stake_weight += net_reward;
        pre_forester_pda.last_compressed_forester_epoch_pda_hash =
            expected_output_compressed_account_with_packed_context
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data_hash;
        pre_forester_pda.last_claimed_epoch = forester_epoch_pda.epoch;
        pre_forester_pda.current_epoch = epoch_pda
            .protocol_config
            .get_current_registration_epoch(current_slot);

        assert_eq!(fee, 5); // 100 * 0.05
        assert_eq!(net_reward, 95); // 100 - 5
        assert_eq!(forester_pda, pre_forester_pda);
    }
}
