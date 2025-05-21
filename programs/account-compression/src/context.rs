use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, ProgramError},
    solana_program::{msg, pubkey::Pubkey},
    Discriminator as AnchorDiscriminator, Key, ToAccountInfo,
};
use light_account_checks::{discriminator::Discriminator, error::AccountError};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopyMut;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopyMut;
use light_merkle_tree_metadata::TreeType;

use crate::{
    address_merkle_tree_from_bytes_zero_copy_mut,
    errors::AccountCompressionErrorCode,
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::check_signer_is_registered_or_authority::{
        manual_check_signer_is_registered_or_authority, GroupAccess,
    },
    AddressMerkleTreeAccount, QueueAccount, StateMerkleTreeAccount,
};

impl GroupAccess for BatchedQueueAccount<'_> {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata
            .access_metadata
            .program_owner
            .to_bytes()
            .into()
    }
}
use super::RegisteredProgram;

/// AccountCompressionProgramAccount
#[derive(Debug)]
pub enum AcpAccount<'a, 'info> {
    Authority(&'a AccountInfo<'info>),
    RegisteredProgramPda(&'a AccountInfo<'info>),
    SystemProgram(&'a AccountInfo<'info>),
    OutputQueue(BatchedQueueAccount<'info>),
    BatchedStateTree(BatchedMerkleTreeAccount<'info>),
    BatchedAddressTree(BatchedMerkleTreeAccount<'info>),
    StateTree((Pubkey, ConcurrentMerkleTreeZeroCopyMut<'info, Poseidon, 26>)),
    AddressTree(
        (
            Pubkey,
            IndexedMerkleTreeZeroCopyMut<'info, Poseidon, usize, 26, 16>,
        ),
    ),
    AddressQueue(Pubkey, AccountInfo<'info>),
    V1Queue(AccountInfo<'info>),
    Unknown(),
}

impl<'a, 'info> AcpAccount<'a, 'info> {
    /// Merkle tree and queue accounts
    #[inline(always)]
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo<'info>],
        authority: &'a AccountInfo<'info>,
        invoked_by_program: bool,
        // TODO: remove in separate pr because it impacts photon derivation.
        _bump: u8,
    ) -> std::result::Result<Vec<AcpAccount<'a, 'info>>, ProgramError> {
        let mut vec = Vec::with_capacity(account_infos.len());
        let mut skip = 0;
        let derived_address = match invoked_by_program {
            true => {
                let account_info = &account_infos[0];
                let data = account_info.try_borrow_data()?;
                if RegisteredProgram::DISCRIMINATOR != &data[..8] {
                    return Err(AccountError::InvalidDiscriminator.into());
                }
                if account_info.owner != &crate::ID {
                    return Err(AccountError::AccountOwnedByWrongProgram.into());
                }
                let account = bytemuck::from_bytes::<RegisteredProgram>(&data[8..]);

                if account.registered_program_signer_pda != *authority.key {
                    return Err(AccountError::InvalidSigner.into());
                }
                skip += 1;
                Some((
                    account.registered_program_signer_pda,
                    account.group_authority_pda,
                ))
            }
            false => None,
        };

        account_infos.iter().skip(skip).try_for_each(
            |account_info| -> Result<(), ProgramError> {
                let account = AcpAccount::try_from_account_info(
                    account_info,
                    &AcpAccount::Authority(authority),
                    &derived_address,
                )?;
                vec.push(account);
                Ok(())
            },
        )?;
        Ok(vec)
    }

    /// Try to deserialize and check account info:
    /// 1. Owner is crate::ID
    /// 2. match discriminator
    /// 3. check signer is registered program or authority
    ///    (Unless the account is a v1 queue account.
    ///    v1 queue accounts are always used in combination
    ///    with a v1 Merkle tree and are checked that these
    ///    are associated to it.)
    #[inline(always)]
    pub(crate) fn try_from_account_info(
        account_info: &'info AccountInfo<'info>,
        authority: &AcpAccount<'a, 'info>,
        registered_program_pda: &Option<(Pubkey, Pubkey)>,
    ) -> anchor_lang::Result<AcpAccount<'a, 'info>> {
        if crate::ID != *account_info.owner {
            msg!("Invalid owner {:?}", account_info.owner);
            msg!("key {:?}", account_info.key());
            return Err(ProgramError::from(AccountError::AccountOwnedByWrongProgram).into());
        }
        let mut discriminator = [0u8; 8];
        {
            let data = account_info.try_borrow_data()?;
            discriminator.copy_from_slice(&data[..8]);
        }
        match &discriminator[..] {
            BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => {
                let mut tree_type = [0u8; 8];
                tree_type.copy_from_slice(&account_info.try_borrow_data()?[8..16]);
                let tree_type = TreeType::from(u64::from_le_bytes(tree_type));
                match tree_type {
                    TreeType::AddressV2 => {
                        let tree =
                            BatchedMerkleTreeAccount::address_from_account_info(account_info)
                                .map_err(ProgramError::from)?;
                        manual_check_signer_is_registered_or_authority::<BatchedMerkleTreeAccount>(
                            registered_program_pda,
                            authority,
                            &tree,
                        )?;
                        Ok(AcpAccount::BatchedAddressTree(tree))
                    }
                    TreeType::StateV2 => {
                        let tree = BatchedMerkleTreeAccount::state_from_account_info(account_info)
                            .map_err(ProgramError::from)?;
                        manual_check_signer_is_registered_or_authority::<BatchedMerkleTreeAccount>(
                            registered_program_pda,
                            authority,
                            &tree,
                        )?;

                        Ok(AcpAccount::BatchedStateTree(tree))
                    }
                    _ => Err(ProgramError::from(AccountError::BorrowAccountDataFailed).into()),
                }
            }
            BatchedQueueAccount::LIGHT_DISCRIMINATOR_SLICE => {
                let queue = BatchedQueueAccount::output_from_account_info(account_info)
                    .map_err(ProgramError::from)?;

                manual_check_signer_is_registered_or_authority::<BatchedQueueAccount>(
                    registered_program_pda,
                    authority,
                    &queue,
                )?;

                Ok(AcpAccount::OutputQueue(queue))
            }
            StateMerkleTreeAccount::DISCRIMINATOR => {
                {
                    let merkle_tree =
                        AccountLoader::<StateMerkleTreeAccount>::try_from(account_info)?;
                    let merkle_tree = merkle_tree.load()?;

                    manual_check_signer_is_registered_or_authority::<StateMerkleTreeAccount>(
                        registered_program_pda,
                        authority,
                        &merkle_tree,
                    )?;
                }
                let mut merkle_tree = account_info.try_borrow_mut_data()?;
                let data_slice: &'info mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
                };
                Ok(AcpAccount::StateTree((
                    account_info.key(),
                    state_merkle_tree_from_bytes_zero_copy_mut(data_slice)?,
                )))
            }
            AddressMerkleTreeAccount::DISCRIMINATOR => {
                {
                    let merkle_tree =
                        AccountLoader::<AddressMerkleTreeAccount>::try_from(account_info)?;
                    let merkle_tree = merkle_tree.load()?;
                    manual_check_signer_is_registered_or_authority::<AddressMerkleTreeAccount>(
                        registered_program_pda,
                        authority,
                        &merkle_tree,
                    )?;
                }
                let mut merkle_tree = account_info.try_borrow_mut_data()?;
                let data_slice: &'info mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
                };
                Ok(AcpAccount::AddressTree((
                    account_info.key(),
                    address_merkle_tree_from_bytes_zero_copy_mut(data_slice)?,
                )))
            }
            QueueAccount::DISCRIMINATOR => {
                msg!("queue account: {:?}", account_info.key());
                Ok(AcpAccount::V1Queue(account_info.to_account_info()))
            }
            _ => Err(AccountCompressionErrorCode::InvalidAccount.into()),
        }
    }
}
