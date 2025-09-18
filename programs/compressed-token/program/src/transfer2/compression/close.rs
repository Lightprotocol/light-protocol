use anchor_lang::prelude::ProgramError;
use light_ctoken_types::instructions::transfer2::{ZCompression, ZCompressionMode};

use crate::{
    close_token_account::{accounts::CloseTokenAccountAccounts, processor::close_token_account},
    transfer2::accounts::Transfer2Accounts,
};

pub fn close_for_compress_and_close(
    compressions: &[ZCompression<'_>],
    validated_accounts: &Transfer2Accounts,
) -> Result<(), ProgramError> {
    for compression in compressions
        .iter()
        .filter(|c| c.mode == ZCompressionMode::CompressAndClose)
    {
        let token_account_info = validated_accounts.packed_accounts.get_u8(
            compression.source_or_recipient,
            "CompressAndClose: source_or_recipient",
        )?;
        let destination = validated_accounts.packed_accounts.get_u8(
            compression.get_destination_index()?,
            "CompressAndClose: destination",
        )?;
        let rent_sponsor = validated_accounts.packed_accounts.get_u8(
            compression.get_rent_sponsor_index()?,
            "CompressAndClose: rent_sponsor",
        )?;
        let authority = validated_accounts
            .packed_accounts
            .get_u8(compression.authority, "CompressAndClose: authority")?;
        close_token_account(&CloseTokenAccountAccounts {
            token_account: token_account_info,
            destination,
            authority,
            rent_sponsor: Some(rent_sponsor),
        })?;
    }
    Ok(())
}
