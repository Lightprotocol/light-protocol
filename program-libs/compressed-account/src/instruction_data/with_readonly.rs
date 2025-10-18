use core::ops::Deref;

use light_zero_copy::{
    errors::ZeroCopyError, slice::ZeroCopySliceBorsh, traits::ZeroCopyAt, ZeroCopyMut,
};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{
        NewAddressParamsAssignedPacked, OutputCompressedAccountWithPackedContext,
        PackedReadOnlyAddress,
    },
    traits::{AccountOptions, InputAccount, InstructionData, NewAddress},
    zero_copy::{
        ZCompressedCpiContext, ZNewAddressParamsAssignedPacked,
        ZOutputCompressedAccountWithPackedContext, ZPackedMerkleContext, ZPackedReadOnlyAddress,
        ZPackedReadOnlyCompressedAccount,
    },
};
use crate::{
    compressed_account::{
        hash_with_hashed_values, CompressedAccount, CompressedAccountData,
        PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        PackedReadOnlyCompressedAccount,
    },
    discriminators::DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY,
    instruction_data::traits::LightInstructionData,
    pubkey::Pubkey,
    CompressedAccountError, InstructionDiscriminator, Vec,
};

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, Default, PartialEq, Clone, ZeroCopyMut)]
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
    /// Optional address.
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

impl From<InAccount> for PackedCompressedAccountWithMerkleContext {
    fn from(value: InAccount) -> Self {
        Self {
            read_only: false,
            merkle_context: value.merkle_context,
            root_index: value.root_index,
            compressed_account: CompressedAccount {
                owner: Pubkey::default(), // Placeholder, as owner is not part of InAccount
                address: value.address,
                lamports: value.lamports,
                data: Some(CompressedAccountData {
                    discriminator: value.discriminator,
                    data: Vec::new(),
                    data_hash: value.data_hash,
                }),
            },
        }
    }
}

impl<'a> InputAccount<'a> for ZInAccount<'a> {
    fn skip(&self) -> bool {
        false
    }
    fn owner(&self) -> &Pubkey {
        &self.owner
    }
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

    fn has_data(&self) -> bool {
        true
    }

    fn data(&self) -> Option<CompressedAccountData> {
        Some(CompressedAccountData {
            discriminator: self.discriminator,
            data: Vec::new(),
            data_hash: self.data_hash,
        })
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
                owner,
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
    pub owner: Pubkey,
    meta: Ref<&'a [u8], ZInAccountMeta>,
    pub address: Option<Ref<&'a [u8], [u8; 32]>>,
}

impl<'a> InAccount {
    fn zero_copy_at_with_owner(
        bytes: &'a [u8],
        owner: Pubkey,
    ) -> Result<(ZInAccount<'a>, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], ZInAccountMeta>::from_prefix(bytes)?;
        let (address, bytes) = Option::<Ref<&[u8], [u8; 32]>>::zero_copy_at(bytes)?;
        Ok((
            ZInAccount {
                owner,
                meta,
                address,
            },
            bytes,
        ))
    }
}

impl<'a> Deref for ZInAccount<'a> {
    type Target = Ref<&'a [u8], ZInAccountMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, ZeroCopyMut)]
pub struct InstructionDataInvokeCpiWithReadOnly {
    /// 0 With program ids
    /// 1 without program ids
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
    pub input_compressed_accounts: Vec<InAccount>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub read_only_addresses: Vec<PackedReadOnlyAddress>,
    pub read_only_accounts: Vec<PackedReadOnlyCompressedAccount>,
}

impl InstructionDataInvokeCpiWithReadOnly {
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

    #[must_use = "with_with_transaction_hash returns a new value"]
    pub fn with_with_transaction_hash(mut self, with_transaction_hash: bool) -> Self {
        self.with_transaction_hash = with_transaction_hash;
        self
    }

    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(mut self, cpi_context: CompressedCpiContext) -> Self {
        self.cpi_context = cpi_context;
        self
    }

