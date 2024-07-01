use anchor_lang::{
    err,
    solana_program::{msg, pubkey::Pubkey},
    Result,
};

use crate::{
    errors::SystemProgramError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    InstructionDataInvoke,
};

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
        )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sdk::compressed_account::CompressedAccount;

    #[test]
    fn test_input_compressed_accounts_signer_check() {
        let authority = Pubkey::new_unique();
        let mut compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: authority,
                ..CompressedAccount::default()
            },
            ..PackedCompressedAccountWithMerkleContext::default()
        };

        assert_eq!(
            input_compressed_accounts_signer_check(
                &InstructionDataInvoke {
                    input_compressed_accounts_with_merkle_context: vec![
                        compressed_account_with_context.clone()
                    ],
                    ..InstructionDataInvoke::default()
                },
                &authority
            ),
            Ok(())
        );

        compressed_account_with_context.compressed_account.owner = Pubkey::new_unique();
        assert_eq!(
            input_compressed_accounts_signer_check(
                &InstructionDataInvoke {
                    input_compressed_accounts_with_merkle_context: vec![
                        compressed_account_with_context
                    ],
                    ..InstructionDataInvoke::default()
                },
                &authority
            ),
            Err(SystemProgramError::SignerCheckFailed.into())
        );
    }
}
