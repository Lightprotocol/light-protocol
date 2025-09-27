use light_compressed_account::instruction_data::zero_copy::ZPackedCompressedAccountWithMerkleContext;
use pinocchio::{msg, pubkey::Pubkey};

use crate::{errors::SystemProgramError, Result};

pub fn input_compressed_accounts_signer_check(
    input_compressed_accounts_with_merkle_context: &[ZPackedCompressedAccountWithMerkleContext],
    authority: &Pubkey,
) -> Result<()> {
    input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context: &ZPackedCompressedAccountWithMerkleContext| {
                if light_compressed_account::pubkey::Pubkey::from(*authority) == compressed_account_with_context.compressed_account.owner
                    && compressed_account_with_context
                        .compressed_account
                        .data
                        .is_none()
                {
                    Ok(())
                } else {
                    msg!(
                        format!("signer check failed compressed account owner {:?} != authority {:?} or data is not none {} (only programs can own compressed accounts with data)",
                        compressed_account_with_context.compressed_account.owner.to_bytes(),
                        authority,
                        compressed_account_with_context.compressed_account.data.is_none()
                    ).as_str());
                    Err(SystemProgramError::SignerCheckFailed.into())
                }
            },
        )
}

#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use light_compressed_account::compressed_account::{
        CompressedAccount, PackedCompressedAccountWithMerkleContext,
    };
    use light_zero_copy::traits::ZeroCopyAt;

    use super::*;

    #[test]
    fn test_input_compressed_accounts_signer_check() {
        let authority = solana_pubkey::Pubkey::new_unique().to_bytes();

        let compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: authority.into(),
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
                std::slice::from_ref(&compressed_account_with_context),
                &authority
            ),
            Ok(())
        );

        {
            let invalid_compressed_account_with_context =
                PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: solana_pubkey::Pubkey::new_unique().to_bytes().into(),
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
