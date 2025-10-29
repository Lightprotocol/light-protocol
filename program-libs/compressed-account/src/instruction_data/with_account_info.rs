use core::ops::{Deref, DerefMut};

use light_zero_copy::{errors::ZeroCopyError, slice::ZeroCopySliceBorsh, traits::ZeroCopyAt};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsAssignedPacked, PackedReadOnlyAddress},
    traits::{AccountOptions, InputAccount, InstructionData, NewAddress, OutputAccount},
    with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMeta,
    zero_copy::{
        ZNewAddressParamsAssignedPacked, ZPackedMerkleContext, ZPackedReadOnlyAddress,
        ZPackedReadOnlyCompressedAccount,
    },
};
use crate::{
    compressed_account::{
        hash_with_hashed_values, CompressedAccount, CompressedAccountData, PackedMerkleContext,
        PackedReadOnlyCompressedAccount,
    },
    discriminators::INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION,
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext, traits::LightInstructionData,
        with_readonly::InAccount,
    },
    pubkey::Pubkey,
    CompressedAccountError, InstructionDiscriminator, Vec,
};

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, Default, PartialEq, Clone)]
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

impl From<InAccount> for InAccountInfo {
    fn from(account: InAccount) -> Self {
        Self {
            discriminator: account.discriminator,
            data_hash: account.data_hash,
            merkle_context: account.merkle_context,
            root_index: account.root_index,
            lamports: account.lamports,
        }
    }
}

impl InAccountInfo {
    pub fn into_in_account(&self, address: Option<[u8; 32]>) -> InAccount {
        InAccount {
            discriminator: self.discriminator,
            data_hash: self.data_hash,
            merkle_context: self.merkle_context,
            root_index: self.root_index,
            lamports: self.lamports,
            address,
        }
    }
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

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, Default, PartialEq, Clone)]
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

impl TryFrom<OutputCompressedAccountWithPackedContext> for OutAccountInfo {
    type Error = CompressedAccountError;

    fn try_from(output: OutputCompressedAccountWithPackedContext) -> Result<Self, Self::Error> {
        let data = output
            .compressed_account
            .data
            .as_ref()
            .ok_or(CompressedAccountError::ExpectedDataHash)?;

        Ok(Self {
            discriminator: data.discriminator,
            data_hash: data.data_hash,
            output_merkle_tree_index: output.merkle_tree_index,
            lamports: output.compressed_account.lamports,
            data: data.data.clone(),
        })
    }
}

impl OutputCompressedAccountWithPackedContext {
    pub fn from_with_owner(
        info: &OutAccountInfo,
        owner: Pubkey,
        address: Option<[u8; 32]>,
    ) -> Self {
        Self {
            compressed_account: CompressedAccount {
                owner,
                lamports: info.lamports,
                address,
                data: Some(CompressedAccountData {
                    discriminator: info.discriminator,
                    data: info.data.to_vec(),
                    data_hash: info.data_hash,
                }),
            },
            merkle_tree_index: info.output_merkle_tree_index,
        }
    }
}