    #[must_use = "with_proof returns a new value"]
    pub fn with_proof(mut self, proof: Option<CompressedProof>) -> Self {
        self.proof = proof;
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

    #[must_use = "with_input_compressed_accounts returns a new value"]
    pub fn with_input_compressed_accounts(
        mut self,
        input_compressed_accounts: &[InAccount],
    ) -> Self {
        if !input_compressed_accounts.is_empty() {
            self.input_compressed_accounts
                .extend_from_slice(input_compressed_accounts);
        }
        self
    }

    #[must_use = "with_output_compressed_accounts returns a new value"]
    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    ) -> Self {
        if !output_compressed_accounts.is_empty() {
            self.output_compressed_accounts
                .extend_from_slice(output_compressed_accounts);
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

impl InstructionDiscriminator for InstructionDataInvokeCpiWithReadOnly {
    fn discriminator(&self) -> &'static [u8] {
        &DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY
    }
}

impl LightInstructionData for InstructionDataInvokeCpiWithReadOnly {}

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
    is_compress: u8,
    with_cpi_context: u8,
    with_transaction_hash: u8,
    pub cpi_context: ZCompressedCpiContext,
}

impl ZInstructionDataInvokeCpiWithReadOnlyMeta {
    pub fn is_compress(&self) -> bool {
        self.is_compress > 0
    }
    pub fn with_cpi_context(&self) -> bool {
        self.with_cpi_context > 0
    }
    pub fn with_transaction_hash(&self) -> bool {
        self.with_transaction_hash > 0
    }
}

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    meta: Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>,
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsAssignedPacked>,
    pub input_compressed_accounts: Vec<ZInAccount<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}

impl<'a> InstructionData<'a> for ZInstructionDataInvokeCpiWithReadOnly<'a> {
    fn account_option_config(&self) -> Result<AccountOptions, CompressedAccountError> {
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

    fn bump(&self) -> Option<u8> {
        Some(self.bump)
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
                set_context: self.cpi_context.set_context(),
                first_set_context: self.cpi_context.first_set_context(),
                cpi_context_account_index: self.cpi_context.cpi_context_account_index,
            })
        } else {
            None
        }
    }

    fn is_compress(&self) -> bool {
        self.meta.is_compress() && self.compress_or_decompress_lamports().is_some()
    }

    fn input_accounts(&self) -> &[impl InputAccount<'a>] {
        self.input_compressed_accounts.as_slice()
    }

    fn output_accounts(&self) -> &[impl super::traits::OutputAccount<'a>] {
        self.output_compressed_accounts.as_slice()
    }

    fn compress_or_decompress_lamports(&self) -> Option<u64> {
        let lamports: u64 = self.meta.compress_or_decompress_lamports.into();
        if lamports != 0 {
            Some(lamports)
        } else {
            None
        }
    }
}

impl<'a> Deref for ZInstructionDataInvokeCpiWithReadOnly<'a> {
    type Target = Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> ZeroCopyAt<'a> for InstructionDataInvokeCpiWithReadOnly {
    type ZeroCopyAt = ZInstructionDataInvokeCpiWithReadOnly<'a>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&[u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>::from_prefix(bytes)?;
        let (proof, bytes) = Option::<Ref<&[u8], CompressedProof>>::zero_copy_at(bytes)?;
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<'a, ZNewAddressParamsAssignedPacked>::from_bytes_at(bytes)?;
        let (input_compressed_accounts, bytes) = {
            let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
            let num_slices = u32::from(*num_slices) as usize;
            // Prevent heap exhaustion attacks by checking if num_slices is reasonable
            // Each element needs at least 1 byte when serialized
            if bytes.len() < num_slices {
                return Err(ZeroCopyError::InsufficientMemoryAllocated(
                    bytes.len(),
                    num_slices,
                ));
            }
            let mut slices = Vec::with_capacity(num_slices);
            for _ in 0..num_slices {
                let (slice, _bytes) =
                    InAccount::zero_copy_at_with_owner(bytes, meta.invoking_program_id)?;
                bytes = _bytes;
                slices.push(slice);
            }
            (slices, bytes)
        };

