use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{errors::AccountCompressionErrorCode, RegisteredProgram};

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
pub fn check_registered_or_signer<
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
        Some(account) => {
            let derived_address =
                Pubkey::find_program_address(&[b"cpi_authority"], &account.pubkey).0;
            if ctx.accounts.get_signing_address().key() == derived_address {
                Ok(())
            } else {
                msg!("registered program check failed");
                msg!("derived_address: {:?}", account.key());
                msg!(
                    "signing_address: {:?}",
                    ctx.accounts.get_signing_address().key()
                );
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
        None => {
            if ctx.accounts.get_signing_address().key() == *checked_account.get_delegate()
                || ctx.accounts.get_signing_address().key() == *checked_account.get_owner()
            {
                Ok(())
            } else {
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
    }
}
