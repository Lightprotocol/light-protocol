use crate::{errors::AccountCompressionErrorCode, RegisteredProgram};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use super::constants::CPI_AUTHORITY_PDA_SEED;

pub trait GroupAccess {
    fn get_owner(&self) -> &Pubkey;
    fn get_program_owner(&self) -> &Pubkey;
}

pub trait GroupAccounts<'info> {
    fn get_authority(&self) -> &Signer<'info>;
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>>;
}

/// if there is a registered program pda check whether the authority is derived from the registered program pda
/// else check whether the authority is the signing address
pub fn check_signer_is_registered_or_authority<
    'a,
    'b,
    'c,
    'info,
    C: GroupAccounts<'info> + anchor_lang::Bumps,
    A: GroupAccess,
>(
    ctx: &'a Context<'a, 'b, 'c, 'info, C>,
    checked_account: &'a A,
) -> Result<()> {
    match ctx.accounts.get_registered_program_pda() {
        Some(registered_program_pda) => {
            let derived_address = Pubkey::find_program_address(
                &[CPI_AUTHORITY_PDA_SEED],
                &registered_program_pda.registered_program_id,
            )
            .0;
            if ctx.accounts.get_authority().key() == derived_address
                && checked_account.get_owner().key() == registered_program_pda.group_authority_pda
            {
                Ok(())
            } else {
                msg!("Registered program check failed.");
                msg!("owner address: {:?}", checked_account.get_owner());
                msg!("derived_address: {:?}", derived_address);
                msg!("signing_address: {:?}", ctx.accounts.get_authority().key());
                msg!(
                    "registered_program_id: {:?}",
                    registered_program_pda.registered_program_id
                );
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
        None => {
            if ctx.accounts.get_authority().key() == *checked_account.get_owner() {
                Ok(())
            } else {
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
    }
}
