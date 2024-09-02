use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{AccountInfo, Key, ProgramError, Pubkey, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{DataHasher, Discriminator, Poseidon};
use light_system_program::sdk::address::derive_address;
pub use light_system_program::{
    sdk::compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
        PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
    },
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

use crate::{
    address::derive_address_seed,
    merkle_context::{
        pack_merkle_context, PackedAddressMerkleContext, PackedMerkleOutputContext,
        RemainingAccounts,
    },
    program_merkle_context::unpack_address_merkle_context,
};

pub trait LightAccounts: Sized {
    fn try_light_accounts(
        inputs: Vec<Vec<u8>>,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self>;
    fn new_address_params(&self) -> Vec<NewAddressParamsPacked>;
    fn input_accounts(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Vec<PackedCompressedAccountWithMerkleContext>>;
    fn output_accounts(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Vec<OutputCompressedAccountWithPackedContext>>;
}

/// A wrapper which abstracts away the UTXO model.
pub enum LightAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    Init(LightInitAccount<T>),
    Mut(LightMutAccount<T>),
    Close(LightCloseAccount<T>),
}

impl<T> LightAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Default + Discriminator,
{
    pub fn new_init(
        seeds: &[&[u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        address_merkle_context: &PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[AccountInfo],
    ) -> Self {
        Self::Init(LightInitAccount::new(
            seeds,
            program_id,
            merkle_context,
            address_merkle_context,
            address_merkle_tree_root_index,
            remaining_accounts,
        ))
    }

    pub fn try_from_slice_mut(
        v: &[u8],
        seeds: &[&[u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self> {
        Ok(Self::Mut(LightMutAccount::try_from_slice(
            v,
            seeds,
            program_id,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
            remaining_accounts,
        )?))
    }

    pub fn try_from_slice_close(
        v: &[u8],
        seeds: &[&[u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self> {
        Ok(Self::Close(LightCloseAccount::try_from_slice(
            v,
            seeds,
            program_id,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
            remaining_accounts,
        )?))
    }

    pub fn new_address_params(&self) -> Option<NewAddressParamsPacked> {
        match self {
            Self::Init(self_init) => Some(self_init.new_address_params()),
            Self::Mut(_) => None,
            Self::Close(_) => None,
        }
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Option<PackedCompressedAccountWithMerkleContext>> {
        match self {
            Self::Init(_) => Ok(None),
            Self::Mut(light_mut_account) => {
                let account =
                    light_mut_account.input_compressed_account(program_id, remaining_accounts)?;
                Ok(Some(account))
            }
            Self::Close(light_close_account) => {
                let account =
                    light_close_account.input_compressed_account(program_id, remaining_accounts)?;
                Ok(Some(account))
            }
        }
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self {
            Self::Init(light_init_account) => {
                let account =
                    light_init_account.output_compressed_account(program_id, remaining_accounts)?;
                Ok(Some(account))
            }
            Self::Mut(light_mut_account) => {
                let account =
                    light_mut_account.output_compressed_account(program_id, remaining_accounts)?;
                Ok(Some(account))
            }
            Self::Close(_) => Ok(None),
        }
    }
}

impl<T> Deref for LightAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Init(light_init_account) => &light_init_account.output_account,
            Self::Mut(light_mut_account) => &light_mut_account.output_account,
            Self::Close(light_close_account) => &light_close_account.input_account,
        }
    }
}

impl<T> DerefMut for LightAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Init(light_init_account) => &mut light_init_account.output_account,
            Self::Mut(light_mut_account) => &mut light_mut_account.output_account,
            Self::Close(light_close_account) => &mut light_close_account.input_account,
        }
    }
}

pub struct LightInitAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    output_account: T,
    address_seed: [u8; 32],
    merkle_context: PackedMerkleContext,
    address_merkle_context: PackedAddressMerkleContext,
    address_merkle_tree_root_index: u16,
}

impl<T> LightInitAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + Default + DataHasher + Discriminator,
{
    pub fn new<'a>(
        seeds: &'a [&'a [u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        address_merkle_context: &PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[AccountInfo],
    ) -> Self {
        let output_account = T::default();

        let unpacked_address_merkle_context =
            unpack_address_merkle_context(*address_merkle_context, remaining_accounts);
        let address_seed = derive_address_seed(seeds, program_id, &unpacked_address_merkle_context);

        Self {
            output_account,
            address_seed,
            merkle_context: *merkle_context,
            address_merkle_context: *address_merkle_context,
            address_merkle_tree_root_index,
        }
    }

    pub fn new_address_params(&self) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: self.address_seed,
            address_merkle_tree_account_index: self
                .address_merkle_context
                .address_merkle_tree_pubkey_index,
            address_queue_account_index: self.address_merkle_context.address_queue_pubkey_index,
            address_merkle_tree_root_index: self.address_merkle_tree_root_index,
        }
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<OutputCompressedAccountWithPackedContext> {
        output_compressed_account(
            &self.output_account,
            &self.address_seed,
            program_id,
            &self.merkle_context,
            &self.address_merkle_context,
            remaining_accounts,
        )
    }
}

impl<T> Deref for LightInitAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.output_account
    }
}

impl<T> DerefMut for LightInitAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output_account
    }
}

