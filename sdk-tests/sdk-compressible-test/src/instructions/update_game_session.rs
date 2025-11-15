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

    // Must manually set compression info
    game_session
        .compression_info_mut()
        .bump_last_written_slot()?;

    Ok(())
}
