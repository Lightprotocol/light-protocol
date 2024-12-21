use anchor_lang::prelude::*;

use crate::errors::AccountCompressionErrorCode;

pub fn transfer_lamports<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let compressed_sol_pda_lamports = from.lamports();

    **from.as_ref().try_borrow_mut_lamports()? =
        match compressed_sol_pda_lamports.checked_sub(lamports) {
            Some(lamports) => lamports,
            None => return err!(AccountCompressionErrorCode::IntegerOverflow),
        };
    let recipient_lamports = to.lamports();
    **to.as_ref().try_borrow_mut_lamports()? = match recipient_lamports.checked_add(lamports) {
        Some(lamports) => lamports,
        None => return err!(AccountCompressionErrorCode::IntegerOverflow),
    };
    Ok(())
}

pub fn transfer_lamports_cpi<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let instruction =
        anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
    anchor_lang::solana_program::program::invoke(&instruction, &[from.clone(), to.clone()])?;
    Ok(())
}
