use std::{mem::size_of, ops::Deref};

use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use super::invoke_cpi::InstructionDataInvokeCpi;
use crate::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    },
    instruction_data::{
        compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext,
    },
    pubkey::Pubkey,
};

#[repr(C)]
#[derive(
    Debug, PartialEq, Default, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZPackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: U16,
    pub address_merkle_tree_account_index: u8,
}

impl Deserialize for ZPackedReadOnlyAddress {
    type Output<'a> = Self;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (address, bytes) = bytes.split_at(size_of::<[u8; 32]>());
        let (address_merkle_tree_root_index, bytes) = U16::ref_from_prefix(bytes)?;
        let (address_merkle_tree_account_index, bytes) = u8::zero_copy_at(bytes)?;

        Ok((
            ZPackedReadOnlyAddress {
                address: address.try_into().unwrap(),
                address_merkle_tree_root_index: *address_merkle_tree_root_index,
                address_merkle_tree_account_index,
            },
            bytes,
        ))
    }
}

#[repr(C)]
#[derive(
    Debug, PartialEq, Default, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZNewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: U16,
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZPackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: U32,
    prove_by_index: u8,
}

impl ZPackedMerkleContext {
    pub fn prove_by_index(&self) -> bool {
        self.prove_by_index == 1
    }
}

impl Deserialize for ZPackedMerkleContext {
    type Output<'a> = Ref<&'a [u8], Self>;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (ref_value, bytes) = Ref::<&[u8], Self>::from_prefix(bytes)?;
        Ok((ref_value, bytes))
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone)]
pub struct ZOutputCompressedAccountWithPackedContext<'a> {
    pub compressed_account: ZCompressedAccount<'a>,
    pub merkle_tree_index: u8,
}

impl<'a> From<&ZOutputCompressedAccountWithPackedContext<'a>>
    for OutputCompressedAccountWithPackedContext
{
    fn from(output_compressed_account: &ZOutputCompressedAccountWithPackedContext<'a>) -> Self {
        OutputCompressedAccountWithPackedContext {
            compressed_account: (&output_compressed_account.compressed_account).into(),
            merkle_tree_index: output_compressed_account.merkle_tree_index,
        }
    }
}

impl Deserialize for ZOutputCompressedAccountWithPackedContext<'_> {
    type Output<'a> = ZOutputCompressedAccountWithPackedContext<'a>;

    #[inline]
    fn zero_copy_at<'a>(vec: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (compressed_account, bytes) = ZCompressedAccount::zero_copy_at(vec)?;
        let (merkle_tree_index, bytes) = u8::zero_copy_at(bytes)?;
        Ok((
            ZOutputCompressedAccountWithPackedContext {
                compressed_account,
                merkle_tree_index,
            },
            bytes,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ZCompressedAccountData<'a> {
    pub discriminator: Ref<&'a [u8], [u8; 8]>,
    pub data: &'a [u8],
    pub data_hash: Ref<&'a [u8], [u8; 32]>,
}

impl From<ZCompressedAccountData<'_>> for CompressedAccountData {
    fn from(compressed_account_data: ZCompressedAccountData) -> Self {
        CompressedAccountData {
            discriminator: *compressed_account_data.discriminator,
            data: compressed_account_data.data.to_vec(),
            data_hash: *compressed_account_data.data_hash,
        }
    }
}

