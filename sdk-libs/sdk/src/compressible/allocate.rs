#[cfg(feature = "anchor")]
use anchor_lang::{
    prelude::CpiContext, system_program::create_account, system_program::CreateAccount, Result,
};
use light_account_checks::AccountInfoTrait;
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use solana_rent::Rent;
use solana_sysvar::Sysvar;

#[cfg(feature = "anchor")]
pub fn create_or_allocate_account<'a>(
    program_id: &Pubkey,
    payer: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    target_account: AccountInfo<'a>,
    signer_seed: &[&[u8]],
    space: usize,
) -> Result<()> {
    let rent = Rent::get()?;
    let current_lamports = target_account.lamports();
    use solana_program::msg;
    msg!("current_lamports, {}", current_lamports);
    msg!("target_account, {:?}", target_account.pubkey());
    if current_lamports == 0 {
        let lamports = rent.minimum_balance(space);
        anchor_lang::prelude::msg!("payer {:?}", payer.key());
        anchor_lang::prelude::msg!("target_account {:?}", target_account.key());
        let cpi_accounts = CreateAccount {
            from: payer,
            to: target_account.clone(),
        };
        let cpi_context = CpiContext::new(system_program.clone(), cpi_accounts);

        create_account(
            cpi_context.with_signer(&[signer_seed]),
            lamports,
            u64::try_from(space).unwrap(),
            program_id,
        )?;
    } else {
        use anchor_lang::{
            prelude::CpiContext,
            system_program::{allocate, assign, Allocate, Assign, AssignBumps},
        };

        let required_lamports = rent
            .minimum_balance(space)
            .max(1)
            .saturating_sub(current_lamports);
        if required_lamports > 0 {
            use anchor_lang::{
                prelude::CpiContext,
                system_program::{transfer, Transfer},
                ToAccountInfo,
            };

            let cpi_accounts = Transfer {
                from: payer.to_account_info(),
                to: target_account.clone(),
            };
            let cpi_context = CpiContext::new(system_program.clone(), cpi_accounts);
            transfer(cpi_context, required_lamports)?;
        }
        let cpi_accounts = Allocate {
            account_to_allocate: target_account.clone(),
        };
        let cpi_context = CpiContext::new(system_program.clone(), cpi_accounts);
        allocate(
            cpi_context.with_signer(&[signer_seed]),
            u64::try_from(space).unwrap(),
        )?;

        let cpi_accounts = Assign {
            account_to_assign: target_account.clone(),
        };
        let cpi_context = CpiContext::new(system_program.clone(), cpi_accounts);
        assign(cpi_context.with_signer(&[signer_seed]), program_id)?;
    }
    Ok(())
}