pub struct LightMutAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    input_account: T,
    output_account: T,
    address_seed: [u8; 32],
    merkle_context: PackedMerkleContext,
    merkle_tree_root_index: u16,
    address_merkle_context: PackedAddressMerkleContext,
}

impl<T> LightMutAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(
        v: &[u8],
        seeds: &[&[u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self> {
        use anchor_lang::prelude::msg;
        msg!("before");
        msg!("v: {:?}", v);
        let account = T::try_from_slice(v)?;
        msg!("after");

        let unpacked_address_merkle_context =
            unpack_address_merkle_context(*address_merkle_context, remaining_accounts);
        let address_seed = derive_address_seed(seeds, program_id, &unpacked_address_merkle_context);

        Ok(Self {
            input_account: account.clone(),
            output_account: account,
            address_seed,
            merkle_context: *merkle_context,
            merkle_tree_root_index,
            address_merkle_context: *address_merkle_context,
        })
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<PackedCompressedAccountWithMerkleContext> {
        input_compressed_account(
            &self.input_account,
            &self.address_seed,
            program_id,
            &self.merkle_context,
            self.merkle_tree_root_index,
            &self.address_merkle_context,
            remaining_accounts,
        )
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<OutputCompressedAccountWithPackedContext> {
        output_compressed_account(
            &self.output_account,
            &self.address_seed,
            program_id,
            &self.merkle_context,
            &self.address_merkle_context,
            remaining_accounts,
        )
    }
}

impl<T> Deref for LightMutAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.output_account
    }
}

impl<T> DerefMut for LightMutAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output_account
    }
}

pub struct LightCloseAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    input_account: T,
    address_seed: [u8; 32],
    merkle_context: PackedMerkleContext,
    merkle_tree_root_index: u16,
    address_merkle_context: PackedAddressMerkleContext,
}

impl<T> LightCloseAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(
        v: &[u8],
        seeds: &[&[u8]],
        program_id: &Pubkey,
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self> {
        let input_account = T::try_from_slice(v)?;

        let unpacked_address_merkle_context =
            unpack_address_merkle_context(*address_merkle_context, remaining_accounts);
        let address_seed = derive_address_seed(seeds, program_id, &unpacked_address_merkle_context);

        Ok(Self {
            input_account,
            address_seed,
            merkle_context: *merkle_context,
            merkle_tree_root_index,
            address_merkle_context: *address_merkle_context,
        })
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<PackedCompressedAccountWithMerkleContext> {
        input_compressed_account(
            &self.input_account,
            &self.address_seed,
            program_id,
            &self.merkle_context,
            self.merkle_tree_root_index,
            &self.address_merkle_context,
            remaining_accounts,
        )
    }
}

impl<T> Deref for LightCloseAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.input_account
    }
}

impl<T> DerefMut for LightCloseAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_account
    }
}

pub fn serialize_and_hash_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<CompressedAccount>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let data = account.try_to_vec()?;
    let data_hash = account.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: T::discriminator(),
        data,
        data_hash,
    };

    let address = derive_address(
        &remaining_accounts[address_merkle_context.address_merkle_tree_pubkey_index as usize].key(),
        address_seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;

    let compressed_account = CompressedAccount {
        owner: *program_id,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(compressed_account)
}

pub fn new_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_output_context: &PackedMerkleOutputContext,
    address_merkle_context: &PackedAddressMerkleContext,
    address_merkle_tree_root_index: u16,
    remaining_accounts: &[AccountInfo],
) -> Result<(
    OutputCompressedAccountWithPackedContext,
    NewAddressParamsPacked,
)>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    let compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: merkle_output_context.merkle_tree_pubkey_index,
    };

    let new_address_params = NewAddressParamsPacked {
        seed: *address_seed,
        address_merkle_tree_account_index: address_merkle_context.address_merkle_tree_pubkey_index,
        address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
        address_merkle_tree_root_index,
    };

    Ok((compressed_account, new_address_params))
}

pub fn input_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_context: &PackedMerkleContext,
    merkle_tree_root_index: u16,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<PackedCompressedAccountWithMerkleContext>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    Ok(PackedCompressedAccountWithMerkleContext {
        compressed_account,
        merkle_context: *merkle_context,
        root_index: merkle_tree_root_index,
        read_only: false,
    })
}

pub fn output_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_context: &PackedMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<OutputCompressedAccountWithPackedContext>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: merkle_context.merkle_tree_pubkey_index,
    })
}

pub fn pack_compressed_accounts(
    compressed_accounts: &[CompressedAccountWithMerkleContext],
    root_indices: &[u16],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedCompressedAccountWithMerkleContext> {
    compressed_accounts
        .iter()
        .zip(root_indices.iter())
        .map(|(x, root_index)| PackedCompressedAccountWithMerkleContext {
            compressed_account: x.compressed_account.clone(),
            merkle_context: pack_merkle_context(x.merkle_context, remaining_accounts),
            root_index: *root_index,
            read_only: false,
        })
        .collect::<Vec<_>>()
}

pub fn pack_compressed_account(
    compressed_account: CompressedAccountWithMerkleContext,
    root_index: u16,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedCompressedAccountWithMerkleContext {
    pack_compressed_accounts(&[compressed_account], &[root_index], remaining_accounts)[0].clone()
}
