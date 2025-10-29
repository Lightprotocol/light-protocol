use core::{mem::size_of, ops::Deref};

use light_zero_copy::{errors::ZeroCopyError, slice::ZeroCopySliceBorsh, traits::ZeroCopyAt};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use super::{
    invoke_cpi::InstructionDataInvokeCpi,
    traits::{AccountOptions, InputAccount, InstructionData, NewAddress, OutputAccount},
};
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
    CompressedAccountError, Vec,
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

impl<'a> ZeroCopyAt<'a> for ZPackedReadOnlyAddress {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
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

impl NewAddress<'_> for ZNewAddressParamsPacked {
    fn seed(&self) -> [u8; 32] {
        self.seed
    }
    fn address_queue_index(&self) -> u8 {
        self.address_queue_account_index
    }

    fn address_merkle_tree_account_index(&self) -> u8 {
        self.address_merkle_tree_account_index
    }

    fn assigned_compressed_account_index(&self) -> Option<usize> {
        None
    }

    fn address_merkle_tree_root_index(&self) -> u16 {
        self.address_merkle_tree_root_index.into()
    }
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZPackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: U32,
    pub prove_by_index: u8,
}

impl ZPackedMerkleContext {
    pub fn prove_by_index(&self) -> bool {
        self.prove_by_index == 1
    }
}

impl<'a> ZeroCopyAt<'a> for ZPackedMerkleContext {
    type ZeroCopyAt = Ref<&'a [u8], Self>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
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

impl<'a> OutputAccount<'a> for ZOutputCompressedAccountWithPackedContext<'a> {
    fn skip(&self) -> bool {
        false
    }
    fn lamports(&self) -> u64 {
        self.compressed_account.lamports.into()
    }
    fn owner(&self) -> Pubkey {
        self.compressed_account.owner
    }

    fn merkle_tree_index(&self) -> u8 {
        self.merkle_tree_index
    }

    fn address(&self) -> Option<[u8; 32]> {
        self.compressed_account.address.map(|x| *x)
    }

    fn has_data(&self) -> bool {
        self.compressed_account.data.is_some()
    }

    fn data(&self) -> Option<CompressedAccountData> {
        self.compressed_account
            .data
            .as_ref()
            .map(|data| data.into())
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], crate::CompressedAccountError> {
        self.compressed_account.hash_with_hashed_values(
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
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

impl<'a> ZeroCopyAt<'a> for ZOutputCompressedAccountWithPackedContext<'a> {
    type ZeroCopyAt = Self;

    #[inline]
    fn zero_copy_at(vec: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
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

impl From<&ZCompressedAccountData<'_>> for CompressedAccountData {
    fn from(compressed_account_data: &ZCompressedAccountData) -> Self {
        CompressedAccountData {
            discriminator: *compressed_account_data.discriminator,
            data: compressed_account_data.data.to_vec(),
            data_hash: *compressed_account_data.data_hash,
        }
    }
}

impl<'a> ZeroCopyAt<'a> for ZCompressedAccountData<'a> {
    type ZeroCopyAt = Self;

    #[inline]
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(ZCompressedAccountData<'a>, &'a [u8]), ZeroCopyError> {
        let (discriminator, bytes) = Ref::<&'a [u8], [u8; 8]>::from_prefix(bytes)?;
        let (len, bytes) = Ref::<&'a [u8], U32>::from_prefix(bytes)?;
        let data_len = u64::from(*len) as usize;
        if bytes.len() < data_len {
            return Err(ZeroCopyError::InvalidConversion);
        }
        let (data, bytes) = bytes.split_at(data_len);
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
            owner: compressed_account.owner,
            lamports: compressed_account.lamports.into(),
            address: compressed_account.address.map(|x| *x),
            data,
        }
    }
}

impl<'a> ZeroCopyAt<'a> for ZCompressedAccount<'a> {
    type ZeroCopyAt = Self;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(ZCompressedAccount<'a>, &'a [u8]), ZeroCopyError> {
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
            queue_pubkey_index: merkle_context.queue_pubkey_index,
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

impl<'a> InputAccount<'a> for ZPackedCompressedAccountWithMerkleContext<'a> {
    fn skip(&self) -> bool {
        false
    }
    fn owner(&self) -> &crate::pubkey::Pubkey {
        &self.compressed_account.owner
    }
    fn lamports(&self) -> u64 {
        self.compressed_account.lamports.into()
    }
    fn address(&self) -> Option<[u8; 32]> {
        self.compressed_account.address.map(|x| *x)
    }

    fn merkle_context(&self) -> ZPackedMerkleContext {
        self.meta.merkle_context
    }

    fn root_index(&self) -> u16 {
        self.meta.root_index.into()
    }

    fn has_data(&self) -> bool {
        self.compressed_account.data.is_some()
    }

    fn data(&self) -> Option<CompressedAccountData> {
        self.compressed_account.data.as_ref().map(|x| x.into())
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], crate::CompressedAccountError> {
        self.compressed_account.hash_with_hashed_values(
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
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

impl<'a> ZeroCopyAt<'a> for ZPackedCompressedAccountWithMerkleContext<'a> {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
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

impl<'a> InstructionData<'a> for ZInstructionDataInvoke<'a> {
    fn bump(&self) -> Option<u8> {
        None
    }
    fn with_transaction_hash(&self) -> bool {
        true
    }
    fn account_option_config(&self) -> Result<AccountOptions, CompressedAccountError> {
        unimplemented!()
    }
    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        None
    }
    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        None
    }
    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>> {
        self.proof
    }
    fn is_compress(&self) -> bool {
        self.is_compress
    }
    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        self.compress_or_decompress_lamports.map(|x| (*x).into())
    }
    fn owner(&self) -> Pubkey {
        // TODO: investigate why this is called if there are no inputs when using mint_to.
        if self
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            Pubkey::default()
        } else {
            self.input_compressed_accounts_with_merkle_context[0]
                .compressed_account
                .owner
        }
    }

    fn new_addresses(&self) -> &[impl NewAddress<'a>] {
        self.new_address_params.as_slice()
    }

    fn input_accounts(&self) -> &[impl InputAccount<'a>] {
        self.input_compressed_accounts_with_merkle_context
            .as_slice()
    }

    fn output_accounts(&self) -> &[impl OutputAccount<'a>] {
        self.output_compressed_accounts.as_slice()
    }

    fn cpi_context(&self) -> Option<CompressedCpiContext> {
        unimplemented!()
    }
}
impl<'a> ZeroCopyAt<'a> for ZInstructionDataInvoke<'a> {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
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

