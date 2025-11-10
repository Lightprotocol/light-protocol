use borsh::BorshDeserialize;
use light_account_checks::{
    checks::{check_owner, check_signer},
    discriminator::Discriminator,
};
use light_batched_merkle_tree::{
    constants::DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2, merkle_tree::BatchedMerkleTreeAccount,
};
use light_compressed_account::constants::{
    ACCOUNT_COMPRESSION_PROGRAM_ID, STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR,
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    cpi_context::state::{cpi_context_account_new, CpiContextAccount, CpiContextAccountInitParams},
    errors::SystemProgramError,
    Result,
};
pub struct InitializeCpiContextAccount<'info> {
    pub fee_payer: &'info AccountInfo,
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

impl<'info> InitializeCpiContextAccount<'info> {
    #[profile]
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

#[profile]
pub fn init_cpi_context_account(accounts: &[AccountInfo]) -> Result<()> {
    // Check that Merkle tree is initialized.
    let ctx = InitializeCpiContextAccount::from_account_infos(accounts)?;
    let params: CpiContextAccountInitParams =
        CpiContextAccountInitParams::new(*ctx.associated_merkle_tree.key());
    cpi_context_account_new::<false>(ctx.cpi_context_account, params)?;

    Ok(())
}

#[profile]
pub fn reinit_cpi_context_account(accounts: &[AccountInfo]) -> Result<()> {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let cpi_context_account = &accounts[0];

    // Check owner before realloc
    check_owner(&crate::ID, cpi_context_account)?;

    // Read associated_merkle_tree BEFORE resizing (in case resize truncates data)
    let associated_merkle_tree = {
        let data = cpi_context_account.try_borrow_data()?;
        CpiContextAccount::deserialize(&mut &data[8..])
            .map_err(|_| ProgramError::BorshIoError)?
            .associated_merkle_tree
    };

    // Realloc account to new size (14020 bytes)
    cpi_context_account.resize(DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2 as usize)?;

    let params: CpiContextAccountInitParams =
        CpiContextAccountInitParams::new(associated_merkle_tree);
    cpi_context_account_new::<true>(cpi_context_account, params)?;

    Ok(())
}
