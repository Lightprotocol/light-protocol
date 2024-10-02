use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize, ProgramError, Pubkey, Result};
use light_hasher::{DataHasher, Discriminator, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    context::LightInstructionInputs,
    merkle_context::{pack_merkle_context, MerkleContext, PackedMerkleContext, RemainingAccounts},
};

pub trait LightAccounts: Sized {
    fn try_light_accounts(inputs: &LightInstructionInputs) -> Result<Self>;
    fn output_accounts(&self) -> Result<Vec<OutputCompressedAccountWithPackedContext>>;
}

/// A wrapper which abstracts away the UTXO model.
pub enum LightAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    Init(LightInitAccount<T>),
    Mut(LightMutAccount<T>),
    Close(LightCloseAccount<T>),
}

impl<T> LightAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Default + Discriminator,
{
    pub fn new_init(compressed_account: &CompressedAccount) -> Self {
        Self::Init(LightInitAccount::new(compressed_account))
    }

    pub fn try_from_slice_mut(compressed_account: &CompressedAccount) -> Result<Self> {
        Ok(Self::Mut(LightMutAccount::try_from_slice(
            compressed_account,
        )?))
    }

    pub fn try_from_slice_close(compressed_account: &CompressedAccount) -> Result<Self> {
        Ok(Self::Close(LightCloseAccount::try_from_slice(
            compressed_account,
        )?))
    }

    pub fn output_compressed_account(
        &self,
    ) -> Result<Option<OutputCompressedAccountWithPackedContext>> {
        match self {
            Self::Init(light_init_account) => {
                let account = light_init_account.output_compressed_account()?;
                Ok(Some(account))
            }
            Self::Mut(light_mut_account) => {
                let account = light_mut_account.output_compressed_account()?;
                Ok(Some(account))
            }
            Self::Close(_) => Ok(None),
        }
    }
}

impl<T> Deref for LightAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
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
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
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
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    output_account: T,
    output_compressed_account: CompressedAccount,
}

impl<T> LightInitAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + Default + DataHasher + Discriminator,
{
    pub fn new(compressed_account: &CompressedAccount) -> Self {
        let output_account = T::default();

        // Clean the provided compressed account from data and use it as an
        // output account. The data of output data is going to be assigned in
        // the `output_compressed_account` method.
        let output_compressed_account = CompressedAccount {
            owner: compressed_account.owner,
            lamports: compressed_account.lamports,
            address: compressed_account.address,
            data: None,
        };

        Self {
            output_account,
            output_compressed_account,
        }
    }

    pub fn output_compressed_account(&self) -> Result<OutputCompressedAccountWithPackedContext> {
        let mut compressed_account = self.output_compressed_account.clone();
        let compressed_account_data = serialize_and_hash_account_data(&self.output_account)?;
        compressed_account.data = Some(compressed_account_data);
        Ok(OutputCompressedAccountWithPackedContext {
            compressed_account,
            merkle_tree_index: 0,
        })
    }
}

impl<T> Deref for LightInitAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.output_account
    }
}

impl<T> DerefMut for LightInitAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output_account
    }
}

pub struct LightMutAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    input_account: T,
    output_account: T,
    output_compressed_account: CompressedAccount,
}

impl<T> LightMutAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(compressed_account: &CompressedAccount) -> Result<Self> {
        let account = T::try_from_slice(&compressed_account.data.as_ref().unwrap().data)?;

        // Clean the provided compressed account from data and use it as an
        // output account. The data of output data is going to be assigned in
        // the `output_compressed_account` method.
        let output_compressed_account = CompressedAccount {
            owner: compressed_account.owner,
            lamports: compressed_account.lamports,
            address: compressed_account.address,
            data: None,
        };

        Ok(Self {
            input_account: account.clone(),
            output_account: account,
            output_compressed_account,
        })
    }

    pub fn output_compressed_account(&self) -> Result<OutputCompressedAccountWithPackedContext> {
        let mut compressed_account = self.output_compressed_account.clone();
        let compressed_account_data = serialize_and_hash_account_data(&self.output_account)?;
        compressed_account.data = Some(compressed_account_data);
        Ok(OutputCompressedAccountWithPackedContext {
            compressed_account,
            merkle_tree_index: 0,
        })
    }
}

impl<T> Deref for LightMutAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.output_account
    }
}

impl<T> DerefMut for LightMutAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.output_account
    }
}

pub struct LightCloseAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    input_account: T,
}

impl<T> LightCloseAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    pub fn try_from_slice(compressed_account: &CompressedAccount) -> Result<Self> {
        let input_account = T::try_from_slice(&compressed_account.data.as_ref().unwrap().data)?;

        Ok(Self { input_account })
    }
}

impl<T> Deref for LightCloseAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.input_account
    }
}

impl<T> DerefMut for LightCloseAccount<T>
where
    T: AnchorDeserialize + AnchorSerialize + Clone + DataHasher + Discriminator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_account
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

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct LightAccountInfo {
    pub compressed_account: CompressedAccount,
    pub input_merkle_context: MerkleContext,
    pub output_merkle_context: MerkleContext,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PackedLightAccountInfo {
    pub compressed_account: CompressedAccount,
    pub input_merkle_context: PackedMerkleContext,
    pub output_merkle_context: PackedMerkleContext,
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

pub fn pack_light_account_infos(
    light_account_infos: &[LightAccountInfo],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedLightAccountInfo> {
    light_account_infos
        .iter()
        .map(|x| PackedLightAccountInfo {
            compressed_account: x.compressed_account.clone(),
            input_merkle_context: pack_merkle_context(x.input_merkle_context, remaining_accounts),
            output_merkle_context: pack_merkle_context(x.output_merkle_context, remaining_accounts),
        })
        .collect::<Vec<_>>()
}

pub fn pack_light_account_info(
    light_account_info: LightAccountInfo,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedLightAccountInfo {
    PackedLightAccountInfo {
        compressed_account: light_account_info.compressed_account,
        input_merkle_context: pack_merkle_context(
            light_account_info.input_merkle_context,
            remaining_accounts,
        ),
        output_merkle_context: pack_merkle_context(
            light_account_info.output_merkle_context,
            remaining_accounts,
        ),
    }
}
