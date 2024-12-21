use anchor_lang::prelude::*;
use light_batched_merkle_tree::{
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, validate_batched_address_tree_params,
        InitAddressTreeAccountsInstructionData,
    },
    merkle_tree::{get_merkle_tree_account_size, BatchedMerkleTreeAccount},
    zero_copy::check_account_info_init,
};

use crate::{
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
        },
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct InitializeBatchAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub merkle_tree: AccountInfo<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeBatchAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_initialize_batched_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeBatchAddressMerkleTree<'info>>,
    params: InitAddressTreeAccountsInstructionData,
) -> Result<()> {
    #[cfg(feature = "test")]
    validate_batched_address_tree_params(params);
    #[cfg(not(feature = "test"))]
    {
        if params != InitAddressTreeAccountsInstructionData::default() {
            return err!(AccountCompressionErrorCode::UnsupportedParameters);
        }
    }

    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeBatchAddressMerkleTree,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    );

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        mt_account_size,
    )?;

    let mt_account_info = ctx.accounts.merkle_tree.to_account_info();
    check_account_info_init::<BatchedMerkleTreeAccount>(crate::ID, &mt_account_info)
        .map_err(ProgramError::from)?;
    let mt_data = &mut mt_account_info.try_borrow_mut_data()?;

    init_batched_address_merkle_tree_account(owner, params, mt_data, merkle_tree_rent)
        .map_err(ProgramError::from)?;

    Ok(())
}

impl GroupAccess for BatchedMerkleTreeAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.get_metadata().metadata.access_metadata.owner
    }

    fn get_program_owner(&self) -> &Pubkey {
        &self.get_metadata().metadata.access_metadata.program_owner
    }
}