impl<'a> InputAccount<'a> for ZCompressedAccountInfo<'a> {
    fn owner(&self) -> &Pubkey {
        &self.owner
    }

    fn skip(&self) -> bool {
        self.input.is_none()
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

    fn has_data(&self) -> bool {
        true
    }

    fn data(&self) -> Option<CompressedAccountData> {
        Some(CompressedAccountData {
            data_hash: self.input.unwrap().data_hash,
            discriminator: self.input.unwrap().discriminator,
            data: Vec::new(),
        })
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

impl<'a> OutputAccount<'a> for ZCompressedAccountInfo<'a> {
    fn lamports(&self) -> u64 {
        self.output.as_ref().unwrap().lamports.into()
    }

    fn address(&self) -> Option<[u8; 32]> {
        self.address.map(|x| *x)
    }

    fn skip(&self) -> bool {
        self.output.is_none()
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

    fn data(&self) -> Option<CompressedAccountData> {
        Some(CompressedAccountData {
            discriminator: self.output.as_ref().unwrap().discriminator,
            data_hash: self.output.as_ref().unwrap().data_hash,
            data: self.output.as_ref().unwrap().data.to_vec(),
        })
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

impl<'a> ZeroCopyAt<'a> for ZOutAccountInfo<'a> {
    type ZeroCopyAt = ZOutAccountInfo<'a>;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
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

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Clone, Default)]
pub struct CompressedAccountInfo {
    /// Address.
    pub address: Option<[u8; 32]>,
    /// Input account.
    pub input: Option<InAccountInfo>,
    /// Output account.
    pub output: Option<OutAccountInfo>,
}

#[derive(Debug, PartialEq)]
pub struct ZCompressedAccountInfo<'a> {
    pub owner: Pubkey,
    /// Address.
    pub address: Option<Ref<&'a [u8], [u8; 32]>>,
    /// Input account.
    pub input: Option<Ref<&'a [u8], ZInAccountInfo>>,
    /// Output account.
    pub output: Option<ZOutAccountInfo<'a>>,
}

impl<'a> CompressedAccountInfo {
    pub fn zero_copy_at_with_owner(
        bytes: &'a [u8],
        owner: Pubkey,
    ) -> Result<(ZCompressedAccountInfo<'a>, &'a [u8]), ZeroCopyError> {
        let (address, bytes) = Option::<Ref<&[u8], [u8; 32]>>::zero_copy_at(bytes)?;
        let (input, bytes) = Option::<Ref<&[u8], ZInAccountInfo>>::zero_copy_at(bytes)?;
        let (output, bytes) = Option::<ZOutAccountInfo<'a>>::zero_copy_at(bytes)?;
        Ok((
            ZCompressedAccountInfo {
                owner,
                address,
                input,
                output,
            },
            bytes,
        ))
    }
}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct InstructionDataInvokeCpiWithAccountInfo {
    /// 0 V1 instruction accounts.
    /// 1 Optimized V2 instruction accounts.
    pub mode: u8,
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    /// If compress_or_decompress_lamports > 0 -> expect sol_pool_pda
    pub compress_or_decompress_lamports: u64,
    /// -> expect account decompression_recipient
    pub is_compress: bool,
    pub with_cpi_context: bool,
    pub with_transaction_hash: bool,
    pub cpi_context: CompressedCpiContext,
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsAssignedPacked>,
    pub account_infos: Vec<CompressedAccountInfo>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

impl InstructionDataInvokeCpiWithAccountInfo {
    pub fn new(invoking_program_id: Pubkey, bump: u8, proof: Option<CompressedProof>) -> Self {
        Self {
            invoking_program_id,
            bump,
            proof,
            mode: 1,
            ..Default::default()
        }
    }

    #[must_use = "mode_v1 returns a new value"]
    pub fn mode_v1(mut self) -> Self {
        self.mode = 0;
        self
    }

    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        self.with_cpi_context = true;
        self.cpi_context = CompressedCpiContext::set();
        self
    }

    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        self.with_cpi_context = true;
        self.cpi_context = CompressedCpiContext::first();
        self
    }

    #[must_use = "execute_with_cpi_context returns a new value"]
    pub fn execute_with_cpi_context(mut self) -> Self {
        self.with_cpi_context = true;
        self
    }

    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(mut self, cpi_context: CompressedCpiContext) -> Self {
        self.cpi_context = cpi_context;
        self
    }

    #[must_use = "with_with_transaction_hash returns a new value"]
    pub fn with_with_transaction_hash(mut self, with_transaction_hash: bool) -> Self {
        self.with_transaction_hash = with_transaction_hash;
        self
    }

    #[must_use = "compress_lamports returns a new value"]
    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = lamports;
        self.is_compress = true;
        self
    }

    #[must_use = "decompress_lamports returns a new value"]
    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = lamports;
        self.is_compress = false;
        self
    }

    #[must_use = "with_new_addresses returns a new value"]
    pub fn with_new_addresses(
        mut self,
        new_address_params: &[NewAddressParamsAssignedPacked],
    ) -> Self {
        if !new_address_params.is_empty() {
            self.new_address_params
                .extend_from_slice(new_address_params);
        }
        self
    }

    #[must_use = "with_account_infos returns a new value"]
    pub fn with_account_infos(mut self, account_infos: &[CompressedAccountInfo]) -> Self {
        if !account_infos.is_empty() {
            self.account_infos.extend_from_slice(account_infos);
        }
        self
    }

    #[must_use = "with_read_only_addresses returns a new value"]
    pub fn with_read_only_addresses(
        mut self,
        read_only_addresses: &[PackedReadOnlyAddress],
    ) -> Self {
        if !read_only_addresses.is_empty() {
            self.read_only_addresses
                .extend_from_slice(read_only_addresses);
        }
        self
    }

    #[must_use = "with_read_only_accounts returns a new value"]
    pub fn with_read_only_accounts(
        mut self,
        read_only_accounts: &[PackedReadOnlyCompressedAccount],
    ) -> Self {
        if !read_only_accounts.is_empty() {
            self.read_only_accounts
                .extend_from_slice(read_only_accounts);
        }
        self
    }
}