impl Deserialize for ZCompressedAccountData<'_> {
    type Output<'a> = ZCompressedAccountData<'a>;

    #[inline]
    fn zero_copy_at<'a>(
        bytes: &'a [u8],
    ) -> Result<(ZCompressedAccountData<'a>, &'a [u8]), ZeroCopyError> {
        let (discriminator, bytes) = Ref::<&'a [u8], [u8; 8]>::from_prefix(bytes)?;
        let (len, bytes) = Ref::<&'a [u8], U32>::from_prefix(bytes)?;
        let (data, bytes) = bytes.split_at(u64::from(*len) as usize);
        let (data_hash, bytes) = Ref::<&'a [u8], [u8; 32]>::from_prefix(bytes)?;

        Ok((
            ZCompressedAccountData {
                discriminator,
                data,
                data_hash,
            },
            bytes,
        ))
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, KnownLayout, FromBytes, IntoBytes, Immutable)]
pub struct AccountDesMeta {
    pub owner: Pubkey,
    pub lamports: U64,
    address_option: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ZCompressedAccount<'a> {
    meta: Ref<&'a [u8], AccountDesMeta>,
    pub address: Option<Ref<&'a [u8], [u8; 32]>>,
    pub data: Option<ZCompressedAccountData<'a>>,
}

impl Deref for ZCompressedAccount<'_> {
    type Target = AccountDesMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl From<&ZCompressedAccount<'_>> for CompressedAccount {
    fn from(compressed_account: &ZCompressedAccount) -> Self {
        let data: Option<CompressedAccountData> =
            compressed_account
                .data
                .as_ref()
                .map(|data| CompressedAccountData {
                    discriminator: *data.discriminator,
                    data: data.data.to_vec(),
                    data_hash: *data.data_hash,
                });
        CompressedAccount {
            owner: compressed_account.owner.into(),
            lamports: compressed_account.lamports.into(),
            address: compressed_account.address.map(|x| *x),
            data,
        }
    }
}

impl Deserialize for ZCompressedAccount<'_> {
    type Output<'a> = ZCompressedAccount<'a>;

    #[inline]
    fn zero_copy_at<'a>(
        bytes: &'a [u8],
    ) -> Result<(ZCompressedAccount<'a>, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], AccountDesMeta>::from_prefix(bytes)?;
        let (address, bytes) = if meta.address_option == 1 {
            let (address, bytes) = Ref::<&[u8], [u8; 32]>::zero_copy_at(bytes)?;
            (Some(address), bytes)
        } else {
            (None, bytes)
        };
        let (data, bytes) = Option::<ZCompressedAccountData>::zero_copy_at(bytes)?;
        Ok((
            ZCompressedAccount {
                meta,
                address,
                data,
            },
            bytes,
        ))
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Immutable, KnownLayout, IntoBytes, FromBytes)]
pub struct ZPackedCompressedAccountWithMerkleContextMeta {
    pub merkle_context: ZPackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: U16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    read_only: u8,
}

impl From<ZPackedMerkleContext> for PackedMerkleContext {
    fn from(merkle_context: ZPackedMerkleContext) -> Self {
        PackedMerkleContext {
            merkle_tree_pubkey_index: merkle_context.merkle_tree_pubkey_index,
            nullifier_queue_pubkey_index: merkle_context.nullifier_queue_pubkey_index,
            leaf_index: merkle_context.leaf_index.into(),
            prove_by_index: merkle_context.prove_by_index == 1,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ZPackedCompressedAccountWithMerkleContext<'a> {
    pub compressed_account: ZCompressedAccount<'a>,
    meta: Ref<&'a [u8], ZPackedCompressedAccountWithMerkleContextMeta>,
}

impl From<&ZPackedCompressedAccountWithMerkleContext<'_>>
    for PackedCompressedAccountWithMerkleContext
{
    fn from(packed_compressed_account: &ZPackedCompressedAccountWithMerkleContext<'_>) -> Self {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: (&packed_compressed_account.compressed_account).into(),
            merkle_context: packed_compressed_account.merkle_context.into(),
            root_index: packed_compressed_account.root_index.into(),
            read_only: packed_compressed_account.read_only == 1,
        }
    }
}

impl Deref for ZPackedCompressedAccountWithMerkleContext<'_> {
    type Target = ZPackedCompressedAccountWithMerkleContextMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl Deserialize for ZPackedCompressedAccountWithMerkleContext<'_> {
    type Output<'a> = ZPackedCompressedAccountWithMerkleContext<'a>;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (compressed_account, bytes) = ZCompressedAccount::zero_copy_at(bytes)?;
        let (meta, bytes) =
            Ref::<&[u8], ZPackedCompressedAccountWithMerkleContextMeta>::from_prefix(bytes)?;
        if meta.read_only == 1 {
            unimplemented!("Read only accounts are implemented as a separate instruction.");
        }

