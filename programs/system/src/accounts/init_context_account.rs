use borsh::BorshSerialize;
use light_account_checks::{
    checks::{check_data_is_zeroed, check_signer},
    discriminator::Discriminator,
};
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::constants;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    errors::SystemProgramError, invoke_cpi::account::CpiContextAccount, Result,
    CPI_CONTEXT_ACCOUNT_DISCRIMINATOR,
};
pub struct InitializeCpiContextAccount<'info> {
    // #[signer]
    pub fee_payer: &'info AccountInfo,
    // #[account(zero)]
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

impl<'info> InitializeCpiContextAccount<'info> {
    pub fn from_account_infos(
        accounts: &'info [AccountInfo],
    ) -> Result<(Self, &'info [AccountInfo])> {
        if accounts.len() < 3 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let fee_payer = &accounts[0];
        let cpi_context_account = &accounts[1];
        let associated_merkle_tree = &accounts[2];
        check_signer(&accounts[0]).map_err(ProgramError::from)?;

        // TODO: replicate anchor [zero] macro check
        // check_is_empty(cpi_context_account)?;

        Ok((
            Self {
                fee_payer,
                cpi_context_account,
                associated_merkle_tree,
            },
            &accounts[3..],
        ))
    }
}

pub fn init_cpi_context_account(accounts: &[AccountInfo], _instruction_data: &[u8]) -> Result<()> {
    // Check that Merkle tree is initialized.
    let (ctx, _accounts) = InitializeCpiContextAccount::from_account_infos(accounts)?;
    let data = ctx.associated_merkle_tree.try_borrow_data()?;
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&data[0..8]);
    match discriminator_bytes {
        constants::STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR => Ok(()),
        BatchedMerkleTreeAccount::DISCRIMINATOR => Ok(()),
        _ => Err(SystemProgramError::AppendStateFailed),
    }
    .map_err(ProgramError::from)?;

    let mut cpi_context_account_data = ctx.cpi_context_account.try_borrow_mut_data()?;

    // Check account is not initialized.
    check_data_is_zeroed(&cpi_context_account_data[0..8]).map_err(ProgramError::from)?;
    // Initialize account with discriminator.
    cpi_context_account_data[..8].copy_from_slice(&CPI_CONTEXT_ACCOUNT_DISCRIMINATOR);

    let mut cpi_context_account = CpiContextAccount::default();
    cpi_context_account.init(*ctx.associated_merkle_tree.key());
    // Initialize account with data.
    cpi_context_account
        .serialize(&mut &mut cpi_context_account_data[8..])
        .unwrap();
    Ok(())
}
