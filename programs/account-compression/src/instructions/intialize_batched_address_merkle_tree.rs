use anchor_lang::prelude::*;
use light_batched_merkle_tree::{
    initialize_address_tree::{
        init_batched_address_merkle_tree_from_account_info, validate_batched_address_tree_params,
        InitAddressTreeAccountsInstructionData,
    },
    merkle_tree::BatchedMerkleTreeAccount,
};

use crate::{
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct InitializeBatchedAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub merkle_tree: AccountInfo<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeBatchedAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// 1. checks signer
/// 2. initializes merkle tree
pub fn process_initialize_batched_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeBatchedAddressMerkleTree<'info>>,
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

    // Check signer.
    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeBatchedAddressMerkleTree,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };

    // Initialize merkle tree.
    let mt_account_info = ctx.accounts.merkle_tree.to_account_info();
    init_batched_address_merkle_tree_from_account_info(params, owner.into(), &mt_account_info)
        .map_err(ProgramError::from)?;
    Ok(())
}

impl GroupAccess for BatchedMerkleTreeAccount<'_> {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata.access_metadata.program_owner.into()
    }
}