        Ok((
            ZPackedCompressedAccountWithMerkleContext {
                compressed_account,
                meta,
            },
            bytes,
        ))
    }
}

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvoke<'a> {
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<ZPackedCompressedAccountWithMerkleContext<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub relay_fee: Option<Ref<&'a [u8], U64>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<Ref<&'a [u8], U64>>,
    pub is_compress: bool,
}

impl Deserialize for ZInstructionDataInvoke<'_> {
    type Output<'a> = ZInstructionDataInvoke<'a>;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (proof, bytes) = Option::<CompressedProof>::zero_copy_at(bytes)?;
        let (input_compressed_accounts_with_merkle_context, bytes) =
            Vec::<ZPackedCompressedAccountWithMerkleContext>::zero_copy_at(bytes)?;
        let (output_compressed_accounts, bytes) =
            Vec::<ZOutputCompressedAccountWithPackedContext>::zero_copy_at(bytes)?;
        let (relay_fee, bytes) = Option::<Ref<&'a [u8], U64>>::zero_copy_at(bytes)?;
        if relay_fee.is_some() {
            return Err(ZeroCopyError::InvalidConversion);
        }
        let (new_address_params, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (compress_or_decompress_lamports, bytes) =
            Option::<Ref<&'a [u8], U64>>::zero_copy_at(bytes)?;
        let (is_compress, bytes) = u8::zero_copy_at(bytes)?;

        Ok((
            ZInstructionDataInvoke {
                proof,
                input_compressed_accounts_with_merkle_context,
                output_compressed_accounts,
                relay_fee: None,
                new_address_params,
                compress_or_decompress_lamports,
                is_compress: is_compress == 1,
            },
            bytes,
        ))
    }
}

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvokeCpi<'a> {
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<ZPackedCompressedAccountWithMerkleContext<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub relay_fee: Option<Ref<&'a [u8], U64>>,
    pub compress_or_decompress_lamports: Option<Ref<&'a [u8], U64>>,
    pub is_compress: bool,
    pub cpi_context: Option<Ref<&'a [u8], ZCompressedCpiContext>>,
}

#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
pub struct ZCompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    set_context: u8,
    /// Is set to clear the cpi context since someone could have set it before
    /// with unrelated data.
    first_set_context: u8,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}

impl ZCompressedCpiContext {
    pub fn set_context(&self) -> bool {
        self.set_context == 1
    }

    pub fn first_set_context(&self) -> bool {
        self.first_set_context == 1
    }
}

impl<'a> From<ZInstructionDataInvokeCpi<'a>> for ZInstructionDataInvoke<'a> {
    fn from(instruction_data_invoke: ZInstructionDataInvokeCpi<'a>) -> Self {
        ZInstructionDataInvoke {
            proof: instruction_data_invoke.proof,
            new_address_params: instruction_data_invoke.new_address_params,
            input_compressed_accounts_with_merkle_context: instruction_data_invoke
                .input_compressed_accounts_with_merkle_context,
            output_compressed_accounts: instruction_data_invoke.output_compressed_accounts,
            relay_fee: instruction_data_invoke.relay_fee,
            compress_or_decompress_lamports: instruction_data_invoke
                .compress_or_decompress_lamports,
            is_compress: instruction_data_invoke.is_compress,
        }
    }
}

