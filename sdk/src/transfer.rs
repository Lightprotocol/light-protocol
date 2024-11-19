use anchor_lang::Result;

use crate::{account_info::LightAccountInfo, error::LightSdkError};

pub fn transfer_compressed_sol(
    from: &mut LightAccountInfo,
    to: &mut LightAccountInfo,
    lamports: u64,
) -> Result<()> {
    let output_from = from
        .input
        .as_ref()
        .ok_or(LightSdkError::TransferFromNoInput)?
        .lamports
        .ok_or(LightSdkError::TransferFromNoLamports)?
        .checked_sub(lamports)
        .ok_or(LightSdkError::TransferFromInsufficientLamports)?;
    let output_to = to
        .input
        .as_ref()
        .and_then(|input| input.lamports)
        .unwrap_or(0)
        .checked_add(lamports)
        .ok_or(LightSdkError::TransferIntegerOverflow)?;

    from.lamports = Some(output_from);
    to.lamports = Some(output_to);

    Ok(())
}
