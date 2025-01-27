use anchor_lang::{
    err,
    solana_program::{msg, pubkey::Pubkey},
    Result,
};
use light_utils::instruction::instruction_data_zero_copy::ZPackedCompressedAccountWithMerkleContext;

use crate::errors::SystemProgramError;

pub fn input_compressed_accounts_signer_check(
    input_compressed_accounts_with_merkle_context: &[ZPackedCompressedAccountWithMerkleContext],
    authority: &Pubkey,
) -> Result<()> {
    input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context: &ZPackedCompressedAccountWithMerkleContext| {
                if *authority == compressed_account_with_context.compressed_account.owner.into()
                    && compressed_account_with_context
                        .compressed_account
                        .data
                        .is_none()
                {
                    Ok(())
                } else {
                    msg!(
                        "signer check failed compressed account owner {} != authority {} or data is not none {} (only programs can own compressed accounts with data)",
                        Pubkey::new_from_array(compressed_account_with_context.compressed_account.owner.to_bytes()),
                        authority,
                        compressed_account_with_context.compressed_account.data.is_none()
                    );
                    err!(SystemProgramError::SignerCheckFailed)
                }
            },
        )
}

#[cfg(test)]
mod test {
    use anchor_lang::prelude::borsh::BorshSerialize;
    use light_utils::instruction::compressed_account::{
        CompressedAccount, PackedCompressedAccountWithMerkleContext,
    };
    use light_zero_copy::borsh::Deserialize;

    use super::*;

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
        let bytes = compressed_account_with_context.try_to_vec().unwrap();
        let compressed_account_with_context =
            ZPackedCompressedAccountWithMerkleContext::zero_copy_at(&bytes)
                .unwrap()
                .0;

        assert_eq!(
            input_compressed_accounts_signer_check(
                &[compressed_account_with_context.clone()],
                &authority
            ),
            Ok(())
        );

        {
            let invalid_compressed_account_with_context =
                PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: Pubkey::new_unique(),
                        ..CompressedAccount::default()
                    },
                    ..PackedCompressedAccountWithMerkleContext::default()
                };

            let bytes = invalid_compressed_account_with_context
                .try_to_vec()
                .unwrap();
            let invalid_compressed_account_with_context =
                ZPackedCompressedAccountWithMerkleContext::zero_copy_at(&bytes)
                    .unwrap()
                    .0;
            assert_eq!(
                input_compressed_accounts_signer_check(
                    &[
                        compressed_account_with_context,
                        invalid_compressed_account_with_context
                    ],
                    &authority
                ),
                Err(SystemProgramError::SignerCheckFailed.into())
            );
        }
    }
}
