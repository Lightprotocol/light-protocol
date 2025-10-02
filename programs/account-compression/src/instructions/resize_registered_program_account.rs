use aligned_sized::aligned_sized;
use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke, system_instruction::transfer},
    Discriminator,
};
use bytemuck::from_bytes_mut;

use crate::{utils::constants::CPI_AUTHORITY_PDA_SEED, RegisteredProgram};

#[repr(C)]
#[derive(Debug, Copy)]
#[account]
#[aligned_sized(anchor)]
pub struct RegisteredProgramV1 {
    pub registered_program_id: Pubkey,
    pub group_authority_pda: Pubkey,
}

#[derive(Accounts)]
pub struct ResizeRegisteredProgramPda<'info> {
    /// CHECK: unchecked since any RegisteredProgramV1 must be resized to continue operation.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: only V1 accounts should be resized. We can detect V1 accounts based on the data length.
    #[account(mut, constraint= registered_program_pda.to_account_info().data_len() == RegisteredProgramV1::LEN)]
    pub registered_program_pda: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_resize_registered_program_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, ResizeRegisteredProgramPda<'info>>,
) -> Result<()> {
    // account checks
    // 1. Discriminator check
    // 2. Ownership check
    {
        let discriminator_bytes = &ctx.accounts.registered_program_pda.try_borrow_data()?[0..8];
        if discriminator_bytes != RegisteredProgram::DISCRIMINATOR {
            return err!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch);
        }
        light_account_checks::checks::check_owner(
            &crate::ID.to_bytes(),
            &ctx.accounts.registered_program_pda,
        )
        .map_err(ProgramError::from)?;
    }
    let pre_account = RegisteredProgramV1::try_from_slice(
        &ctx.accounts.registered_program_pda.try_borrow_mut_data()?[8..],
    )?;
    // Resize account.
    {
        let rent = Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(RegisteredProgram::LEN);
        let account_info = ctx.accounts.registered_program_pda.to_account_info();
        let lamports_diff = new_minimum_balance.saturating_sub(account_info.lamports());
        invoke(
            &transfer(
                ctx.accounts.authority.key,
                ctx.accounts.registered_program_pda.key,
                lamports_diff,
            ),
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.registered_program_pda.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        account_info.realloc(RegisteredProgram::LEN, true)?;
    }

    // Initialize registered_program_signer_pda with derived signer pda.
    let account_info = ctx.accounts.registered_program_pda.to_account_info();
    let mut data = account_info.try_borrow_mut_data()?;
    let account = from_bytes_mut::<RegisteredProgram>(&mut data[8..]);

    let derived_signer = Pubkey::find_program_address(
        &[CPI_AUTHORITY_PDA_SEED],
        &pre_account.registered_program_id,
    )
    .0;
    account.registered_program_signer_pda = derived_signer;
    Ok(())
}
