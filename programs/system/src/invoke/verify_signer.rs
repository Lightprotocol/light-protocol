use crate::{
    errors::SystemProgramError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    InstructionDataInvoke,
};
use anchor_lang::{
    err,
    solana_program::{msg, pubkey::Pubkey},
    Result,
};
use light_macros::heap_neutral;

#[inline(never)]
#[heap_neutral]
pub fn input_compressed_accounts_signer_check(
    inputs: &InstructionDataInvoke,
    authority: &Pubkey,
) -> Result<()> {
    inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context: &PackedCompressedAccountWithMerkleContext| {
                if compressed_account_with_context.compressed_account.owner != *authority {
                    msg!(
                        "signer check failed compressed account owner {} != authority {}",
                        compressed_account_with_context.compressed_account.owner,
                        authority
                    );
                    err!(SystemProgramError::SignerCheckFailed)
                } else {
                    Ok(())
                }
            },
        )?;
    Ok(())
}