impl ZInstructionDataInvokeCpi<'_> {
    pub fn owner(&self) -> Pubkey {
        if self
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            Pubkey::default()
        } else {
            self.input_compressed_accounts_with_merkle_context[0]
                .compressed_account
                .owner
        }
    }
}

impl<'a> InstructionData<'a> for ZInstructionDataInvokeCpi<'a> {
    fn bump(&self) -> Option<u8> {
        None
    }

    fn with_transaction_hash(&self) -> bool {
        true
    }

    fn account_option_config(&self) -> Result<AccountOptions, CompressedAccountError> {
        let sol_pool_pda = self.compress_or_decompress_lamports().is_some();
        let decompression_recipient = sol_pool_pda && !self.is_compress();
        let cpi_context_account = self.cpi_context().is_some();
        let write_to_cpi_context = false; // Not used

        Ok(AccountOptions {
            sol_pool_pda,
            decompression_recipient,
            cpi_context_account,
            write_to_cpi_context,
        })
    }

    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        None
    }

    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        None
    }

    fn owner(&self) -> Pubkey {
        if self
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            Pubkey::default()
        } else {
            self.input_compressed_accounts_with_merkle_context[0]
                .compressed_account
                .owner
        }
    }

    fn is_compress(&self) -> bool {
        self.is_compress
    }

    fn proof(&self) -> Option<Ref<&'a [u8], CompressedProof>> {
        self.proof
    }

    fn new_addresses(&self) -> &[impl NewAddress<'a>] {
        self.new_address_params.as_slice()
    }

    fn output_accounts(&self) -> &[impl OutputAccount<'a>] {
        self.output_compressed_accounts.as_slice()
    }

    fn input_accounts(&self) -> &[impl InputAccount<'a>] {
        self.input_compressed_accounts_with_merkle_context
            .as_slice()
    }

    fn cpi_context(&self) -> Option<CompressedCpiContext> {
        self.cpi_context
            .as_ref()
            .map(|cpi_context| CompressedCpiContext {
                set_context: cpi_context.set_context(),
                first_set_context: cpi_context.first_set_context(),
                cpi_context_account_index: cpi_context.cpi_context_account_index,
            })
    }

    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        self.compress_or_decompress_lamports.map(|x| (*x).into())
    }
}

