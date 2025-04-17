use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{context::AcpAccount, errors::AccountCompressionErrorCode, RegisteredProgram};

pub trait GroupAccess {
    fn get_owner(&self) -> Pubkey;
    fn get_program_owner(&self) -> Pubkey;
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
            if ctx.accounts.get_authority().key()
                == registered_program_pda.registered_program_signer_pda
                && checked_account.get_owner().key() == registered_program_pda.group_authority_pda
            {
                Ok(())
            } else {
                msg!("Registered program check failed.");
                msg!("owner address: {:?}", checked_account.get_owner());
                msg!(
                    "derived_address: {:?}",
                    registered_program_pda.registered_program_signer_pda
                );
                msg!("signing_address: {:?}", ctx.accounts.get_authority().key());
                msg!(
                    "registered_program_id: {:?}",
                    registered_program_pda.registered_program_id
                );
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
        None => {
            if ctx.accounts.get_authority().key() == checked_account.get_owner() {
                Ok(())
            } else {
                Err(AccountCompressionErrorCode::InvalidAuthority.into())
            }
        }
    }
}

pub fn manual_check_signer_is_registered_or_authority<'a, A: GroupAccess>(
    derived_address: &Option<(Pubkey, Pubkey)>,
    authority: &AcpAccount<'a, '_>,
    checked_account: &'a A,
) -> std::result::Result<(), AccountCompressionErrorCode> {
    let authority = match authority {
        AcpAccount::Authority(authority) => authority,
        _ => return Err(AccountCompressionErrorCode::InvalidAuthority),
    };
    match derived_address {
        Some((derived_address, group_authority_pda)) => {
            let auth = authority.key() == *derived_address;
            let owner = checked_account.get_owner().key() == *group_authority_pda;
            if auth && owner {
                Ok(())
            } else {
                msg!("Registered program check failed.");
                msg!("owner address: {:?}", checked_account.get_owner());
                msg!(
                    "owner address: {:?}",
                    checked_account.get_owner().to_bytes()
                );
                msg!("derived_address: {:?}", derived_address);
                msg!("signing_address: {:?}", authority.key());
                msg!("group_authority_pda: {:?}", group_authority_pda.to_bytes());
                Err(AccountCompressionErrorCode::InvalidAuthority)
            }
        }
        None => {
            if authority.key() == checked_account.get_owner() {
                Ok(())
            } else {
                Err(AccountCompressionErrorCode::InvalidAuthority)
            }
        }
    }
}
