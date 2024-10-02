use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize, ProgramError, Pubkey, Result};
use light_hasher::{DataHasher, Discriminator, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_be;
use solana_program::account_info::AccountInfo;

use crate::{
    account_info::{
        LightAccountInfo, LightCloseAccountInfo, LightInitAccountInfo, LightMutAccountInfo,
    },
    address::{derive_address, derive_address_seed, PackedNewAddressParams},
    merkle_context::{pack_merkle_context, MerkleContext, PackedMerkleContext, RemainingAccounts},
    program_merkle_context::unpack_address_merkle_context,
};

pub trait LightAccounts<'a>: Sized {
    fn try_light_accounts(accounts: &'a Option<Vec<LightAccountInfo>>) -> Result<Self>;
    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>>;
}

/// A wrapper which abstracts away the UTXO model.
pub enum LightAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    Init(LightInitAccount<'a, T>),
    Mut(LightMutAccount<'a, T>),
    Close(LightCloseAccount<'a, T>),
}

impl<'a, T> LightAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    pub fn new(account_info: &'a LightAccountInfo) -> Result<Self> {
        match account_info {
            LightAccountInfo::Init(account_info) => {
                Ok(Self::Init(LightInitAccount::new(account_info)))
            }
            LightAccountInfo::Mut(account_info) => {
                Ok(Self::Mut(LightMutAccount::try_from_slice(account_info)?))
            }
            LightAccountInfo::Close(account_info) => Ok(Self::Close(
                LightCloseAccount::try_from_slice(account_info)?,
            )),
        }
    }

    pub fn derive_address(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) {
        match self {
            Self::Init(account) => account.derive_address(seeds, program_id, remaining_accounts),
            Self::Mut(account) => account.derive_address(seeds, program_id, remaining_accounts),
            Self::Close(_) => {}
        }
    }

    pub fn new_address_params(&self) -> Option<PackedNewAddressParams> {
        match self {
            Self::Init(account) => account.new_address_params(),
            Self::Mut(account) => account.new_address_params(),
            Self::Close(_) => None,
        }
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> Option<PackedCompressedAccountWithMerkleContext> {
        match self {
            Self::Init(_) => None,
            Self::Mut(light_mut_account) => {
                Some(light_mut_account.input_compressed_account(program_id))
            }
            Self::Close(light_close_account) => {
                Some(light_close_account.input_compressed_account(program_id))
            }
        }
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self {
            Self::Init(light_init_account) => {
                let account = light_init_account.output_compressed_account(program_id)?;
                Ok(Some(account))
            }
            Self::Mut(light_mut_account) => {
                let account = light_mut_account.output_compressed_account(program_id)?;
                Ok(Some(account))
            }
            Self::Close(_) => Ok(None),
        }
    }
}

impl<'a, T> Deref for LightAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Init(light_init_account) => &light_init_account.account_state,
            Self::Mut(light_mut_account) => &light_mut_account.account_state,
            Self::Close(light_close_account) => &light_close_account.account_state,
        }
    }
}

impl<'a, T> DerefMut for LightAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Init(light_init_account) => &mut light_init_account.account_state,
            Self::Mut(light_mut_account) => &mut light_mut_account.account_state,
            Self::Close(light_close_account) => &mut light_close_account.account_state,
        }
    }
}

pub struct LightInitAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    account_state: T,
    account_info: &'a LightInitAccountInfo,
    new_address_params: Option<PackedNewAddressParams>,
    address: Option<[u8; 32]>,
}

impl<'a, T> LightInitAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    pub fn new(account_info: &'a LightInitAccountInfo) -> Self {
        let account_state = T::default();

        Self {
            account_state,
            account_info,
            new_address_params: None,
            address: None,
        }
    }

    pub fn derive_address(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) {
        let seed = derive_address_seed(seeds, program_id);
        let address_merkle_context = unpack_address_merkle_context(
            self.account_info.address_merkle_context,
            remaining_accounts,
        );
        let address = derive_address(&seed, &address_merkle_context);

        self.new_address_params = Some(PackedNewAddressParams {
            seed,
            address_queue_account_index: self
                .account_info
                .address_merkle_context
                .address_queue_pubkey_index,
            address_merkle_tree_account_index: self
                .account_info
                .address_merkle_context
                .address_merkle_tree_pubkey_index,
            address_merkle_tree_root_index: self.account_info.address_merkle_tree_root_index,
        });
        self.address = Some(address);
    }

    pub fn new_address_params(&self) -> Option<PackedNewAddressParams> {
        self.new_address_params
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> Result<OutputCompressedAccountWithPackedContext> {
        let data = serialize_and_hash_account_data(&self.account_state)?;
        Ok(OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: self.account_info.owner.unwrap_or(*program_id),
                lamports: self.account_info.lamports.unwrap_or(0),
                address: self.address,
                data: Some(data),
            },
            merkle_tree_index: 0,
        })
    }
}