#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    FromBytes,
    IntoBytes,
    Immutable,
    Unaligned,
    KnownLayout,
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

impl<'a> ZeroCopyAt<'a> for ZInstructionDataInvokeCpi<'a> {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
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

impl ZeroCopyAt<'_> for CompressedCpiContext {
    type ZeroCopyAt = Self;
    fn zero_copy_at(bytes: &[u8]) -> Result<(Self, &[u8]), ZeroCopyError> {
        let (set_context, bytes) = u8::zero_copy_at(bytes)?;
        let (first_set_context, bytes) = u8::zero_copy_at(bytes)?;
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

impl<'a> ZeroCopyAt<'a> for ZPackedReadOnlyCompressedAccount {
    type ZeroCopyAt = Ref<&'a [u8], Self>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Self>::from_prefix(bytes)?)
    }
}

impl From<&ZInstructionDataInvokeCpi<'_>> for InstructionDataInvokeCpi {
    fn from(data: &ZInstructionDataInvokeCpi<'_>) -> Self {
        Self {
            proof: None,
            new_address_params: crate::vec![],
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

#[repr(C)]
#[derive(
    Debug, PartialEq, Default, Clone, Copy, KnownLayout, Immutable, FromBytes, IntoBytes, Unaligned,
)]
pub struct ZNewAddressParamsAssignedPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: U16,
    pub assigned_to_account: u8,
    pub assigned_account_index: u8,
}

impl NewAddress<'_> for ZNewAddressParamsAssignedPacked {
    fn seed(&self) -> [u8; 32] {
        self.seed
    }
    fn address_queue_index(&self) -> u8 {
        self.address_queue_account_index
    }

    fn address_merkle_tree_account_index(&self) -> u8 {
        self.address_merkle_tree_account_index
    }

    fn assigned_compressed_account_index(&self) -> Option<usize> {
        if self.assigned_to_account > 0 {
            Some(self.assigned_account_index as usize)
        } else {
            None
        }
    }

    fn address_merkle_tree_root_index(&self) -> u16 {
        self.address_merkle_tree_root_index.into()
    }
}