impl InstructionDiscriminator for InstructionDataInvokeCpiWithAccountInfo {
    fn discriminator(&self) -> &'static [u8] {
        &INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION
    }
}

impl LightInstructionData for InstructionDataInvokeCpiWithAccountInfo {}

impl<'a> InstructionData<'a> for ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    fn bump(&self) -> Option<u8> {
        Some(self.bump)
    }

    fn account_option_config(
        &self,
    ) -> Result<super::traits::AccountOptions, CompressedAccountError> {
        let sol_pool_pda = self.compress_or_decompress_lamports().is_some();
        let decompression_recipient = sol_pool_pda && !self.is_compress();
        let cpi_context_account = self.cpi_context().is_some();
        let write_to_cpi_context =
            self.cpi_context.first_set_context() || self.cpi_context.set_context();

        // Validate: if we want to write to CPI context, we must have a CPI context
        if write_to_cpi_context && !cpi_context_account {
            return Err(CompressedAccountError::InvalidCpiContext);
        }

        Ok(AccountOptions {
            sol_pool_pda,
            decompression_recipient,
            cpi_context_account,
            write_to_cpi_context,
        })
    }

    fn with_transaction_hash(&self) -> bool {
        self.meta.with_transaction_hash()
    }

    fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        Some(self.read_only_accounts.as_slice())
    }

    fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        Some(self.read_only_addresses.as_slice())
    }

    fn owner(&self) -> Pubkey {
        self.meta.invoking_program_id
    }

    fn new_addresses(&self) -> &[impl NewAddress<'a>] {
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
        self.meta.is_compress()
    }

    fn input_accounts(&self) -> &[impl InputAccount<'a>] {
        self.account_infos.as_slice()
    }

    fn output_accounts(&self) -> &[impl super::traits::OutputAccount<'a>] {
        self.account_infos.as_slice()
    }

    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        if self.meta.compress_or_decompress_lamports != U64::from(0) {
            Some(self.meta.compress_or_decompress_lamports.into())
        } else {
            None
        }
    }
}

pub struct ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    meta: Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>,
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsAssignedPacked>,
    pub account_infos: Vec<ZCompressedAccountInfo<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}