impl<'a, T> Deref for LightInitAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account_state
    }
}

impl<'a, T> DerefMut for LightInitAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account_state
    }
}

pub struct LightMutAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    account_info: &'a LightMutAccountInfo,
    account_state: T,
    new_address_params: Option<PackedNewAddressParams>,
    address: Option<[u8; 32]>,
}

impl<'a, T> LightMutAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(account_info: &'a LightMutAccountInfo) -> Result<Self> {
        let account_state = T::try_from_slice(&account_info.data.as_ref().unwrap())?;

        Ok(Self {
            account_info,
            account_state,
            new_address_params: None,
            address: None,
        })
    }

    pub fn derive_address(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        remaining_accounts: &[AccountInfo],
    ) {
        if let Some(packed_address_merkle_context) = self.account_info.new_address_merkle_context {
            let seed = derive_address_seed(seeds, program_id);
            let address_merkle_context =
                unpack_address_merkle_context(packed_address_merkle_context, remaining_accounts);
            let address = derive_address(&seed, &address_merkle_context);

            self.new_address_params = Some(PackedNewAddressParams {
                seed,
                address_queue_account_index: packed_address_merkle_context
                    .address_queue_pubkey_index,
                address_merkle_tree_account_index: packed_address_merkle_context
                    .address_merkle_tree_pubkey_index,
                address_merkle_tree_root_index: self
                    .account_info
                    .address_merkle_tree_root_index
                    .unwrap(),
            });
            self.address = Some(address);
        }
    }

    pub fn new_address_params(&self) -> Option<PackedNewAddressParams> {
        self.new_address_params
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: self.account_info.owner.unwrap_or(*program_id),
                lamports: self.account_info.lamports.unwrap_or(0),
                address: self.account_info.address,
                data: self
                    .account_info
                    .data_hash
                    .map(|data_hash| CompressedAccountData {
                        discriminator: T::discriminator(),
                        data: Vec::new(),
                        data_hash,
                    }),
            },
            merkle_context: self.account_info.merkle_context,
            root_index: self.account_info.root_index,
            read_only: false,
        }
    }

    pub fn output_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> Result<OutputCompressedAccountWithPackedContext> {
        let data = Some(serialize_and_hash_account_data(&self.account_state)?);
        let compressed_account = CompressedAccount {
            owner: self.account_info.owner.unwrap_or(*program_id),
            lamports: self.account_info.lamports.unwrap_or(0),
            address: self.account_info.address,
            data,
        };
        Ok(OutputCompressedAccountWithPackedContext {
            compressed_account,
            merkle_tree_index: 0,
        })
    }
}

impl<'a, T> Deref for LightMutAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account_state
    }
}

impl<'a, T> DerefMut for LightMutAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account_state
    }
}

pub struct LightCloseAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    account_info: &'a LightCloseAccountInfo,
    account_state: T,
}

impl<'a, T> LightCloseAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(account_info: &'a LightCloseAccountInfo) -> Result<Self> {
        let account_state = T::try_from_slice(&account_info.data.as_ref().unwrap())?;

        Ok(Self {
            account_info,
            account_state,
        })
    }

    pub fn input_compressed_account(
        &self,
        program_id: &Pubkey,
    ) -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: self.account_info.owner.unwrap_or(*program_id),
                lamports: self.account_info.lamports.unwrap_or(0),
                address: self.account_info.address,
                data: self
                    .account_info
                    .data_hash
                    .map(|data_hash| CompressedAccountData {
                        discriminator: T::discriminator(),
                        data: Vec::new(),
                        data_hash,
                    }),
            },
            merkle_context: self.account_info.merkle_context,
            root_index: self.account_info.root_index,
            read_only: false,
        }
    }
}

impl<'a, T> Deref for LightCloseAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.account_state
    }
}

impl<'a, T> DerefMut for LightCloseAccount<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.account_state
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
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

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

impl CompressedAccountWithMerkleContext {
    pub fn new_init_account(
        owner: Pubkey,
        lamports: u64,
        address: Option<[u8; 32]>,
        merkle_context: MerkleContext,
    ) -> Self {
        Self {
            compressed_account: CompressedAccount {
                owner,
                lamports,
                address,
                data: None,
            },
            merkle_context,
        }
    }

    pub fn hash(&self) -> Result<[u8; 32]> {
        self.compressed_account.hash::<Poseidon>(
            &self.merkle_context.merkle_tree_pubkey,
            &self.merkle_context.leaf_index,
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

pub fn serialize_and_hash_account_data<T>(account: &T) -> Result<CompressedAccountData>
where
    T: AnchorSerialize + DataHasher + Discriminator,
{
    let data = account.try_to_vec()?;
    let data_hash = account.hash::<Poseidon>().map_err(ProgramError::from)?;
    Ok(CompressedAccountData {
        discriminator: T::discriminator(),
        data,
        data_hash,
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
