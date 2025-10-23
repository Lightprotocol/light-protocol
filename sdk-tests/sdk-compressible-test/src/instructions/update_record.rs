use anchor_lang::prelude::*;
use light_sdk::compressible::HasCompressionInfo;

use crate::instruction_accounts::*;

pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;

    user_record.name = name;
    user_record.score = score;

    user_record.compression_info().top_up_rent(
        &user_record.to_account_info(),
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    Ok(())
}