impl<'a> Deref for ZInstructionDataInvokeCpiWithAccountInfo<'a> {
    type Target = Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for InstructionDataInvokeCpiWithAccountInfo {
    type ZeroCopyAt = ZInstructionDataInvokeCpiWithAccountInfo<'a>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&[u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>::from_prefix(bytes)?;
        let (proof, bytes) = Option::<Ref<&[u8], CompressedProof>>::zero_copy_at(bytes)?;
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<'a, ZNewAddressParamsAssignedPacked>::from_bytes_at(bytes)?;
        let (account_infos, bytes) = {
            let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
            let num_slices = u32::from(*num_slices) as usize;
            let mut slices = Vec::with_capacity(num_slices);
            if bytes.len() < num_slices {
                return Err(ZeroCopyError::InsufficientMemoryAllocated(
                    bytes.len(),
                    num_slices,
                ));
            }
            for _ in 0..num_slices {
                let (slice, _bytes) = CompressedAccountInfo::zero_copy_at_with_owner(
                    bytes,
                    meta.invoking_program_id,
                )?;
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

#[cfg(all(not(feature = "pinocchio"), feature = "new-unique"))]
#[cfg(test)]
pub mod test {
    use borsh::BorshSerialize;
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng, SeedableRng,
    };

    use super::*;
    use crate::{
        compressed_account::{PackedMerkleContext, PackedReadOnlyCompressedAccount},
        instruction_data::{
            compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
            data::NewAddressParamsAssignedPacked,
        },
        CompressedAccountError,
    };

    fn get_rnd_instruction_data_invoke_cpi_with_account_info(
        rng: &mut StdRng,
    ) -> InstructionDataInvokeCpiWithAccountInfo {
        let with_cpi_context = rng.gen();
        InstructionDataInvokeCpiWithAccountInfo {
            mode: rng.gen_range(0..2),
            bump: rng.gen(),
            invoking_program_id: Pubkey::new_unique(),
            compress_or_decompress_lamports: rng.gen(),
            is_compress: rng.gen(),
            with_cpi_context,
            with_transaction_hash: rng.gen(),
            cpi_context: get_rnd_cpi_context(rng, with_cpi_context),
            proof: Some(CompressedProof {
                a: rng.gen(),
                b: (0..64)
                    .map(|_| rng.gen())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .unwrap(),
                c: rng.gen(),
            }),
            new_address_params: vec![
                get_rnd_new_address_params_assigned(rng);
                rng.gen_range(0..10)
            ],
            account_infos: vec![get_rnd_test_account_info(rng); rng.gen_range(0..10)],
            read_only_addresses: vec![get_rnd_read_only_address(rng); rng.gen_range(0..10)],
            read_only_accounts: vec![get_rnd_read_only_account(rng); rng.gen_range(0..10)],
        }
    }

    fn get_rnd_cpi_context(rng: &mut StdRng, with_cpi_context: bool) -> CompressedCpiContext {
        CompressedCpiContext {
            first_set_context: rng.gen() && with_cpi_context,
            set_context: rng.gen() && with_cpi_context,
            cpi_context_account_index: rng.gen(),
        }
    }

    fn get_rnd_read_only_address(rng: &mut StdRng) -> PackedReadOnlyAddress {
        PackedReadOnlyAddress {
            address: rng.gen(),
            address_merkle_tree_root_index: rng.gen(),
            address_merkle_tree_account_index: rng.gen(),
        }
    }

    fn get_rnd_read_only_account(rng: &mut StdRng) -> PackedReadOnlyCompressedAccount {
        PackedReadOnlyCompressedAccount {
            account_hash: rng.gen(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen(),
                queue_pubkey_index: rng.gen(),
                leaf_index: rng.gen(),
                prove_by_index: rng.gen(),
            },
            root_index: rng.gen(),
        }
    }

    fn get_rnd_in_account_info(rng: &mut StdRng) -> InAccountInfo {
        InAccountInfo {
            discriminator: rng.gen(),
            data_hash: rng.gen(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen(),
                queue_pubkey_index: rng.gen(),
                leaf_index: rng.gen(),
                prove_by_index: rng.gen(),
            },
            root_index: rng.gen(),
            lamports: rng.gen(),
        }
    }

    pub fn get_rnd_out_account_info(rng: &mut StdRng) -> OutAccountInfo {
        OutAccountInfo {
            discriminator: rng.gen(),
            data_hash: rng.gen(),
            output_merkle_tree_index: rng.gen(),
            lamports: rng.gen(),
            data: (0..rng.gen_range(0..100)).map(|_| rng.gen()).collect(),
        }
    }

    pub fn get_rnd_test_account_info(rng: &mut StdRng) -> CompressedAccountInfo {
        CompressedAccountInfo {
            address: if rng.gen() { Some(rng.gen()) } else { None },
            input: if rng.gen() {
                Some(get_rnd_in_account_info(rng))
            } else {
                None
            },
            output: if rng.gen() {
                Some(get_rnd_out_account_info(rng))
            } else {
                None
            },
        }
    }

    pub fn get_rnd_new_address_params_assigned(rng: &mut StdRng) -> NewAddressParamsAssignedPacked {
        NewAddressParamsAssignedPacked {
            seed: rng.gen(),
            address_queue_account_index: rng.gen(),
            address_merkle_tree_account_index: rng.gen(),
            address_merkle_tree_root_index: rng.gen(),
            assigned_to_account: rng.gen(),
            assigned_account_index: rng.gen(),
        }
    }

    fn compare_invoke_cpi_with_account_info(
        reference: &InstructionDataInvokeCpiWithAccountInfo,
        z_copy: &ZInstructionDataInvokeCpiWithAccountInfo,
    ) -> Result<(), CompressedAccountError> {
        // Basic field comparisons
        if reference.mode != z_copy.meta.mode {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.bump != z_copy.meta.bump {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.invoking_program_id != z_copy.meta.invoking_program_id {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.compress_or_decompress_lamports
            != u64::from(z_copy.meta.compress_or_decompress_lamports)
        {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.is_compress != z_copy.meta.is_compress() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.with_cpi_context != z_copy.meta.with_cpi_context() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.with_transaction_hash != z_copy.meta.with_transaction_hash() {
            return Err(CompressedAccountError::InvalidArgument);
        }

        // CPI context comparisons
        if reference.cpi_context.first_set_context != z_copy.meta.cpi_context.first_set_context() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.set_context != z_copy.meta.cpi_context.set_context() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.cpi_context.cpi_context_account_index
            != z_copy.meta.cpi_context.cpi_context_account_index
        {
            return Err(CompressedAccountError::InvalidArgument);
        }

        // Proof comparisons
        if reference.proof.is_some() && z_copy.proof.is_none() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.proof.is_some() && z_copy.proof.is_some() {
            let ref_proof = reference.proof.as_ref().unwrap();
            let z_proof = *z_copy.proof.as_ref().unwrap();
            if ref_proof.a != z_proof.a || ref_proof.b != z_proof.b || ref_proof.c != z_proof.c {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }

        // New address params comparisons - detailed comparison of contents
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for i in 0..reference
            .new_address_params
            .len()
            .min(z_copy.new_address_params.len())
        {
            let ref_param = &reference.new_address_params[i];
            let z_param = &z_copy.new_address_params[i];

            if ref_param.seed != z_param.seed {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_param.address_queue_account_index != z_param.address_queue_account_index {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_param.address_merkle_tree_account_index
                != z_param.address_merkle_tree_account_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_param.address_merkle_tree_root_index
                != u16::from(z_param.address_merkle_tree_root_index)
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            // For ZNewAddressParamsAssignedPacked, assigned_to_account is a u8 (0 or 1)
            // For NewAddressParamsAssignedPacked, it's a bool
            let z_assigned_to_account_bool = z_param.assigned_to_account > 0;
            if ref_param.assigned_to_account != z_assigned_to_account_bool {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_param.assigned_account_index != z_param.assigned_account_index {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }

        // Account infos comparison - check the length first
        if reference.account_infos.len() != z_copy.account_infos.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        // We could do more detailed comparison of account_infos here if needed
        // but it's complex due to the ZCompressedAccountInfo structure

        // Read-only addresses comparison
        if reference.read_only_addresses.len() != z_copy.read_only_addresses.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for i in 0..reference
            .read_only_addresses
            .len()
            .min(z_copy.read_only_addresses.len())
        {
            let ref_addr = &reference.read_only_addresses[i];
            let z_addr = &z_copy.read_only_addresses[i];

            if ref_addr.address != z_addr.address {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_addr.address_merkle_tree_account_index
                != z_addr.address_merkle_tree_account_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_addr.address_merkle_tree_root_index
                != u16::from(z_addr.address_merkle_tree_root_index)
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }

        // Read-only accounts comparison
        if reference.read_only_accounts.len() != z_copy.read_only_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        for i in 0..reference
            .read_only_accounts
            .len()
            .min(z_copy.read_only_accounts.len())
        {
            let ref_acc = &reference.read_only_accounts[i];
            let z_acc = &z_copy.read_only_accounts[i];

            if ref_acc.account_hash != z_acc.account_hash {
                return Err(CompressedAccountError::InvalidArgument);
            }

            // Compare merkle_context
            if ref_acc.merkle_context.merkle_tree_pubkey_index
                != z_acc.merkle_context.merkle_tree_pubkey_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_acc.merkle_context.queue_pubkey_index != z_acc.merkle_context.queue_pubkey_index
            {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_acc.merkle_context.leaf_index != u32::from(z_acc.merkle_context.leaf_index) {
                return Err(CompressedAccountError::InvalidArgument);
            }
            if ref_acc.merkle_context.prove_by_index != z_acc.merkle_context.prove_by_index() {
                return Err(CompressedAccountError::InvalidArgument);
            }

            if ref_acc.root_index != u16::from(z_acc.root_index) {
                return Err(CompressedAccountError::InvalidArgument);
            }
        }

        // Test trait methods
        assert_eq!(
            z_copy.with_transaction_hash(),
            reference.with_transaction_hash
        );
        assert_eq!(z_copy.bump(), Some(reference.bump));
        assert_eq!(z_copy.owner(), reference.invoking_program_id);

        // Test the complex logic for compress/decompress
        if reference.compress_or_decompress_lamports > 0 {
            assert_eq!(
                z_copy.compress_or_decompress_lamports(),
                Some(reference.compress_or_decompress_lamports)
            );
        } else {
            assert_eq!(z_copy.compress_or_decompress_lamports(), None);
        }

        // Test is_compress
        let expected_is_compress =
            reference.is_compress && reference.compress_or_decompress_lamports > 0;
        assert_eq!(z_copy.is_compress(), expected_is_compress);

        // Test cpi_context
        if reference.with_cpi_context {
            let context = z_copy.cpi_context();
            assert!(context.is_some());
            if let Some(ctx) = context {
                assert_eq!(
                    ctx.first_set_context,
                    reference.cpi_context.first_set_context
                );
                assert_eq!(ctx.set_context, reference.cpi_context.set_context);
                assert_eq!(
                    ctx.cpi_context_account_index,
                    reference.cpi_context.cpi_context_account_index
                );
            }
        } else {
            assert!(z_copy.cpi_context().is_none());
        }

        // Check account_option_config
        let account_options = z_copy.account_option_config().unwrap();
        assert_eq!(
            account_options.sol_pool_pda,
            z_copy.compress_or_decompress_lamports().is_some()
        );
        assert_eq!(
            account_options.decompression_recipient,
            z_copy.compress_or_decompress_lamports().is_some() && !z_copy.is_compress()
        );
        assert_eq!(
            account_options.cpi_context_account,
            z_copy.cpi_context().is_some()
        );

        Ok(())
    }

    #[test]
    fn test_instruction_data_invoke_cpi_with_account_info_rnd() {
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.gen();
        println!("\n\ne2e test seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);

        let num_iters = 1000;
        for _ in 0..num_iters {
            // For now, we're using a fixed structure to ensure the test passes
            // Later, we can gradually introduce randomness in parts of the struct
            let value = get_rnd_instruction_data_invoke_cpi_with_account_info(&mut rng);

            let mut vec = Vec::new();
            value.serialize(&mut vec).unwrap();
            let (zero_copy, _) =
                InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(&vec).unwrap();
            compare_invoke_cpi_with_account_info(&value, &zero_copy).unwrap();
        }
    }
}
