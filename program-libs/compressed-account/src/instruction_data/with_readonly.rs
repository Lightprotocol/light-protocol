use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};

use crate::{
    compressed_account::{
        hash_with_hashed_values, CompressedAccount, CompressedAccountData,
        PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        PackedReadOnlyCompressedAccount,
    },
    pubkey::Pubkey,
    AnchorDeserialize, AnchorSerialize, CompressedAccountError,
};

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{
        NewAddressParamsPacked, OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
    },
    traits::InputAccountTrait,
    zero_copy::{
        ZCompressedCpiContext, ZNewAddressParamsPacked, ZOutputCompressedAccountWithPackedContext,
        ZPackedMerkleContext, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
    },
};
use std::{env::var, ops::Deref};
use zerocopy::{
    little_endian::{U16, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

#[derive(Debug, Default, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InAccount {
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Lamports.
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
}

impl From<PackedCompressedAccountWithMerkleContext> for InAccount {
    fn from(value: PackedCompressedAccountWithMerkleContext) -> Self {
        Self {
            discriminator: value
                .compressed_account
                .data
                .as_ref()
                .expect("Into InAccount expected data to exist.")
                .discriminator,
            merkle_context: value.merkle_context,
            data_hash: value
                .compressed_account
                .data
                .as_ref()
                .expect("Into InAccount expected data to exist.")
                .data_hash,
            root_index: value.root_index,
            lamports: value.compressed_account.lamports,
            address: value.compressed_account.address,
        }
    }
}

impl<'a> InputAccountTrait<'a> for ZInAccount<'a> {
    fn lamports(&self) -> u64 {
        self.lamports.into()
    }
    fn address(&self) -> Option<[u8; 32]> {
        self.address.map(|x| *x)
    }
    fn merkle_context(&self) -> ZPackedMerkleContext {
        self.merkle_context
    }

    fn root_index(&self) -> u16 {
        self.root_index.into()
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        hash_with_hashed_values(
            &(self.lamports.into()),
            self.address.as_ref().map(|x| x.as_slice()),
            Some((self.discriminator.as_slice(), self.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
}

impl InAccount {
    pub fn into_packed_compressed_account_with_merkle_context(
        &self,
        owner: Pubkey,
    ) -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            read_only: false,
            merkle_context: self.merkle_context,
            root_index: self.root_index,
            compressed_account: CompressedAccount {
                owner: owner.into(),
                address: self.address,
                lamports: self.lamports,
                data: Some(CompressedAccountData {
                    data: Vec::new(),
                    discriminator: self.discriminator,
                    data_hash: self.data_hash,
                }),
            },
        }
    }
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct ZInAccountMeta {
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: ZPackedMerkleContext,
    /// Root index.
    pub root_index: U16,
    /// Lamports.
    pub lamports: U64,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZInAccount<'a> {
    meta: Ref<&'a [u8], ZInAccountMeta>,
    pub address: Option<Ref<&'a [u8], [u8; 32]>>,
}

impl<'a> Deserialize<'a> for InAccount {
    type Output = ZInAccount<'a>;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZInAccountMeta>::from_prefix(bytes)?;
        let (address, bytes) = Option::<Ref<&[u8], [u8; 32]>>::zero_copy_at(bytes)?;
        Ok((Self::Output { meta, address }, bytes))
    }
}

impl<'a> Deref for ZInAccount<'a> {
    type Target = Ref<&'a [u8], ZInAccountMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataInvokeCpiWithReadOnly {
    /// 0 With program ids
    /// 1 without program ids
    pub mode: u8,
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    /// If compress_or_decompress_lamports > 0 -> expect sol_pool_pda
    pub compress_or_decompress_lamports: u64,
    /// -> expect account decompression_recipient
    pub is_decompress: bool,
    pub with_cpi_context: bool,
    pub cpi_context: CompressedCpiContext,
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts: Vec<InAccount>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct ZInstructionDataInvokeCpiWithReadOnlyMeta {
    /// 0 With program ids
    /// 1 without program ids
    pub mode: u8,
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    /// If compress_or_decompress_lamports > 0 -> expect sol_pool_pda
    pub compress_or_decompress_lamports: U64,
    /// -> expect account decompression_recipient
    is_decompress: u8,
    with_cpi_context: u8,
    pub cpi_context: ZCompressedCpiContext,
}

impl ZInstructionDataInvokeCpiWithReadOnlyMeta {
    pub fn is_decompress(&self) -> bool {
        self.is_decompress > 0
    }
    pub fn with_cpi_context(&self) -> bool {
        self.with_cpi_context > 0
    }
}

pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    meta: Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>,
    pub proof: Ref<&'a [u8], Option<CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub input_compressed_accounts: Vec<ZInAccount<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}

impl<'a> Deref for ZInstructionDataInvokeCpiWithReadOnly<'a> {
    type Target = Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> Deserialize<'a> for InstructionDataInvokeCpiWithReadOnly {
    type Output = ZInstructionDataInvokeCpiWithReadOnly<'a>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&[u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>::from_prefix(bytes)?;
        let (proof, bytes) = Ref::<&[u8], Option<CompressedProof>>::from_prefix(bytes)?;
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<'a, ZNewAddressParamsPacked>::from_bytes_at(bytes)?;
        let (input_compressed_accounts, bytes) = Vec::<InAccount>::zero_copy_at(bytes)?;
        let (output_compressed_accounts, bytes) =
            <Vec<ZOutputCompressedAccountWithPackedContext<'a>> as Deserialize<'a>>::zero_copy_at(
                bytes,
            )?;
        let (read_only_addresses, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyAddress>::from_bytes_at(bytes)?;
        let (read_only_accounts, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyCompressedAccount>::from_bytes_at(bytes)?;
        Ok((
            ZInstructionDataInvokeCpiWithReadOnly {
                meta,
                proof,
                new_address_params,
                input_compressed_accounts,
                output_compressed_accounts,
                read_only_addresses,
                read_only_accounts,
            },
            bytes,
        ))
    }
}
