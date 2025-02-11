use anchor_lang::prelude::*;
use light_account_checks::checks::check_account_balance_is_rent_exempt;

use crate::{
    errors::AccountCompressionErrorCode,
    processor::{
        initialize_address_merkle_tree::process_initialize_address_merkle_tree,
        initialize_address_queue::process_initialize_address_queue,
    },
    state::QueueAccount,
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
        },
        constants::{
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_CHANGELOG,
            ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
            ADDRESS_MERKLE_TREE_ROOTS,
        },
    },
    AddressMerkleTreeAccount, NullifierQueueConfig, RegisteredProgram, SAFETY_MARGIN,
};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct AddressMerkleTreeConfig {
    pub height: u32,
    pub changelog_size: u64,
    pub roots_size: u64,
    pub canopy_depth: u64,
    pub address_changelog_size: u64,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
}

impl Default for AddressMerkleTreeConfig {
    fn default() -> Self {
        Self {
            height: ADDRESS_MERKLE_TREE_HEIGHT as u32,
            changelog_size: ADDRESS_MERKLE_TREE_CHANGELOG,
            roots_size: ADDRESS_MERKLE_TREE_ROOTS,
            canopy_depth: ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
            address_changelog_size: ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

pub type AddressQueueConfig = NullifierQueueConfig;

#[derive(Accounts)]
pub struct InitializeAddressMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    #[account(zero)]
    pub queue: AccountLoader<'info, QueueAccount>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeAddressMerkleTreeAndQueue<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl GroupAccess for RegisteredProgram {
    fn get_owner(&self) -> Pubkey {
        self.group_authority_pda
    }
    fn get_program_owner(&self) -> Pubkey {
        self.registered_program_id
    }
}

pub fn process_initialize_address_merkle_tree_and_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTreeAndQueue<'info>>,
    index: u64,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: AddressMerkleTreeConfig,
    queue_config: AddressQueueConfig,
) -> Result<()> {
    if merkle_tree_config.height as u64 != ADDRESS_MERKLE_TREE_HEIGHT {
        msg!(
            "Unsupported Merkle tree height: {}. The only currently supported height is: {}",
            merkle_tree_config.height,
            ADDRESS_MERKLE_TREE_HEIGHT
        );
        return err!(AccountCompressionErrorCode::UnsupportedHeight);
    }
    if merkle_tree_config.canopy_depth != ADDRESS_MERKLE_TREE_CANOPY_DEPTH {
        msg!(
            "Unsupported canopy depth: {}. The only currently supported depth is: {}",
            merkle_tree_config.canopy_depth,
            ADDRESS_MERKLE_TREE_CANOPY_DEPTH
        );
        return err!(AccountCompressionErrorCode::UnsupportedCanopyDepth);
    }
    if merkle_tree_config.close_threshold.is_some() {
        msg!("close_threshold is not supported yet");
        return err!(AccountCompressionErrorCode::UnsupportedCloseThreshold);
    }
    let minimum_sequence_threshold = merkle_tree_config.roots_size + SAFETY_MARGIN;
    if queue_config.sequence_threshold < minimum_sequence_threshold {
        msg!(
            "Invalid sequence threshold: {}. Should be at least {}",
            queue_config.sequence_threshold,
            minimum_sequence_threshold
        );
        return err!(AccountCompressionErrorCode::InvalidSequenceThreshold);
    }
    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeAddressMerkleTreeAndQueue,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    let merkle_tree_expected_size = AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );
    let queue_expected_size = QueueAccount::size(queue_config.capacity as usize)?;
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        merkle_tree_expected_size,
    )
    .map_err(ProgramError::from)?;
    check_account_balance_is_rent_exempt(
        &ctx.accounts.queue.to_account_info(),
        queue_expected_size,
    )
    .map_err(ProgramError::from)?;
    process_initialize_address_queue(
        &ctx.accounts.queue.to_account_info(),
        &ctx.accounts.queue,
        index,
        owner,
        program_owner,
        forester,
        ctx.accounts.merkle_tree.key(),
        queue_config.capacity,
        queue_config.sequence_threshold,
        queue_config.network_fee.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
        merkle_tree_config.height,
        merkle_tree_rent,
    )?;
    process_initialize_address_merkle_tree(
        &ctx.accounts.merkle_tree,
        index,
        owner,
        program_owner,
        forester,
        merkle_tree_config.height,
        merkle_tree_config.changelog_size,
        merkle_tree_config.roots_size,
        merkle_tree_config.canopy_depth,
        merkle_tree_config.address_changelog_size,
        ctx.accounts.queue.key(),
        merkle_tree_config.network_fee.unwrap_or_default(),
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
    )
}
