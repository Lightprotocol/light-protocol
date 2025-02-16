use std::default;

use anchor_lang::prelude::*;
use light_account_checks::checks::check_account_balance_is_rent_exempt;

use crate::{
    errors::AccountCompressionErrorCode,
    processor::{
        initialize_concurrent_merkle_tree::process_initialize_state_merkle_tree,
        initialize_nullifier_queue::process_initialize_nullifier_queue,
    },
    state::{QueueAccount, StateMerkleTreeAccount},
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        constants::{
            STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_CHANGELOG, STATE_MERKLE_TREE_HEIGHT,
            STATE_MERKLE_TREE_ROOTS, STATE_NULLIFIER_QUEUE_SEQUENCE_THRESHOLD,
            STATE_NULLIFIER_QUEUE_VALUES,
        },
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct InitializeStateMerkleTreeAndNullifierQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(zero)]
    pub nullifier_queue: AccountLoader<'info, QueueAccount>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct StateMerkleTreeConfig {
    pub height: u32,
    pub changelog_size: u64,
    pub roots_size: u64,
    pub canopy_depth: u64,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
}

impl default::Default for StateMerkleTreeConfig {
    fn default() -> Self {
        Self {
            height: STATE_MERKLE_TREE_HEIGHT as u32,
            changelog_size: STATE_MERKLE_TREE_CHANGELOG,
            roots_size: STATE_MERKLE_TREE_ROOTS,
            canopy_depth: STATE_MERKLE_TREE_CANOPY_DEPTH,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}
impl<'info> GroupAccounts<'info> for InitializeStateMerkleTreeAndNullifierQueue<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct NullifierQueueConfig {
    pub capacity: u16,
    pub sequence_threshold: u64,
    pub network_fee: Option<u64>,
}
// Arbitrary safety margin.
pub const SAFETY_MARGIN: u64 = 10;

impl default::Default for NullifierQueueConfig {
    fn default() -> Self {
        Self {
            capacity: STATE_NULLIFIER_QUEUE_VALUES,
            sequence_threshold: STATE_NULLIFIER_QUEUE_SEQUENCE_THRESHOLD + SAFETY_MARGIN,
            network_fee: None,
        }
    }
}

pub fn process_initialize_state_merkle_tree_and_nullifier_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeStateMerkleTreeAndNullifierQueue<'info>>,
    index: u64,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    _additional_bytes: u64,
) -> Result<()> {
    if state_merkle_tree_config.height as u64 != STATE_MERKLE_TREE_HEIGHT {
        msg!(
            "Unsupported Merkle tree height: {}. The only currently supported height is: {}",
            state_merkle_tree_config.height,
            STATE_MERKLE_TREE_HEIGHT
        );
        return err!(AccountCompressionErrorCode::UnsupportedHeight);
    }
    if state_merkle_tree_config.canopy_depth != STATE_MERKLE_TREE_CANOPY_DEPTH {
        msg!(
            "Unsupported canopy depth: {}. The only currently supported depth is: {}",
            state_merkle_tree_config.canopy_depth,
            STATE_MERKLE_TREE_CANOPY_DEPTH
        );
        return err!(AccountCompressionErrorCode::UnsupportedCanopyDepth);
    }
    if state_merkle_tree_config.close_threshold.is_some() {
        msg!("close_threshold is not supported yet");
        return err!(AccountCompressionErrorCode::UnsupportedCloseThreshold);
    }
    let minimum_sequence_threshold = state_merkle_tree_config.roots_size + SAFETY_MARGIN;
    if nullifier_queue_config.sequence_threshold < minimum_sequence_threshold {
        msg!(
            "Invalid sequence threshold: {}. Should be at least: {}",
            nullifier_queue_config.sequence_threshold,
            minimum_sequence_threshold
        );
        return err!(AccountCompressionErrorCode::InvalidSequenceThreshold);
    }
    let merkle_tree_expected_size = StateMerkleTreeAccount::size(
        state_merkle_tree_config.height as usize,
        state_merkle_tree_config.changelog_size as usize,
        state_merkle_tree_config.roots_size as usize,
        state_merkle_tree_config.canopy_depth as usize,
    );
    let queue_expected_size = QueueAccount::size(nullifier_queue_config.capacity as usize)?;
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        merkle_tree_expected_size,
    )
    .map_err(ProgramError::from)?;
    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.nullifier_queue.to_account_info(),
        queue_expected_size,
    )
    .map_err(ProgramError::from)?;
    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeStateMerkleTreeAndNullifierQueue,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    process_initialize_state_merkle_tree(
        &ctx.accounts.merkle_tree,
        index,
        owner,
        program_owner,
        forester,
        &state_merkle_tree_config.height,
        &state_merkle_tree_config.changelog_size,
        &state_merkle_tree_config.roots_size,
        &state_merkle_tree_config.canopy_depth,
        ctx.accounts.nullifier_queue.key(),
        state_merkle_tree_config.network_fee.unwrap_or(0),
        state_merkle_tree_config.rollover_threshold,
        state_merkle_tree_config.close_threshold,
        merkle_tree_rent,
        queue_rent,
    )?;
    process_initialize_nullifier_queue(
        ctx.accounts.nullifier_queue.to_account_info(),
        &ctx.accounts.nullifier_queue,
        index,
        owner,
        program_owner,
        forester,
        ctx.accounts.merkle_tree.key(),
        nullifier_queue_config.capacity,
        nullifier_queue_config.sequence_threshold,
        state_merkle_tree_config.rollover_threshold,
        state_merkle_tree_config.close_threshold,
        nullifier_queue_config.network_fee.unwrap_or(0),
    )?;
    Ok(())
}
