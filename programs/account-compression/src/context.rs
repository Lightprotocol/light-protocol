use anchor_lang::{
    prelude::{AccountInfo, AccountLoader},
    solana_program::{msg, pubkey::Pubkey},
    Discriminator as AnchorDiscriminator, Key, ToAccountInfo,
};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopyMut;
use light_hasher::{Discriminator, Poseidon};
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopyMut;
use light_merkle_tree_metadata::merkle_tree::TreeType;

use crate::{
    address_merkle_tree_from_bytes_zero_copy_mut,
    errors::AccountCompressionErrorCode,
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::{
        check_signer_is_registered_or_authority::{
            manual_check_signer_is_registered_or_authority, GroupAccess,
        },
        constants::CPI_AUTHORITY_PDA_SEED,
    },
    AddressMerkleTreeAccount, QueueAccount, StateMerkleTreeAccount,
};

impl GroupAccess for BatchedQueueAccount<'_> {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.to_bytes().into()
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
pub struct LightContext<'a, 'info> {
    pub accounts: Vec<AcpAccount<'a, 'info>>,
    invoked_by_program: bool,
}

impl<'a, 'info> LightContext<'a, 'info> {
    #[inline(always)]
    pub fn new(
        authority: &'a AccountInfo<'info>,
        account_infos: &'info [AccountInfo<'info>],
        invoked_by_program: bool,
        bump: u8,
    ) -> LightContext<'a, 'info> {
        let accounts =
            AcpAccount::from_account_infos(account_infos, authority, invoked_by_program, bump)
                .unwrap();
        LightContext {
            accounts,
            invoked_by_program,
        }
    }

    pub fn authority(&self) -> &AccountInfo<'info> {
        match self.accounts[AUTHORITY_INDEX] {
            AcpAccount::Authority(account) => account,
            _ => panic!("Invalid fee payer account"),
        }
    }

    /// Index 0 : authority
    /// ... other accounts (registry program PDA is not added)
    #[inline(always)]
    pub fn remaining_accounts_mut(&mut self) -> &mut [AcpAccount<'a, 'info>] {
        let offset = if self.invoked_by_program { 1 } else { 0 };
        &mut self.accounts[offset..]
    }

    /// Index 0 : authority
    /// ... other accounts (registry program PDA is not added)
    #[inline(always)]
    pub fn remaining_accounts(&self) -> &[AcpAccount<'a, 'info>] {
        let offset = if self.invoked_by_program { 1 } else { 0 };
        &self.accounts[offset..]
    }
}

const AUTHORITY_INDEX: usize = 0;

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
    // NullifierQueue(AccountInfo<'info>),
    V1Queue(AccountInfo<'info>),
    Unknown(),
}