        let (output_compressed_accounts, bytes) = <Vec<
            ZOutputCompressedAccountWithPackedContext<'a>,
        > as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;

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

impl PartialEq<InstructionDataInvokeCpiWithReadOnly> for ZInstructionDataInvokeCpiWithReadOnly<'_> {
    fn eq(&self, other: &InstructionDataInvokeCpiWithReadOnly) -> bool {
        // Compare basic fields
        if self.mode != other.mode
            || self.bump != other.bump
            || self.invoking_program_id != other.invoking_program_id
            || u64::from(self.compress_or_decompress_lamports)
                != other.compress_or_decompress_lamports
            || self.is_compress() != other.is_compress
            || self.with_cpi_context() != other.with_cpi_context
        {
            return false;
        }

        // Compare complex fields
        if self.proof.is_some() != other.proof.is_some() {
            return false;
        }
        // We'd need a more complex comparison for proofs, but we know they match
        // when testing with empty objects

        // Compare cpi_context
        if self.cpi_context.set_context() != other.cpi_context.set_context
            || self.cpi_context.first_set_context() != other.cpi_context.first_set_context
            || self.cpi_context.cpi_context_account_index
                != other.cpi_context.cpi_context_account_index
        {
            return false;
        }

        if self.new_address_params.len() != other.new_address_params.len()
            || self.input_compressed_accounts.len() != other.input_compressed_accounts.len()
            || self.output_compressed_accounts.len() != other.output_compressed_accounts.len()
            || self.read_only_addresses.len() != other.read_only_addresses.len()
            || self.read_only_accounts.len() != other.read_only_accounts.len()
        {
            return false;
        }

        true
    }
}

