use std::ops::{Deref, DerefMut};

use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};

use crate::{
    compressed_account::{
        hash_with_hashed_values, PackedMerkleContext, PackedReadOnlyCompressedAccount,
    },
    pubkey::Pubkey,
    AnchorDeserialize, AnchorSerialize, CompressedAccountError,
};

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsPacked, PackedReadOnlyAddress},
    traits::{InputAccountTrait, InstructionDataTrait, OutputAccountTrait},
    zero_copy::{
        ZCompressedCpiContext, ZNewAddressParamsPacked, ZPackedMerkleContext,
        ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
    },
};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

/// Issues:
/// 1. we don't have access to owner -> need to pass in as function parameter
/// 2. we have a different struct -> need trait
/// 3. we have cpi context that has to be passed seperately so that we can iterate over the cpi context independently.
///    -> we get rid of the ugly combine method and easily add support for addresses again.
///    -> we need to handle
#[derive(Debug, Default, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InAccountInfo {
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Root index.
    pub root_index: u16,
    /// Lamports.
    pub lamports: u64,
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct ZInAccountInfo {
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

#[derive(Debug, Default, PartialEq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct OutAccountInfo {
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    /// Lamports.
    pub lamports: u64,
    /// Account data.
    pub data: Vec<u8>,
}

impl<'a> InputAccountTrait<'a> for ZCAccountInfo<'a> {
    fn owner(&self) -> &Pubkey {
        &self.owner
    }

    fn lamports(&self) -> u64 {
        self.input.as_ref().unwrap().lamports.into()
    }
    fn address(&self) -> Option<[u8; 32]> {
        self.address.map(|x| *x)
    }

    fn merkle_context(&self) -> ZPackedMerkleContext {
        self.input.as_ref().unwrap().merkle_context
    }

    fn root_index(&self) -> u16 {
        self.input.as_ref().unwrap().root_index.into()
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let input = self.input.as_ref().unwrap();
        let address_slice = self.address.as_ref().map(|x| x.as_slice());
        hash_with_hashed_values(
            &input.lamports.into(),
            address_slice,
            Some((input.discriminator.as_slice(), input.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
}

impl<'a> OutputAccountTrait<'a> for ZCAccountInfo<'a> {
    fn lamports(&self) -> u64 {
        self.output.as_ref().unwrap().lamports.into()
    }

    fn address(&self) -> Option<[u8; 32]> {
        self.address.map(|x| *x)
    }

    fn owner(&self) -> Pubkey {
        self.owner
    }

    fn merkle_tree_index(&self) -> u8 {
        self.output.as_ref().unwrap().output_merkle_tree_index
    }

    fn has_data(&self) -> bool {
        true
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        let output = self.output.as_ref().unwrap();
        let address_slice = self.address.as_ref().map(|x| x.as_slice());
        hash_with_hashed_values(
            &output.lamports.into(),
            address_slice,
            Some((output.discriminator.as_slice(), output.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct ZOutAccountInfoMeta {
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    /// Lamports.
    pub lamports: U64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ZOutAccountInfo<'a> {
    meta: Ref<&'a [u8], ZOutAccountInfoMeta>,
    /// Account data.
    pub data: &'a [u8],
}

impl<'a> Deserialize<'a> for ZOutAccountInfo<'a> {
    type Output = ZOutAccountInfo<'a>;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZOutAccountInfoMeta>::from_prefix(bytes)?;
        let (len, bytes) = Ref::<&'a [u8], U32>::from_prefix(bytes)?;
        let (data, bytes) = bytes.split_at(u64::from(*len) as usize);
        Ok((Self { meta, data }, bytes))
    }
}

impl Deref for ZOutAccountInfo<'_> {
    type Target = ZOutAccountInfoMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

#[derive(Debug, PartialEq)]
pub struct ZOutAccountInfoMut<'a> {
    meta: Ref<&'a mut [u8], ZOutAccountInfoMeta>,
    /// Account data.
    pub data: &'a mut [u8],
}

impl Deref for ZOutAccountInfoMut<'_> {
    type Target = ZOutAccountInfoMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl DerefMut for ZOutAccountInfoMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}

#[derive(Debug, PartialEq, Clone, Default, AnchorSerialize, AnchorDeserialize)]
pub struct CAccountInfo {
    pub discriminator: [u8; 8], // 1
    /// Address.
    pub address: Option<[u8; 32]>, // 2
    /// Input account.
    pub input: Option<InAccountInfo>, // 3
    /// Output account.
    pub output: Option<OutAccountInfo>, // 5
}

pub struct ZCAccountInfo<'a> {
    pub owner: Pubkey,
    pub discriminator: Ref<&'a [u8], [u8; 8]>, // 1
    /// Address.
    pub address: Option<Ref<&'a [u8], [u8; 32]>>, // 2
    /// Input account.
    pub input: Option<Ref<&'a [u8], ZInAccountInfo>>, // 3
    /// Output account.
    pub output: Option<ZOutAccountInfo<'a>>, // 5
}

impl<'a> CAccountInfo {
    pub fn zero_copy_at_with_owner(
        bytes: &'a [u8],
        owner: Pubkey,
    ) -> Result<(ZCAccountInfo<'a>, &'a [u8]), ZeroCopyError> {
        let (discriminator, bytes) = Ref::<&[u8], [u8; 8]>::from_prefix(bytes)?;
        let (address, bytes) = Option::<Ref<&[u8], [u8; 32]>>::zero_copy_at(bytes)?;
        let (input, bytes) = Option::<Ref<&[u8], ZInAccountInfo>>::zero_copy_at(bytes)?;
        let (output, bytes) = Option::<ZOutAccountInfo<'a>>::zero_copy_at(bytes)?;
        Ok((
            ZCAccountInfo {
                owner,
                discriminator,
                address,
                input,
                output,
            },
            bytes,
        ))
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataInvokeCpiWithAccountInfo {
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
    pub account_infos: Vec<CAccountInfo>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

impl<'a> InstructionDataTrait<'a> for ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        Some(self.read_only_accounts.as_slice())
    }

    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        Some(self.read_only_addresses.as_slice())
    }

    fn owner(&self) -> Pubkey {
        self.meta.invoking_program_id
    }

    fn new_addresses(&self) -> &[ZNewAddressParamsPacked] {
        self.new_address_params.as_slice()
    }

    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>> {
        self.proof
    }

    fn cpi_context(&self) -> Option<CompressedCpiContext> {
        if self.meta.with_cpi_context() {
            Some(CompressedCpiContext {
                set_context: self.meta.cpi_context.set_context(),
                first_set_context: self.meta.cpi_context.first_set_context(),
                cpi_context_account_index: self.meta.cpi_context.cpi_context_account_index,
            })
        } else {
            None
        }
    }

    fn is_compress(&self) -> bool {
        !self.meta.is_decompress()
    }

    fn input_accounts(&self) -> &[impl InputAccountTrait<'a>] {
        self.account_infos.as_slice()
    }

    fn output_accounts(&self) -> &[impl super::traits::OutputAccountTrait<'a>] {
        self.account_infos.as_slice()
    }

    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        if self.meta.is_decompress() || self.is_compress() {
            Some(self.meta.compress_or_decompress_lamports.into())
        } else {
            None
        }
    }

    /// TODO: implement
    fn into_instruction_data_invoke_cpi(self) -> super::zero_copy::ZInstructionDataInvokeCpi<'a> {
        todo!()
    }
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct ZInstructionDataInvokeCpiWithAccountInfoMeta {
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

impl ZInstructionDataInvokeCpiWithAccountInfoMeta {
    pub fn is_decompress(&self) -> bool {
        self.is_decompress > 0
    }
    pub fn with_cpi_context(&self) -> bool {
        self.with_cpi_context > 0
    }
}

pub struct ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    meta: Ref<&'a [u8], ZInstructionDataInvokeCpiWithAccountInfoMeta>,
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub account_infos: Vec<ZCAccountInfo<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}

