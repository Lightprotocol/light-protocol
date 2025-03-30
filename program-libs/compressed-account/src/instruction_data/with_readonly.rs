use std::ops::Deref;

use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use super::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{
        NewAddressParamsPacked, OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
    },
    traits::{AccountOptions, InputAccountTrait, InstructionDataTrait},
    zero_copy::{
        ZCompressedCpiContext, ZNewAddressParamsPacked, ZOutputCompressedAccountWithPackedContext,
        ZPackedMerkleContext, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
    },
};
use crate::{
    compressed_account::{
        hash_with_hashed_values, CompressedAccount, CompressedAccountData,
        PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        PackedReadOnlyCompressedAccount,
    },
    pubkey::Pubkey,
    AnchorDeserialize, AnchorSerialize, CompressedAccountError,
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

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    meta: Ref<&'a [u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>,
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub input_compressed_accounts: Vec<ZInAccount<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}

impl<'a> InstructionDataTrait<'a> for ZInstructionDataInvokeCpiWithReadOnly<'a> {
    fn account_option_config(&self) -> AccountOptions {
        AccountOptions {
            sol_pool_pda: self.is_compress(),
            decompression_recipient: self.compress_or_decompress_lamports().is_some()
                && !self.is_compress(),
            cpi_context_account: self.cpi_context().is_some(),
        }
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

    fn new_addresses(&self) -> &[ZNewAddressParamsPacked] {
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
        !self.meta.is_decompress() && self.compress_or_decompress_lamports().is_some()
    }

    fn input_accounts(&self) -> &[impl InputAccountTrait<'a>] {
        self.input_compressed_accounts.as_slice()
    }

    fn output_accounts(&self) -> &[impl super::traits::OutputAccountTrait<'a>] {
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

impl<'a> Deserialize<'a> for InstructionDataInvokeCpiWithReadOnly {
    type Output = ZInstructionDataInvokeCpiWithReadOnly<'a>;
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) =
            Ref::<&[u8], ZInstructionDataInvokeCpiWithReadOnlyMeta>::from_prefix(bytes)?;
        let (proof, bytes) = Option::<Ref<&[u8], CompressedProof>>::zero_copy_at(bytes)?;
        let (new_address_params, bytes) =
            ZeroCopySliceBorsh::<'a, ZNewAddressParamsPacked>::from_bytes_at(bytes)?;
        let (input_compressed_accounts, bytes) = {
            let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
            let num_slices = u32::from(*num_slices) as usize;
            // TODO: add check that remaining data is enough to read num_slices
            // This prevents agains invalid data allocating a lot of heap memory
            let mut slices = Vec::with_capacity(num_slices);
            for _ in 0..num_slices {
                let (slice, _bytes) =
                    InAccount::zero_copy_at_with_owner(bytes, meta.invoking_program_id)?;
                bytes = _bytes;
                slices.push(slice);
            }
            (slices, bytes)
        };
        #[cfg(feature = "pinocchio")]
        pinocchio::msg!("post inputs");
        let (output_compressed_accounts, bytes) =
            <Vec<ZOutputCompressedAccountWithPackedContext<'a>> as Deserialize<'a>>::zero_copy_at(
                bytes,
            )?;
        #[cfg(feature = "pinocchio")]
        pinocchio::msg!("post output_compressed_accounts");
        let (read_only_addresses, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyAddress>::from_bytes_at(bytes)?;
        #[cfg(feature = "pinocchio")]
        pinocchio::msg!("post read_only_addresses");
        let (read_only_accounts, bytes) =
            ZeroCopySliceBorsh::<'a, ZPackedReadOnlyCompressedAccount>::from_bytes_at(bytes)?;
        #[cfg(feature = "pinocchio")]
        pinocchio::msg!("post read_only_accounts");
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
            || self.is_decompress() != other.is_decompress
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

#[test]
fn test_read_only_zero_copy() {
    let borsh_struct = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 0,
        invoking_program_id: Pubkey::default(),
        compress_or_decompress_lamports: 0,
        is_decompress: false,
        with_cpi_context: false,
        cpi_context: CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0,
        },
        proof: None,
        new_address_params: vec![NewAddressParamsPacked {
            seed: [1; 32],
            address_merkle_tree_account_index: 1,
            address_queue_account_index: 2,
            address_merkle_tree_root_index: 3,
        }],
        input_compressed_accounts: vec![InAccount {
            discriminator: [1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [10; 32],
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 1,
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                prove_by_index: false,
            },
            root_index: 3,
            lamports: 1000,
            address: Some([30; 32]),
        }],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::default().into(),
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
                nullifier_queue_pubkey_index: 6,
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
