use anchor_lang::{prelude::*, solana_program::sysvar::clock::Clock, system_program};
use light_sdk::compressible::HasCompressionInfo;

use crate::instruction_accounts::*;
pub fn update_game_session(
    ctx: Context<UpdateGameSession>,
    _session_id: u64,
    new_score: u64,
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;

    game_session.score = new_score;
    game_session.end_time = Some(Clock::get()?.unix_timestamp as u64);

    // Calculate rent top-up
    let compression_info = game_session.compression_info();
    let bytes = game_session.to_account_info().data_len() as u64;
    let current_lamports = game_session.to_account_info().lamports();
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
                    from: ctx.accounts.player.to_account_info(),
                    to: game_session.to_account_info(),
                },
            ),
            top_up,
        )?;
    }

    Ok(())
}
