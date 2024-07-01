use anchor_lang::{
    err,
    solana_program::{msg, pubkey::Pubkey},
    Result,
};

use crate::{
    errors::SystemProgramError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
};

pub fn input_compressed_accounts_signer_check(
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    authority: &Pubkey,
) -> Result<()> {
    input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context: &PackedCompressedAccountWithMerkleContext| {
                if compressed_account_with_context.compressed_account.owner == *authority {
                    Ok(())
                } else {
                    msg!(
                        "signer check failed compressed account owner {} != authority {}",
                        compressed_account_with_context.compressed_account.owner,
                        authority
                    );
                    err!(SystemProgramError::SignerCheckFailed)
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
        let compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: authority,
                ..CompressedAccount::default()
            },
            ..PackedCompressedAccountWithMerkleContext::default()
        };

        assert_eq!(
            input_compressed_accounts_signer_check(
                &vec![compressed_account_with_context.clone()],
                &authority
            ),
            Ok(())
        );
        let invalid_compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::new_unique(),
                ..CompressedAccount::default()
            },
            ..PackedCompressedAccountWithMerkleContext::default()
        };
        assert_eq!(
            input_compressed_accounts_signer_check(
                &vec![
                    compressed_account_with_context,
                    invalid_compressed_account_with_context
                ],
                &authority
            ),
            Err(SystemProgramError::SignerCheckFailed.into())
        );
    }
}