#[cfg(all(not(feature = "pinocchio"), feature = "new-unique"))]
#[cfg(test)]
pub mod test {
    use borsh::BorshSerialize;
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng,
    };

    use super::*;
    use crate::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
            PackedMerkleContext,
        },
        instruction_data::{
            data::{InstructionDataInvoke, NewAddressParamsPacked},
            invoke_cpi::InstructionDataInvokeCpi,
        },
        CompressedAccountError,
    };

    fn get_instruction_data_invoke_cpi() -> InstructionDataInvokeCpi {
        InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            new_address_params: vec![get_new_address_params(); 3],
            input_compressed_accounts_with_merkle_context: vec![get_test_input_account(); 3],
            output_compressed_accounts: vec![get_test_output_account(); 2],
            relay_fee: None,
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            cpi_context: Some(get_cpi_context()),
        }
    }

    fn get_rnd_instruction_data_invoke_cpi(rng: &mut StdRng) -> InstructionDataInvokeCpi {
        InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: rng.gen(),
                b: (0..64)
                    .map(|_| rng.gen())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap(),
                c: rng.gen(),
            }),
            new_address_params: vec![get_rnd_new_address_params(rng); rng.gen_range(0..10)],
            input_compressed_accounts_with_merkle_context: vec![
                get_rnd_test_input_account(rng);
                rng.gen_range(0..10)
            ],
            output_compressed_accounts: vec![
                get_rnd_test_output_account(rng);
                rng.gen_range(0..10)
            ],
            relay_fee: None,
            compress_or_decompress_lamports: rng.gen(),
            is_compress: rng.gen(),
            cpi_context: Some(get_rnd_cpi_context(rng)),
        }
    }

    fn compare_invoke_cpi_instruction_data(
        reference: &InstructionDataInvokeCpi,
        z_copy: &ZInstructionDataInvokeCpi,
    ) -> Result<(), CompressedAccountError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            println!("proof is none");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            println!("proof is some");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_some()
            && z_copy.proof.is_some()
            && reference.proof.as_ref().unwrap().a != z_copy.proof.as_ref().unwrap().a
            || reference.proof.as_ref().unwrap().b != z_copy.proof.as_ref().unwrap().b
            || reference.proof.as_ref().unwrap().c != z_copy.proof.as_ref().unwrap().c
        {
            println!("proof is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            println!("input_compressed_accounts_with_merkle_context is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            println!("output_compressed_accounts is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            println!("relay_fee is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            println!("new_address_params is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                println!("seed is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                println!("address_queue_account_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                println!("address_merkle_tree_account_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                println!("address_merkle_tree_root_index is not equal");
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        if reference.compress_or_decompress_lamports
            != z_copy.compress_or_decompress_lamports.map(|x| (*x).into())
        {
            println!("compress_or_decompress_lamports is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.is_compress != z_copy.is_compress {
            println!("is_compress is not equal");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_none() {
            println!("cpi_context is none");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_none() && z_copy.cpi_context.is_some() {
            println!("cpi_context is some");
            println!("reference: {:?}", reference.cpi_context);
            println!("z_copy: {:?}", z_copy.cpi_context);
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_some() {
            let reference = reference.cpi_context.as_ref().unwrap();
            let zcopy = z_copy.cpi_context.as_ref().unwrap();
            if reference.first_set_context != zcopy.first_set_context()
                || reference.set_context != zcopy.set_context()
                || reference.cpi_context_account_index != zcopy.cpi_context_account_index
            {
                println!("reference: {:?}", reference);
                println!("z_copy: {:?}", zcopy);
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        Ok(())
    }

    #[test]
    fn test_cpi_context_instruction_data() {
        let reference = get_instruction_data_invoke_cpi();

        let mut bytes = Vec::new();
        reference.serialize(&mut bytes).unwrap();
        let (z_copy, bytes) = ZInstructionDataInvokeCpi::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_invoke_cpi_instruction_data(&reference, &z_copy).unwrap();
    }

    fn get_cpi_context() -> CompressedCpiContext {
        CompressedCpiContext {
            first_set_context: true,
            set_context: true,
            cpi_context_account_index: 1,
        }
    }

    fn get_rnd_cpi_context(rng: &mut StdRng) -> CompressedCpiContext {
        CompressedCpiContext {
            first_set_context: rng.gen(),
            set_context: rng.gen(),
            cpi_context_account_index: rng.gen(),
        }
    }

    #[test]
    fn test_cpi_context_deserialize() {
        let cpi_context = get_cpi_context();
        let mut bytes = Vec::new();
        cpi_context.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = CompressedCpiContext::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(z_copy, cpi_context);
    }

    #[test]
    fn test_account_deserialize() {
        let test_account = get_test_account();
        let mut bytes = Vec::new();
        test_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = ZCompressedAccount::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_compressed_account(&test_account, &z_copy).unwrap();
    }

    fn get_test_account_data() -> CompressedAccountData {
        CompressedAccountData {
            discriminator: 1u64.to_le_bytes(),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [1; 32],
        }
    }

    fn get_rnd_test_account_data(rng: &mut StdRng) -> CompressedAccountData {
        CompressedAccountData {
            discriminator: rng.gen(),
            data: (0..100).map(|_| rng.gen()).collect::<Vec<u8>>(),
            data_hash: rng.gen(),
        }
    }

    fn get_test_account() -> CompressedAccount {
        CompressedAccount {
            owner: crate::Pubkey::new_unique(),
            lamports: 100,
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_test_account_data()),
        }
    }

    fn get_rnd_test_account(rng: &mut StdRng) -> CompressedAccount {
        CompressedAccount {
            owner: crate::Pubkey::new_unique(),
            lamports: rng.gen(),
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_rnd_test_account_data(rng)),
        }
    }

    fn get_test_output_account() -> OutputCompressedAccountWithPackedContext {
        OutputCompressedAccountWithPackedContext {
            compressed_account: get_test_account(),
            merkle_tree_index: 1,
        }
    }

    fn get_rnd_test_output_account(rng: &mut StdRng) -> OutputCompressedAccountWithPackedContext {
        OutputCompressedAccountWithPackedContext {
            compressed_account: get_rnd_test_account(rng),
            merkle_tree_index: rng.gen(),
        }
    }

    #[test]
    fn test_output_account_deserialize() {
        let test_output_account = get_test_output_account();
        let mut bytes = Vec::new();
        test_output_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            ZOutputCompressedAccountWithPackedContext::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_compressed_output_account(&test_output_account, &z_copy).unwrap();
    }

    fn compare_compressed_output_account(
        reference: &OutputCompressedAccountWithPackedContext,
        z_copy: &ZOutputCompressedAccountWithPackedContext,
    ) -> Result<(), CompressedAccountError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        if reference.merkle_tree_index != z_copy.merkle_tree_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn get_test_input_account() -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: crate::Pubkey::new_unique(),
                lamports: 100,
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(CompressedAccountData {
                    discriminator: 1u64.to_le_bytes(),
                    data: vec![1, 2, 3, 4, 5, 6, 7, 8],
                    data_hash: [1; 32],
                }),
            },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                queue_pubkey_index: 2,
                leaf_index: 3,
                prove_by_index: true,
            },
            root_index: 5,
            read_only: false,
        }
    }

    fn get_rnd_test_input_account(rng: &mut StdRng) -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: crate::Pubkey::new_unique(),
                lamports: 100,
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(get_rnd_test_account_data(rng)),
            },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen(),
                queue_pubkey_index: rng.gen(),
                leaf_index: rng.gen(),
                prove_by_index: rng.gen(),
            },
            root_index: rng.gen(),
            read_only: false,
        }
    }
    #[test]
    fn test_input_account_deserialize() {
        let input_account = get_test_input_account();

        let mut bytes = Vec::new();
        input_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            ZPackedCompressedAccountWithMerkleContext::zero_copy_at(&bytes).unwrap();

        assert!(bytes.is_empty());
        compare_packed_compressed_account_with_merkle_context(&input_account, &z_copy).unwrap();
    }

    fn get_new_address_params() -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: [1; 32],
            address_queue_account_index: 1,
            address_merkle_tree_account_index: 2,
            address_merkle_tree_root_index: 3,
        }
    }

    // get_instruction_data_invoke_cpi
    fn get_rnd_new_address_params(rng: &mut StdRng) -> NewAddressParamsPacked {
        NewAddressParamsPacked {
            seed: rng.gen(),
            address_queue_account_index: rng.gen(),
            address_merkle_tree_account_index: rng.gen(),
            address_merkle_tree_root_index: rng.gen(),
        }
    }
    #[test]
    fn test_account_data_deserialize() {
        let test_data = CompressedAccountData {
            discriminator: 1u64.to_le_bytes(),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [1; 32],
        };

        let mut bytes = Vec::new();
        test_data.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = ZCompressedAccountData::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(
            z_copy.discriminator.as_slice(),
            test_data.discriminator.as_slice()
        );
        assert_eq!(z_copy.data, test_data.data.as_slice());
        assert_eq!(z_copy.data_hash.as_slice(), test_data.data_hash.as_slice());
    }

    #[test]
    fn test_invoke_ix_data_deserialize_rnd() {
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.gen();
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ne2e test seed for invoke_ix_data {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);

        let num_iters = 1000;
        for i in 0..num_iters {
            // Create randomized instruction data
            let invoke_ref = InstructionDataInvoke {
                proof: if rng.gen() {
                    Some(CompressedProof {
                        a: rng.gen(),
                        b: (0..64)
                            .map(|_| rng.gen())
                            .collect::<Vec<u8>>()
                            .try_into()
                            .unwrap(),
                        c: rng.gen(),
                    })
                } else {
                    None
                },
                input_compressed_accounts_with_merkle_context: if i % 5 == 0 {
                    // Only add inputs occasionally to keep test manageable
                    vec![get_rnd_test_input_account(&mut rng); rng.gen_range(1..3)]
                } else {
                    vec![]
                },
                output_compressed_accounts: if i % 4 == 0 {
                    vec![get_rnd_test_output_account(&mut rng); rng.gen_range(1..3)]
                } else {
                    vec![]
                },
                relay_fee: None, // Relay fee is currently not supported
                new_address_params: if i % 3 == 0 {
                    vec![get_rnd_new_address_params(&mut rng); rng.gen_range(1..3)]
                } else {
                    vec![]
                },
                compress_or_decompress_lamports: if rng.gen() { Some(rng.gen()) } else { None },
                is_compress: rng.gen(),
            };

            let mut bytes = Vec::new();
            invoke_ref.serialize(&mut bytes).unwrap();

            let (z_copy, bytes) = ZInstructionDataInvoke::zero_copy_at(&bytes).unwrap();
            assert!(bytes.is_empty());

            // Compare serialized and deserialized data
            compare_instruction_data(&invoke_ref, &z_copy).unwrap();

            // Test trait methods
            assert!(z_copy.with_transaction_hash()); // Always true for ZInstructionDataInvoke
            assert!(z_copy.bump().is_none()); // Always None for ZInstructionDataInvoke
            assert_eq!(z_copy.is_compress(), invoke_ref.is_compress);
            assert_eq!(
                z_copy.compress_or_decompress_lamports(),
                invoke_ref.compress_or_decompress_lamports
            );

            // The account_option_config() method will call unimplemented!(),
            // so we don't call it directly in the test. Instead, we'll just verify other trait methods.

            // Additional trait method checks
            assert!(z_copy.read_only_accounts().is_none());
            assert!(z_copy.read_only_addresses().is_none());

            // Verify new_addresses() - check that length matches
            assert_eq!(
                z_copy.new_addresses().len(),
                invoke_ref.new_address_params.len()
            );

            // Verify input_accounts() and output_accounts() count matches
            assert_eq!(
                z_copy.input_accounts().len(),
                invoke_ref
                    .input_compressed_accounts_with_merkle_context
                    .len()
            );
            assert_eq!(
                z_copy.output_accounts().len(),
                invoke_ref.output_compressed_accounts.len()
            );

            // Check owner() method returns expected value
            if !invoke_ref
                .input_compressed_accounts_with_merkle_context
                .is_empty()
            {
                let expected_owner: Pubkey = invoke_ref
                    .input_compressed_accounts_with_merkle_context[0]
                    .compressed_account
                    .owner;
                assert_eq!(z_copy.owner(), expected_owner);
            } else {
                assert_eq!(z_copy.owner(), Pubkey::default());
            }
        }
    }

    fn compare_instruction_data(
        reference: &InstructionDataInvoke,
        z_copy: &ZInstructionDataInvoke,
    ) -> Result<(), CompressedAccountError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_some() && z_copy.proof.is_some() {
            let ref_proof = reference.proof.as_ref().unwrap();
            let z_proof = z_copy.proof.as_ref().unwrap();

            if ref_proof.a != z_proof.a || ref_proof.b != z_proof.b || ref_proof.c != z_proof.c {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }
        Ok(())
    }

    fn compare_compressed_account_data(
        reference: &CompressedAccountData,
        z_copy: &ZCompressedAccountData,
    ) -> Result<(), CompressedAccountError> {
        if reference.discriminator.as_slice() != z_copy.discriminator.as_slice() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data != z_copy.data {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data_hash.as_slice() != z_copy.data_hash.as_slice() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn compare_compressed_account(
        reference: &CompressedAccount,
        z_copy: &ZCompressedAccount,
    ) -> Result<(), CompressedAccountError> {
        if reference.owner.to_bytes() != z_copy.owner.as_bytes() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.lamports != u64::from(z_copy.lamports) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.address != z_copy.address.map(|x| *x) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_some() && z_copy.data.is_none() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_none() && z_copy.data.is_some() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.data.is_some() && z_copy.data.is_some() {
            compare_compressed_account_data(
                reference.data.as_ref().unwrap(),
                z_copy.data.as_ref().unwrap(),
            )?;
        }
        Ok(())
    }

    fn compare_merkle_context(
        reference: PackedMerkleContext,
        z_copy: ZPackedMerkleContext,
    ) -> Result<(), CompressedAccountError> {
        if reference.merkle_tree_pubkey_index != z_copy.merkle_tree_pubkey_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.queue_pubkey_index != z_copy.queue_pubkey_index {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.leaf_index != u32::from(z_copy.leaf_index) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.prove_by_index != (z_copy.prove_by_index == 1) {
            return Err(CompressedAccountError::InvalidArgument);
        }
        Ok(())
    }

    fn compare_packed_compressed_account_with_merkle_context(
        reference: &PackedCompressedAccountWithMerkleContext,
        z_copy: &ZPackedCompressedAccountWithMerkleContext,
    ) -> Result<(), CompressedAccountError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        compare_merkle_context(reference.merkle_context, z_copy.merkle_context)?;
        if reference.root_index != u16::from(z_copy.root_index) {
            return Err(CompressedAccountError::InvalidArgument);
        }

        Ok(())
    }

    #[test]
    fn test_instruction_data_invoke_cpi_rnd() {
        use rand::{rngs::StdRng, Rng, SeedableRng};
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.gen();
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ne2e test seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);

        let num_iters = 10000;
        for _ in 0..num_iters {
            let value = get_rnd_instruction_data_invoke_cpi(&mut rng);
            let mut vec = Vec::new();
            value.serialize(&mut vec).unwrap();
            let (zero_copy, _) = ZInstructionDataInvokeCpi::zero_copy_at(&vec).unwrap();
            compare_invoke_cpi_instruction_data(&value, &zero_copy).unwrap();
        }
    }
}
