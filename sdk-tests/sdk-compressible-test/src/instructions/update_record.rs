use anchor_lang::prelude::*;
use light_sdk::compressible::HasCompressionInfo;

use crate::instruction_accounts::*;

pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;

    user_record.name = name;
    user_record.score = score;

    // 1. Must manually set compression info
    user_record
        .compression_info_mut()
        .bump_last_written_slot()?;

    Ok(())
}
