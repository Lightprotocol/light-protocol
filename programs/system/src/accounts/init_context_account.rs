use borsh::BorshSerialize;
use light_account_checks::{
    checks::{account_info_init, check_owner, check_signer},
    discriminator::Discriminator,
};
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::constants::{
    ACCOUNT_COMPRESSION_PROGRAM_ID, STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{errors::SystemProgramError, invoke_cpi::account::CpiContextAccount, Result};
pub struct InitializeCpiContextAccount<'info> {
    pub fee_payer: &'info AccountInfo,
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

impl<'info> InitializeCpiContextAccount<'info> {
    pub fn from_account_infos(accounts: &'info [AccountInfo]) -> Result<Self> {
        if accounts.len() < 3 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let fee_payer = &accounts[0];
        check_signer(&accounts[0]).map_err(ProgramError::from)?;

        let cpi_context_account = &accounts[1];
        let associated_merkle_tree = &accounts[2];
        check_owner(&ACCOUNT_COMPRESSION_PROGRAM_ID, associated_merkle_tree)?;
        let mut discriminator_bytes = [0u8; 8];
        let data = associated_merkle_tree.try_borrow_data()?;
        discriminator_bytes.copy_from_slice(&data[0..8]);

        match discriminator_bytes {
            STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR => Ok(()),
            BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR => Ok(()),
            _ => Err(SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch),
        }
        .map_err(ProgramError::from)?;

        Ok(Self {
            fee_payer,
            cpi_context_account,
            associated_merkle_tree,
        })
    }
}

pub fn init_cpi_context_account(accounts: &[AccountInfo]) -> Result<()> {
    // Check that Merkle tree is initialized.
    let ctx = InitializeCpiContextAccount::from_account_infos(accounts)?;

    // 1. Check discriminator bytes are zeroed.
    // 2. Set discriminator.
    account_info_init::<CpiContextAccount, AccountInfo>(ctx.cpi_context_account)?;

    let mut cpi_context_account_data = ctx.cpi_context_account.try_borrow_mut_data()?;
    let cpi_context_account = CpiContextAccount {
        associated_merkle_tree: *ctx.associated_merkle_tree.key(),
        ..Default::default()
    };
    // Initialize account with data.
    cpi_context_account
        .serialize(&mut &mut cpi_context_account_data[8..])
        .unwrap();

    Ok(())
}