impl Deserialize for ZInstructionDataInvokeCpi<'_> {
    type Output<'a> = ZInstructionDataInvokeCpi<'a>;

    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (proof, bytes) = Option::<CompressedProof>::zero_copy_at(bytes)?;
        let (new_address_params, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (input_compressed_accounts_with_merkle_context, bytes) =
            Vec::<ZPackedCompressedAccountWithMerkleContext>::zero_copy_at(bytes)?;
        let (output_compressed_accounts, bytes) =
            Vec::<ZOutputCompressedAccountWithPackedContext>::zero_copy_at(bytes)?;
        let (option_relay_fee, bytes) = bytes.split_at(1);
        if option_relay_fee[0] == 1 {
            return Err(ZeroCopyError::InvalidConversion);
        }
        let (compress_or_decompress_lamports, bytes) =
            Option::<Ref<&'a [u8], U64>>::zero_copy_at(bytes)?;
        let (is_compress, bytes) = u8::zero_copy_at(bytes)?;
        let (cpi_context, bytes) =
            Option::<Ref<&[u8], ZCompressedCpiContext>>::zero_copy_at(bytes)?;

        Ok((
            ZInstructionDataInvokeCpi {
                proof,
                new_address_params,
                input_compressed_accounts_with_merkle_context,
                output_compressed_accounts,
                relay_fee: None,
                compress_or_decompress_lamports,
                is_compress: is_compress == 1,
                cpi_context,
            },
            bytes,
        ))
    }
}

impl Deserialize for CompressedCpiContext {
    type Output<'a> = Self;
    fn zero_copy_at(bytes: &[u8]) -> Result<(Self::Output<'_>, &[u8]), ZeroCopyError> {
        let (first_set_context, bytes) = u8::zero_copy_at(bytes)?;
        let (set_context, bytes) = u8::zero_copy_at(bytes)?;
        let (cpi_context_account_index, bytes) = u8::zero_copy_at(bytes)?;

        Ok((
            CompressedCpiContext {
                first_set_context: first_set_context == 1,
                set_context: set_context == 1,
                cpi_context_account_index,
            },
            bytes,
        ))
    }
}

#[repr(C)]
#[derive(
    Debug, PartialEq, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZPackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: ZPackedMerkleContext,
    pub root_index: U16,
}

impl Deserialize for ZPackedReadOnlyCompressedAccount {
    type Output<'a> = Ref<&'a [u8], Self>;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Self>::from_prefix(bytes)?)
    }
}

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    pub invoke_cpi: ZInstructionDataInvokeCpi<'a>,
    pub read_only_addresses: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>>,
    pub read_only_accounts: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>>,
}

impl Deserialize for ZInstructionDataInvokeCpiWithReadOnly<'_> {
    type Output<'a> = ZInstructionDataInvokeCpiWithReadOnly<'a>;
    fn zero_copy_at<'a>(bytes: &'a [u8]) -> Result<(Self::Output<'a>, &'a [u8]), ZeroCopyError> {
        let (invoke_cpi, bytes) = ZInstructionDataInvokeCpi::zero_copy_at(bytes)?;
        let (read_only_addresses, bytes) =
            Option::<ZeroCopySliceBorsh<ZPackedReadOnlyAddress>>::zero_copy_at(bytes)?;
        let (read_only_accounts, bytes) =
            Option::<ZeroCopySliceBorsh<ZPackedReadOnlyCompressedAccount>>::zero_copy_at(bytes)?;
        Ok((
            ZInstructionDataInvokeCpiWithReadOnly {
                invoke_cpi,
                read_only_addresses,
                read_only_accounts,
            },
            bytes,
        ))
    }
}

impl From<&ZInstructionDataInvokeCpi<'_>> for InstructionDataInvokeCpi {
    fn from(data: &ZInstructionDataInvokeCpi<'_>) -> Self {
        Self {
            proof: None,
            new_address_params: vec![],
            input_compressed_accounts_with_merkle_context: data
                .input_compressed_accounts_with_merkle_context
                .iter()
                .map(PackedCompressedAccountWithMerkleContext::from)
                .collect::<Vec<_>>(),
            output_compressed_accounts: data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect::<Vec<_>>(),
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: data.is_compress,
            cpi_context: None,
        }
    }
}
