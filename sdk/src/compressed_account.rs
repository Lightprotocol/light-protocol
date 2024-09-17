use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{AccountInfo, ProgramError, Pubkey, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{DataHasher, Discriminator, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    address::{derive_address, NewAddressParamsPacked},
    merkle_context::{
        pack_merkle_context, MerkleContext, PackedAddressMerkleContext, PackedMerkleContext,
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
        merkle_context: &PackedMerkleContext,
        address_merkle_context: &PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
    ) -> Self {
        Self::Init(LightInitAccount::new(
            merkle_context,
            address_merkle_context,
            address_merkle_tree_root_index,
        ))
    }

    pub fn try_from_slice_mut(
        v: &[u8],
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
    ) -> Result<Self> {
        Ok(Self::Mut(LightMutAccount::try_from_slice(
            v,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
        )?))
    }

    pub fn try_from_slice_close(
        v: &[u8],
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
    ) -> Result<Self> {
        Ok(Self::Close(LightCloseAccount::try_from_slice(
            v,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
        )?))
    }

    pub fn set_address_seed(&mut self, address_seed: [u8; 32]) {
        match self {
            Self::Init(light_init_account) => light_init_account.set_address_seed(address_seed),
            Self::Mut(light_mut_account) => light_mut_account.set_address_seed(address_seed),
            Self::Close(light_close_account) => light_close_account.set_address_seed(address_seed),
        }
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
    address_seed: Option<[u8; 32]>,
    merkle_context: PackedMerkleContext,
    address_merkle_context: PackedAddressMerkleContext,
    address_merkle_tree_root_index: u16,
}

impl<T> LightInitAccount<T>
where
    T: BorshDeserialize + BorshSerialize + Clone + Default + DataHasher + Discriminator,
{
    pub fn new(
        merkle_context: &PackedMerkleContext,
        address_merkle_context: &PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
    ) -> Self {
        let output_account = T::default();

        Self {
            output_account,
            address_seed: None,
            merkle_context: *merkle_context,
            address_merkle_context: *address_merkle_context,
            address_merkle_tree_root_index,
        }
    }

    pub fn set_address_seed(&mut self, address_seed: [u8; 32]) {
        self.address_seed = Some(address_seed);
    }

    pub fn new_address_params(&self) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: self.address_seed.unwrap(),
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
            &self.address_seed.unwrap(),
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
    address_seed: Option<[u8; 32]>,
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
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
    ) -> Result<Self> {
        let account = T::try_from_slice(v)?;

        Ok(Self {
            input_account: account.clone(),
            output_account: account,
            address_seed: None,
            merkle_context: *merkle_context,
            merkle_tree_root_index,
            address_merkle_context: *address_merkle_context,
        })
    }

    pub fn set_address_seed(&mut self, address_seed: [u8; 32]) {
        self.address_seed = Some(address_seed);
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<PackedCompressedAccountWithMerkleContext> {
        input_compressed_account(
            &self.input_account,
            &self.address_seed.unwrap(),
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
            &self.address_seed.unwrap(),
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
    address_seed: Option<[u8; 32]>,
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
        merkle_context: &PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: &PackedAddressMerkleContext,
    ) -> Result<Self> {
        let input_account = T::try_from_slice(v)?;

        Ok(Self {
            input_account,
            address_seed: None,
            merkle_context: *merkle_context,
            merkle_tree_root_index,
            address_merkle_context: *address_merkle_context,
        })
    }

    pub fn set_address_seed(&mut self, address_seed: [u8; 32]) {
        self.address_seed = Some(address_seed);
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<PackedCompressedAccountWithMerkleContext> {
        input_compressed_account(
            &self.input_account,
            &self.address_seed.unwrap(),
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

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct CompressedAccount {
    pub owner: Pubkey,
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

/// Hashing scheme:
/// H(owner || leaf_index || merkle_tree_pubkey || lamports || address || data.discriminator || data.data_hash)
impl CompressedAccount {
    pub fn hash_with_hashed_values<H: Hasher>(
        &self,
        &owner_hashed: &[u8; 32],
        &merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
    ) -> Result<[u8; 32]> {
        let capacity = 3
            + std::cmp::min(self.lamports, 1) as usize
            + self.address.is_some() as usize
            + self.data.is_some() as usize * 2;
        let mut vec: Vec<&[u8]> = Vec::with_capacity(capacity);
        vec.push(owner_hashed.as_slice());

        // leaf index and merkle tree pubkey are used to make every compressed account hash unique
        let leaf_index = leaf_index.to_le_bytes();
        vec.push(leaf_index.as_slice());

        vec.push(merkle_tree_hashed.as_slice());

        // Lamports are only hashed if non-zero to safe CU
        // For safety we prefix the lamports with 1 in 1 byte.
        // Thus even if the discriminator has the same value as the lamports, the hash will be different.
        let mut lamports_bytes = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        if self.lamports != 0 {
            lamports_bytes[1..].copy_from_slice(&self.lamports.to_le_bytes());
            vec.push(lamports_bytes.as_slice());
        }

        if self.address.is_some() {
            vec.push(self.address.as_ref().unwrap().as_slice());
        }

        let mut discriminator_bytes = [2, 0, 0, 0, 0, 0, 0, 0, 0];
        if let Some(data) = &self.data {
            discriminator_bytes[1..].copy_from_slice(&data.discriminator);
            vec.push(&discriminator_bytes);
            vec.push(&data.data_hash);
        }
        let hash = H::hashv(&vec).map_err(ProgramError::from)?;
        Ok(hash)
    }

    pub fn hash<H: Hasher>(
        &self,
        &merkle_tree_pubkey: &Pubkey,
        leaf_index: &u32,
    ) -> Result<[u8; 32]> {
        self.hash_with_hashed_values::<H>(
            &hash_to_bn254_field_size_be(&self.owner.to_bytes())
                .unwrap()
                .0,
            &hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0,
            leaf_index,
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    pub read_only: bool,
}

impl CompressedAccountWithMerkleContext {
    pub fn hash(&self) -> Result<[u8; 32]> {
        self.compressed_account.hash::<Poseidon>(
            &self.merkle_context.merkle_tree_pubkey,
            &self.merkle_context.leaf_index,
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
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

    let address_merkle_context =
        unpack_address_merkle_context(*address_merkle_context, remaining_accounts);
    let address = derive_address(address_seed, &address_merkle_context);
    anchor_lang::prelude::msg!("ADDRESS: {:?}", address);

    let compressed_account = CompressedAccount {
        owner: *program_id,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(compressed_account)
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
