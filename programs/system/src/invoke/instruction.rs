use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    // processor::sol_compression::SOL_POOL_PDA_SEED,
    LightContext,
    Result,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
// #[derive(Accounts)]
pub struct InvokeInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    // #[account(mut)]
    pub fee_payer: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    /// CHECK: this account
    pub registered_program_pda: &'info AccountInfo,
    /// CHECK: is checked when emitting the event.
    /// Unused legacy.
    pub noop_program: &'info AccountInfo,
    /// CHECK: this account in account compression program.
    /// This pda is used to invoke the account compression program.
    pub account_compression_authority: &'info AccountInfo,
    /// CHECK: Account compression program is used to update state and address
    /// Merkle trees.
    pub account_compression_program: &'info AccountInfo,
    /// Sol pool pda is used to store the native sol that has been compressed.
    /// It's only required when compressing or decompressing sol.
    // #[account(
    //     mut,
    //     seeds = [SOL_POOL_PDA_SEED], bump
    // )]
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// Only needs to be provided for decompression as a recipient for the
    /// decompressed sol.
    /// Compressed sol originate from authority.
    // #[account(mut)]
    pub decompression_recipient: Option<&'info AccountInfo>,
    pub system_program: &'info AccountInfo,
}
use light_account_checks::checks::{check_owner, check_program, check_signer};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;

impl<'info> LightContext<'info> for InvokeInstruction<'info> {
    fn from_account_infos(accounts: &'info [AccountInfo]) -> Result<(Self, &[AccountInfo])> {
        let fee_payer = &accounts[0];
        check_signer(fee_payer).map_err(ProgramError::from)?;
        let authority = &accounts[1];
        check_signer(authority).map_err(ProgramError::from)?;
        let registered_program_pda = &accounts[2];
        let noop_program = &accounts[3];
        let account_compression_authority = &accounts[4];
        let account_compression_program = &accounts[5];
        check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
            .map_err(ProgramError::from)?;
        let option_sol_pool_pda = &accounts[6];
        let sol_pool_pda = if *option_sol_pool_pda.key() == crate::ID {
            None
        } else {
            // TODO: add checks
            Some(option_sol_pool_pda)
        };
        let option_decompression_recipient = &accounts[7];
        let decompression_recipient = if *option_decompression_recipient.key() == crate::ID {
            None
        } else {
            // TODO: add checks
            Some(option_decompression_recipient)
        };
        let system_program = &accounts[8];
        check_program(&Pubkey::default(), system_program).map_err(ProgramError::from)?;
        Ok((
            Self {
                fee_payer,
                authority,
                registered_program_pda,
                noop_program,
                account_compression_authority,
                account_compression_program,
                sol_pool_pda,
                decompression_recipient,
                system_program,
            },
            &accounts[9..],
        ))
    }
}

impl<'info> SignerAccounts<'info> for InvokeInstruction<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo {
        &self.fee_payer
    }

    fn get_authority(&self) -> &'info AccountInfo {
        &self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeInstruction<'info> {
    fn get_registered_program_pda(&self) -> &'info AccountInfo {
        &self.registered_program_pda
    }

    fn get_noop_program(&self) -> &'info AccountInfo {
        &self.noop_program
    }

    fn get_account_compression_authority(&self) -> &'info AccountInfo {
        &self.account_compression_authority
    }

    fn get_account_compression_program(&self) -> &'info AccountInfo {
        &self.account_compression_program
    }

    fn get_system_program(&self) -> &'info AccountInfo {
        &self.system_program
    }

    fn get_sol_pool_pda(&self) -> Option<&'info AccountInfo> {
        self.sol_pool_pda
    }

    fn get_decompression_recipient(&self) -> Option<&'info AccountInfo> {
        self.decompression_recipient
    }
}
