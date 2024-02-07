use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{errors::ErrorCode, RegisteredProgram};

pub trait GroupAccess {
    fn get_owner(&self) -> &Pubkey;
    fn get_delegate(&self) -> &Pubkey;
}

pub trait GroupAccounts<'info> {
    fn get_signing_address(&self) -> &Signer<'info>;
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>>;
}

/// if there is a registered program pda check whether the authority is derived from the registered program pda
/// else check whether the authority is the signing address
pub fn check_registered_or_signer<'a, 'b, 'c, 'info, C: GroupAccounts<'info>, A: GroupAccess>(
    ctx: &'a Context<'a, 'b, 'c, 'info, C>,
    checked_account: &'a A,
) -> Result<()> {
    match ctx.accounts.get_registered_program_pda() {
        Some(account) => {
            let derived_address = Pubkey::find_program_address(
                &[ctx.program_id.to_bytes().as_ref()],
                &account.pubkey,
            )
            .0;
            if ctx.accounts.get_signing_address().key() == derived_address
                && derived_address == *checked_account.get_owner()
            {
                Ok(())
            } else {
                Err(ErrorCode::InvalidAuthority.into())
            }
        }
        None => {
            if ctx.accounts.get_signing_address().key() == *checked_account.get_delegate()
                || ctx.accounts.get_signing_address().key() == *checked_account.get_owner()
            {
                Ok(())
            } else {
                Err(ErrorCode::InvalidAuthority.into())
            }
        }
    }
}
