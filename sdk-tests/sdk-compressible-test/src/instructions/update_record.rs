use anchor_lang::prelude::*;
use anchor_lang::system_program;
use light_sdk::compressible::HasCompressionInfo;

use crate::instruction_accounts::*;

pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;

    user_record.name = name;
    user_record.score = score;

    // Calculate rent top-up
    let compression_info = user_record.compression_info();
    let bytes = user_record.to_account_info().data_len() as u64;
    let current_lamports = user_record.to_account_info().lamports();
    let current_slot = Clock::get()?.slot;
    let rent_exemption_lamports = Rent::get()?.minimum_balance(bytes as usize);

    let top_up = compression_info.calculate_top_up_lamports(
        bytes,
        current_slot,
        current_lamports,
        rent_exemption_lamports,
    );

    // Transfer lamports via System Program CPI if needed
    if top_up > 0 {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: user_record.to_account_info(),
                },
            ),
            top_up,
        )?;
    }

    Ok(())
}