impl<'a, 'info> AcpAccount<'a, 'info> {
    /// Account order:
    /// 1. Fee payer
    /// 2. Authority
    /// 3. Option<Registered program PDA>
    ///     ... other accounts
    #[inline(always)]
    pub fn from_account_infos(
        account_infos: &'info [AccountInfo<'info>],
        authority: &'a AccountInfo<'info>,
        invoked_by_program: bool,
        bump: u8,
    ) -> std::result::Result<Vec<AcpAccount<'a, 'info>>, AccountCompressionErrorCode> {
        // TODO: remove + 1 and pass in fee_payer once we removed anchor.
        let mut vec = Vec::with_capacity(account_infos.len() + 1);
        vec.push(AcpAccount::Authority(authority));
        let mut skip = 0;
        let derived_address = match invoked_by_program {
            true => {
                let account_info = &account_infos[0];
                let data = account_info.try_borrow_data().unwrap();
                if RegisteredProgram::DISCRIMINATOR.as_slice() != &data[..8] {
                    panic!("Invalid discriminator");
                }
                let account = bytemuck::from_bytes::<RegisteredProgram>(&data[8..]);
                // 1,670 CU
                // TODO: get from RegisteredProgram account and compare
                let derived_address = Pubkey::create_program_address(
                    &[CPI_AUTHORITY_PDA_SEED, &[bump]],
                    &account.registered_program_id,
                )
                .unwrap();
                skip += 1;
                Some((derived_address, account.group_authority_pda))
            }
            false => None,
        };

        account_infos.iter().skip(skip).for_each(|account_info| {
            let account =
                AcpAccount::try_from_account_info(account_info, &vec[0], &derived_address).unwrap();
            vec.push(account);
        });
        Ok(vec)
    }

    #[inline(always)]
    pub(crate) fn try_from_account_info(
        account_info: &'info AccountInfo<'info>,
        authority: &AcpAccount<'a, 'info>,
        registered_program_pda: &Option<(Pubkey, Pubkey)>,
    ) -> std::result::Result<AcpAccount<'a, 'info>, AccountCompressionErrorCode> {
        if crate::ID != *account_info.owner {
            msg!("Invalid owner {:?}", account_info.owner);
            msg!("key {:?}", account_info.key());
            return Err(AccountCompressionErrorCode::InputDeserializationFailed);
        }
        let mut discriminator = [0u8; 8];
        {
            let data = account_info
                .try_borrow_data()
                .map_err(|_| AccountCompressionErrorCode::InvalidAccount)?;
            discriminator.copy_from_slice(&data[..8]);
        }
        match discriminator {
            BatchedMerkleTreeAccount::DISCRIMINATOR => {
                let mut tree_type = [0u8; 8];
                tree_type.copy_from_slice(
                    &account_info
                        .try_borrow_data()
                        .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?
                        [8..16],
                );
                let tree_type = TreeType::from(u64::from_le_bytes(tree_type));
                match tree_type {
                    TreeType::BatchedAddress => Ok(AcpAccount::BatchedAddressTree(
                        BatchedMerkleTreeAccount::address_from_account_info(account_info).unwrap(),
                    )),
                    TreeType::BatchedState => {
                        let tree = BatchedMerkleTreeAccount::state_from_account_info(account_info)
                            .unwrap();
                        manual_check_signer_is_registered_or_authority::<BatchedMerkleTreeAccount>(
                            registered_program_pda,
                            authority,
                            &tree,
                        )
                        .unwrap();

                        Ok(AcpAccount::BatchedStateTree(tree))
                    }
                    _ => Err(AccountCompressionErrorCode::InputDeserializationFailed),
                }
            }
            BatchedQueueAccount::DISCRIMINATOR => {
                let queue = BatchedQueueAccount::output_from_account_info(account_info).unwrap();

                manual_check_signer_is_registered_or_authority::<BatchedQueueAccount>(
                    registered_program_pda,
                    authority,
                    &queue,
                )
                .unwrap();

                Ok(AcpAccount::OutputQueue(queue))
            }
            StateMerkleTreeAccount::DISCRIMINATOR => {
                {
                    let merkle_tree =
                        AccountLoader::<StateMerkleTreeAccount>::try_from(account_info).unwrap();
                    let merkle_tree = merkle_tree.load().unwrap();

                    manual_check_signer_is_registered_or_authority::<StateMerkleTreeAccount>(
                        registered_program_pda,
                        authority,
                        &merkle_tree,
                    )
                    .unwrap();
                }
                let mut merkle_tree = account_info
                    .try_borrow_mut_data()
                    .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
                let data_slice: &'info mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
                };
                Ok(AcpAccount::StateTree((
                    account_info.key(),
                    state_merkle_tree_from_bytes_zero_copy_mut(data_slice).unwrap(),
                )))
            }
            AddressMerkleTreeAccount::DISCRIMINATOR => {
                {
                    let merkle_tree =
                        AccountLoader::<AddressMerkleTreeAccount>::try_from(account_info).unwrap();
                    let merkle_tree = merkle_tree.load().unwrap();
                    manual_check_signer_is_registered_or_authority::<AddressMerkleTreeAccount>(
                        registered_program_pda,
                        authority,
                        &merkle_tree,
                    )
                    .unwrap();
                }
                let mut merkle_tree = account_info
                    .try_borrow_mut_data()
                    .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
                let data_slice: &'info mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(merkle_tree.as_mut_ptr(), merkle_tree.len())
                };
                Ok(AcpAccount::AddressTree((
                    account_info.key(),
                    address_merkle_tree_from_bytes_zero_copy_mut(data_slice).unwrap(),
                )))
            }
            QueueAccount::DISCRIMINATOR => {
                msg!("queue account: {:?}", account_info.key());
                Ok(AcpAccount::V1Queue(account_info.to_account_info()))
            }
            _ => panic!("invalid account"),
        }
    }
}
