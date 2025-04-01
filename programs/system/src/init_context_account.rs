use crate::errors::SystemProgramError;
use crate::{
    invoke_cpi::account::CpiContextAccount, LightContext, Result, CPI_CONTEXT_ACCOUNT_DISCRIMINATOR,
};
use borsh::BorshSerialize;
use light_account_checks::checks::check_signer;
use light_account_checks::discriminator::Discriminator;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::constants;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
pub struct InitializeCpiContextAccount<'info> {
    // #[signer]
    pub fee_payer: &'info AccountInfo,
    // #[account(zero)]
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

impl<'info> LightContext<'info> for InitializeCpiContextAccount<'info> {
    fn from_account_infos(accounts: &'info [AccountInfo]) -> Result<(Self, &'info [AccountInfo])> {
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
    let (ctx, _accounts) =
        <InitializeCpiContextAccount<'_> as LightContext<'_>>::from_account_infos(accounts)?;
    let data = ctx.associated_merkle_tree.try_borrow_data()?;
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&data[0..8]);
    match discriminator_bytes {
        constants::STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR => Ok(()),
        BatchedMerkleTreeAccount::DISCRIMINATOR => Ok(()),
        _ => Err(SystemProgramError::AppendStateFailed),
    }
    .map_err(ProgramError::from)?;
    let mut cpi_context_account = CpiContextAccount::default();
    cpi_context_account.init(*ctx.associated_merkle_tree.key());

    cpi_context_account
        .serialize(&mut &mut ctx.cpi_context_account.try_borrow_mut_data()?[8..])
        .unwrap();
    ctx.cpi_context_account.try_borrow_mut_data()?[..8]
        .copy_from_slice(&CPI_CONTEXT_ACCOUNT_DISCRIMINATOR);
    Ok(())
}