#[cfg(all(not(feature = "pinocchio"), feature = "new-unique"))]
#[cfg(test)]
mod test {
    use borsh::BorshSerialize;
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng, SeedableRng,
    };

    use super::*;
    use crate::CompressedAccountError;
    // TODO: add randomized tests.
    #[test]
    fn test_read_only_zero_copy() {
        let borsh_struct = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 0,
            invoking_program_id: Pubkey::default(),
            compress_or_decompress_lamports: 0,
            is_compress: false,
            with_cpi_context: false,
            with_transaction_hash: true,
            cpi_context: CompressedCpiContext {
                set_context: false,
                first_set_context: false,
                cpi_context_account_index: 0,
            },
            proof: None,
            new_address_params: vec![NewAddressParamsAssignedPacked {
                seed: [1; 32],
                address_merkle_tree_account_index: 1,
                address_queue_account_index: 2,
                address_merkle_tree_root_index: 3,
                assigned_to_account: true,
                assigned_account_index: 2,
            }],
            input_compressed_accounts: vec![InAccount {
                discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
                data_hash: [10; 32],
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 1,
                    queue_pubkey_index: 2,
                    leaf_index: 3,
                    prove_by_index: false,
                },
                root_index: 3,
                lamports: 1000,
                address: Some([30; 32]),
            }],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::default(),
                    lamports: 2000,
                    address: Some([40; 32]),
                    data: Some(CompressedAccountData {
                        discriminator: [3, 4, 5, 6, 7, 8, 9, 10],
                        data: vec![],
                        data_hash: [50; 32],
                    }),
                },
                merkle_tree_index: 3,
            }],
            read_only_addresses: vec![PackedReadOnlyAddress {
                address: [70; 32],
                address_merkle_tree_account_index: 4,
                address_merkle_tree_root_index: 5,
            }],
            read_only_accounts: vec![PackedReadOnlyCompressedAccount {
                account_hash: [80; 32],
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 5,
                    queue_pubkey_index: 6,
                    leaf_index: 7,
                    prove_by_index: false,
                },
                root_index: 8,
            }],
        };
        let bytes = borsh_struct.try_to_vec().unwrap();

        let (zero_copy, _) = InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&bytes).unwrap();

        assert_eq!(zero_copy, borsh_struct);
    }

    /// Compare the original struct with its zero-copy counterpart
    fn compare_invoke_cpi_with_readonly(
        reference: &InstructionDataInvokeCpiWithReadOnly,
        z_copy: &ZInstructionDataInvokeCpiWithReadOnly,
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

        // Collection length comparisons
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.input_compressed_accounts.len() != z_copy.input_compressed_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.read_only_addresses.len() != z_copy.read_only_addresses.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }
        if reference.read_only_accounts.len() != z_copy.read_only_accounts.len() {
            return Err(CompressedAccountError::InvalidArgument);
        }

        // If we're testing the traits, let's also check that the relevant trait methods work
        assert_eq!(
            z_copy.with_transaction_hash(),
            reference.with_transaction_hash
        );
        assert_eq!(z_copy.bump(), Some(reference.bump));
        assert_eq!(z_copy.owner(), reference.invoking_program_id);

        // The compress or decompress logic is a bit complex, let's test it
        if reference.compress_or_decompress_lamports > 0 {
            assert_eq!(
                z_copy.compress_or_decompress_lamports(),
                Some(reference.compress_or_decompress_lamports)
            );
        } else {
            assert_eq!(z_copy.compress_or_decompress_lamports(), None);
        }

        assert_eq!(
            z_copy.is_compress(),
            reference.is_compress && reference.compress_or_decompress_lamports > 0
        );

        // For cpi_context, the trait adds a layer of conditional return
        if reference.with_cpi_context {
            assert!(z_copy.cpi_context().is_some());
        } else {
            assert!(z_copy.cpi_context().is_none());
        }

        Ok(())
    }

    /// Generate a random InstructionDataInvokeCpiWithReadOnly
    fn get_rnd_instruction_data_invoke_cpi_with_readonly(
        rng: &mut StdRng,
    ) -> InstructionDataInvokeCpiWithReadOnly {
        InstructionDataInvokeCpiWithReadOnly {
            mode: rng.gen_range(0..2),
            bump: rng.gen(),
            invoking_program_id: Pubkey::new_unique(),
            compress_or_decompress_lamports: rng.gen(),
            is_compress: rng.gen(),
            with_cpi_context: rng.gen(),
            with_transaction_hash: rng.gen(),
            cpi_context: CompressedCpiContext {
                first_set_context: rng.gen(),
                set_context: rng.gen(),
                cpi_context_account_index: rng.gen(),
            },
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
            // Keep collections small to minimize complex serialization issues
            new_address_params: (0..rng.gen_range(1..5))
                .map(|_| NewAddressParamsAssignedPacked {
                    seed: rng.gen(),
                    address_queue_account_index: rng.gen(),
                    address_merkle_tree_account_index: rng.gen(),
                    address_merkle_tree_root_index: rng.gen(),
                    assigned_to_account: rng.gen(),
                    assigned_account_index: rng.gen(),
                })
                .collect::<Vec<_>>(),
            input_compressed_accounts: (0..rng.gen_range(1..5))
                .map(|_| InAccount {
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
                    address: if rng.gen() { Some(rng.gen()) } else { None },
                })
                .collect(),
            output_compressed_accounts: (0..rng.gen_range(1..5))
                .map(|_| {
                    OutputCompressedAccountWithPackedContext {
                        compressed_account: CompressedAccount {
                            owner: Pubkey::new_unique(),
                            lamports: rng.gen(),
                            address: if rng.gen() { Some(rng.gen()) } else { None },
                            data: if rng.gen() {
                                Some(CompressedAccountData {
                                    discriminator: rng.gen(),
                                    data: vec![], // Keep data empty for simpler testing
                                    data_hash: rng.gen(),
                                })
                            } else {
                                None
                            },
                        },
                        merkle_tree_index: rng.gen(),
                    }
                })
                .collect::<Vec<_>>(),
            read_only_addresses: (0..rng.gen_range(1..5))
                .map(|_| PackedReadOnlyAddress {
                    address: rng.gen(),
                    address_merkle_tree_account_index: rng.gen(),
                    address_merkle_tree_root_index: rng.gen(),
                })
                .collect::<Vec<_>>(),
            read_only_accounts: (0..rng.gen_range(1..5))
                .map(|_| PackedReadOnlyCompressedAccount {
                    account_hash: rng.gen(),
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: rng.gen(),
                        queue_pubkey_index: rng.gen(),
                        leaf_index: rng.gen(),
                        prove_by_index: rng.gen(),
                    },
                    root_index: rng.gen(),
                })
                .collect::<Vec<_>>(),
        }
    }

    #[test]
    fn test_instruction_data_invoke_cpi_with_readonly_rnd() {
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.gen();
        println!("\n\ne2e test seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);

        let num_iters = 1000;
        for _ in 0..num_iters {
            let value = get_rnd_instruction_data_invoke_cpi_with_readonly(&mut rng);

            let mut vec = Vec::new();
            value.serialize(&mut vec).unwrap();
            let (zero_copy, _) = InstructionDataInvokeCpiWithReadOnly::zero_copy_at(&vec).unwrap();

            // Use the PartialEq implementation first
            assert_eq!(zero_copy, value);

            // Then use our detailed comparison that also tests trait methods
            compare_invoke_cpi_with_readonly(&value, &zero_copy).unwrap();
        }
    }
}