impl<'a> Deref for ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    type Target = Ref<&'a [u8], ZInstructionDataInvokeCpiWithAccountInfoMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> Deserialize<'a> for InstructionDataInvokeCpiWithAccountInfo {
    type Output = ZInstructionDataInvokeCpiWithAccountInfo<'a>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&[u8], ZInstructionDataInvokeCpiWithAccountInfoMeta>::from_prefix(bytes)?;
        let (proof, bytes) = Option::<Ref<&[u8], CompressedProof>>::zero_copy_at(bytes)?;
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<'a, ZNewAddressParamsPacked>::from_bytes_at(bytes)?;
        let (account_infos, bytes) = {
            let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
            let num_slices = u32::from(*num_slices) as usize;
            // TODO: add check that remaining data is enough to read num_slices
            // This prevents agains invalid data allocating a lot of heap memory
            let mut slices = Vec::with_capacity(num_slices);
            for _ in 0..num_slices {
                let (slice, _bytes) =
                    CAccountInfo::zero_copy_at_with_owner(bytes, meta.invoking_program_id)?;
                bytes = _bytes;
                slices.push(slice);
            }
            (slices, bytes)
        };
        let (read_only_addresses, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyAddress>::from_bytes_at(bytes)?;
        let (read_only_accounts, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyCompressedAccount>::from_bytes_at(bytes)?;
        Ok((
            ZInstructionDataInvokeCpiWithAccountInfo {
                meta,
                proof,
                new_address_params,
                account_infos,
                read_only_addresses,
                read_only_accounts,
            },
            bytes,
        ))
    }
}
