use anchor_lang::{prelude::*, solana_program::sysvar::clock::Clock};
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

    // Rent top-up on write using the abstracted method
    game_session.compression_info().top_up_rent(
        &game_session.to_account_info(),
        &ctx.accounts.player.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    Ok(())
}
