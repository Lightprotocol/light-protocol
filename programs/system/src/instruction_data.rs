use anchor_lang::solana_program::log::sol_log_compute_units;
use light_utils::pubkey::Pubkey;
use light_verifier::CompressedProof;
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError, slice::ZeroCopySliceBorsh};
use std::{mem::size_of, ops::Deref};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
};

use crate::{
    sdk::{
        compressed_account::{CompressedAccount, CompressedAccountData},
        CompressedCpiContext,
    },
    OutputCompressedAccountWithPackedContext,
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

impl<'a> Deserialize<'a> for ZPackedReadOnlyAddress {
    type Output = Self;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (address, bytes) = bytes.split_at(size_of::<[u8; 32]>());
        let (address_merkle_tree_root_index, bytes) = U16::ref_from_prefix(bytes)?;
        let (address_merkle_tree_account_index, bytes) = u8::deserialize_at(bytes)?;

        Ok((
            ZPackedReadOnlyAddress {
                address: address.try_into().unwrap(),
                address_merkle_tree_root_index: (*address_merkle_tree_root_index).into(),
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

impl<'a> Deserialize<'a> for ZPackedMerkleContext {
    type Output = Ref<&'a [u8], Self>;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &[u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], Self>::from_prefix(bytes)?)
    }
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ZOutputCompressedAccountWithPackedContext<'a> {
    pub compressed_account: ZCompressedAccount<'a>,
    pub merkle_tree_index: u8,
}

impl<'a> From<ZOutputCompressedAccountWithPackedContext<'a>>
    for OutputCompressedAccountWithPackedContext
{
    fn from(output_compressed_account: ZOutputCompressedAccountWithPackedContext<'a>) -> Self {
        OutputCompressedAccountWithPackedContext {
            compressed_account: output_compressed_account.compressed_account.into(),
            merkle_tree_index: output_compressed_account.merkle_tree_index,
        }
    }
}

impl<'a> Deserialize<'a> for ZOutputCompressedAccountWithPackedContext<'a> {
    type Output = Self;

    #[inline]
    fn deserialize_at(vec: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (compressed_account, bytes) = ZCompressedAccount::deserialize_at(vec)?;
        let (merkle_tree_index, bytes) = u8::deserialize_at(bytes)?;
        Ok((
            ZOutputCompressedAccountWithPackedContext {
                compressed_account,
                merkle_tree_index,
            },
            bytes,
        ))
    }
}

#[derive(Debug, PartialEq)]
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

impl<'a> Deserialize<'a> for ZCompressedAccountData<'a> {
    type Output = Self;

    #[inline]
    fn deserialize_at(
        bytes: &'a [u8],
    ) -> Result<(ZCompressedAccountData<'a>, &'a [u8]), ZeroCopyError> {
        let (discriminator, bytes) = Ref::<&'a [u8], [u8; 8]>::from_prefix(bytes)?;
        let (len, bytes) = Ref::<&'a [u8], U32>::from_prefix(bytes)?;
        let (data, bytes) = bytes.split_at(u64::from(*len) as usize);
        // let (data, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
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

#[derive(Debug, PartialEq)]
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

impl<'a> From<ZCompressedAccount<'a>> for CompressedAccount {
    fn from(compressed_account: ZCompressedAccount) -> Self {
        let data: Option<CompressedAccountData> =
            if let Some(data) = compressed_account.data.as_ref() {
                Some(CompressedAccountData {
                    discriminator: *data.discriminator,
                    data: data.data.to_vec(),
                    data_hash: *data.data_hash,
                })
            } else {
                None
            };
        CompressedAccount {
            owner: compressed_account.owner.into(),
            lamports: compressed_account.lamports.into(),
            address: compressed_account.address.map(|x| *x),
            data,
        }
    }
}

use anchor_lang::solana_program::msg;
impl<'a> Deserialize<'a> for ZCompressedAccount<'a> {
    type Output = Self;

    #[inline]
    fn deserialize_at(
        bytes: &'a [u8],
    ) -> Result<(ZCompressedAccount<'a>, &'a [u8]), ZeroCopyError> {
        let (meta, bytes) = Ref::<&[u8], AccountDesMeta>::from_prefix(&bytes)?;
        let (address, bytes) = if meta.address_option == 1 {
            let (address, bytes) = Ref::<&[u8], [u8; 32]>::deserialize_at(bytes)?;
            (Some(address), bytes)
        } else {
            (None, bytes)
        };
        let (data, bytes) = Option::<ZCompressedAccountData>::deserialize_at(bytes)?;
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

#[derive(Debug, PartialEq)]
pub struct ZPackedCompressedAccountWithMerkleContext<'a> {
    pub compressed_account: ZCompressedAccount<'a>,
    meta: Ref<&'a [u8], ZPackedCompressedAccountWithMerkleContextMeta>, // pub merkle_context: Ref<&'a [u8], ZPackedMerkleContext>,
                                                                        // /// Index of root used in inclusion validity proof.
                                                                        // pub root_index: Ref<&'a [u8], U16>,
                                                                        // /// Placeholder to mark accounts read-only unimplemented set to false.
                                                                        // pub read_only: bool,
}

impl Deref for ZPackedCompressedAccountWithMerkleContext<'_> {
    type Target = ZPackedCompressedAccountWithMerkleContextMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> Deserialize<'a> for ZPackedCompressedAccountWithMerkleContext<'a> {
    type Output = Self;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (compressed_account, bytes) = ZCompressedAccount::deserialize_at(bytes)?;
        // let (merkle_context, bytes) = ZPackedMerkleContext::deserialize_at(bytes)?;
        // let (root_index, bytes) = Ref::<&[u8], U16>::from_prefix(bytes)?;
        // let (read_only, bytes) = u8::deserialize_at(bytes)?;
        let (meta, bytes) =
            Ref::<&[u8], ZPackedCompressedAccountWithMerkleContextMeta>::from_prefix(bytes)?;
        if meta.read_only == 1 {
            unimplemented!("Read only accounts not implemented");
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

impl<'a> Deserialize<'a> for ZInstructionDataInvoke<'a> {
    type Output = Self;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (proof, bytes) = Option::<CompressedProof>::deserialize_at(bytes)?;
        let (input_compressed_accounts_with_merkle_context, bytes) =
            Vec::<ZPackedCompressedAccountWithMerkleContext>::deserialize_at(bytes)?;
        let (output_compressed_accounts, bytes) =
            Vec::<ZOutputCompressedAccountWithPackedContext>::deserialize_at(bytes)?;
        let (relay_fee, bytes) = Option::<Ref<&'a [u8], U64>>::deserialize_at(bytes)?;
        if relay_fee.is_some() {
            unimplemented!("Relay fee not implemented");
        }

        let (new_address_params, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        let (compress_or_decompress_lamports, bytes) =
            Option::<Ref<&'a [u8], U64>>::deserialize_at(bytes)?;
        let (is_compress, bytes) = u8::deserialize_at(bytes)?;

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

impl<'a> Deserialize<'a> for ZInstructionDataInvokeCpi<'a> {
    type Output = Self;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        // msg!("proof");
        // sol_log_compute_units();
        let (proof, bytes) = Option::<CompressedProof>::deserialize_at(bytes)?;
        // sol_log_compute_units();
        // msg!("address");
        // sol_log_compute_units();
        let (new_address_params, bytes) = ZeroCopySliceBorsh::from_bytes_at(bytes)?;
        // msg!("inputs");
        // sol_log_compute_units();
        let (input_compressed_accounts_with_merkle_context, bytes) =
            Vec::<ZPackedCompressedAccountWithMerkleContext>::deserialize_at(bytes)?;
        // msg!("outputs");
        // sol_log_compute_units();
        let (output_compressed_accounts, bytes) =
            Vec::<ZOutputCompressedAccountWithPackedContext>::deserialize_at(bytes)?;
        // sol_log_compute_units();
        // msg!("relay fee");
        // sol_log_compute_units();
        let (option_relay_fee, bytes) = bytes.split_at(1);
        if option_relay_fee[0] == 1 {
            unimplemented!(" Relay fee is unimplemented");
        }
        // sol_log_compute_units();

        let (compress_or_decompress_lamports, bytes) =
            Option::<Ref<&'a [u8], U64>>::deserialize_at(bytes)?;
        let (is_compress, bytes) = u8::deserialize_at(bytes)?;
        let (cpi_context, bytes) =
            Option::<Ref<&[u8], ZCompressedCpiContext>>::deserialize_at(bytes)?;

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

impl Deserialize<'_> for CompressedCpiContext {
    type Output = Self;
    fn deserialize_at(bytes: &[u8]) -> Result<(Self, &[u8]), ZeroCopyError> {
        let (first_set_context, bytes) = u8::deserialize_at(bytes)?;
        let (set_context, bytes) = u8::deserialize_at(bytes)?;
        let (cpi_context_account_index, bytes) = u8::deserialize_at(bytes)?;

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

impl<'a> Deserialize<'a> for ZPackedReadOnlyCompressedAccount {
    type Output = Self;
    fn deserialize_at(_bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        unimplemented!("");

        // let (account_hash, bytes) = bytes.split_at(size_of::<[u8; 32]>());
        // // let (merkle_context, bytes) = ZPackedMerkleContext::deserialize_at(bytes)?;
        // let (root_index, bytes) = U16::ref_from_prefix(bytes)?;
        // Ok((
        //     ZPackedReadOnlyCompressedAccount {
        //         account_hash: account_hash.try_into().unwrap(),
        //         merkle_context: ZPackedMerkleContext::default(),
        //         root_index: (*root_index).into(),
        //     },
        //     bytes,
        // ))
    }
}

#[derive(Debug, PartialEq)]
pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    pub invoke_cpi: ZInstructionDataInvokeCpi<'a>,
    pub read_only_addresses: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>>,
    pub read_only_accounts: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>>,
}

impl<'a> Deserialize<'a> for ZInstructionDataInvokeCpiWithReadOnly<'a> {
    type Output = Self;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (invoke_cpi, bytes) = ZInstructionDataInvokeCpi::deserialize_at(bytes)?;
        let (read_only_addresses, bytes) =
            Option::<ZeroCopySliceBorsh<ZPackedReadOnlyAddress>>::deserialize_at(bytes)?;
        let (read_only_accounts, bytes) =
            Option::<ZeroCopySliceBorsh<ZPackedReadOnlyCompressedAccount>>::deserialize_at(bytes)?;
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

// TODO: add randomized tests
// TODO: add unit test ZInstructionDataInvokeCpiWithReadOnly
#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        errors::SystemProgramError,
        sdk::compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
            PackedMerkleContext,
        },
        InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
    };
    use crate::{
        invoke::processor::CompressedProof, InstructionDataInvoke, NewAddressParamsPacked,
    };
    use anchor_lang::AnchorSerialize;

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

    fn compare_invoke_cpi_instruction_data(
        reference: &InstructionDataInvokeCpi,
        z_copy: &ZInstructionDataInvokeCpi,
    ) -> Result<(), SystemProgramError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.proof.is_some() && z_copy.proof.is_some() {
            if reference.proof.as_ref().unwrap().a != z_copy.proof.as_ref().unwrap().a
                || reference.proof.as_ref().unwrap().b != z_copy.proof.as_ref().unwrap().b
                || reference.proof.as_ref().unwrap().c != z_copy.proof.as_ref().unwrap().c
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
        }
        if reference.compress_or_decompress_lamports
            != z_copy.compress_or_decompress_lamports.map(|x| (*x).into())
        {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.is_compress != z_copy.is_compress {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_none() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.cpi_context.is_none() && z_copy.cpi_context.is_some() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.cpi_context.is_some() && z_copy.cpi_context.is_some()
        // && reference.cpi_context.as_ref().unwrap()!= z_copy.cpi_context.as_ref().unwrap()
        {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        Ok(())
    }

    #[test]
    fn test_cpi_context_instruction_data() {
        let reference = get_instruction_data_invoke_cpi();

        let mut bytes = Vec::new();
        reference.serialize(&mut bytes).unwrap();
        let (z_copy, bytes) = ZInstructionDataInvokeCpi::deserialize_at(&bytes).unwrap();
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

    #[test]
    fn test_cpi_context_deserialize() {
        let cpi_context = get_cpi_context();
        let mut bytes = Vec::new();
        cpi_context.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = CompressedCpiContext::deserialize_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(z_copy, cpi_context);
    }

    #[test]
    fn test_account_deserialize() {
        let test_account = get_test_account();
        let mut bytes = Vec::new();
        test_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = ZCompressedAccount::deserialize_at(&bytes).unwrap();
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

    fn get_test_account() -> CompressedAccount {
        CompressedAccount {
            owner: Pubkey::new_unique().into(),
            lamports: 100,
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_test_account_data()),
        }
    }

    fn get_test_output_account() -> OutputCompressedAccountWithPackedContext {
        OutputCompressedAccountWithPackedContext {
            compressed_account: get_test_account(),
            merkle_tree_index: 1,
        }
    }

    #[test]
    fn test_output_account_deserialize() {
        let test_output_account = get_test_output_account();
        let mut bytes = Vec::new();
        test_output_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            ZOutputCompressedAccountWithPackedContext::deserialize_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_compressed_output_account(&test_output_account, &z_copy).unwrap();
    }

    fn compare_compressed_output_account(
        reference: &OutputCompressedAccountWithPackedContext,
        z_copy: &ZOutputCompressedAccountWithPackedContext,
    ) -> Result<(), SystemProgramError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        if reference.merkle_tree_index != z_copy.merkle_tree_index {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        Ok(())
    }

    fn get_test_input_account() -> PackedCompressedAccountWithMerkleContext {
        PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::new_unique().into(),
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
                nullifier_queue_pubkey_index: 2,
                leaf_index: 3,
                prove_by_index: true,
            },
            root_index: 5,
            read_only: false,
        }
    }
    #[test]
    fn test_input_account_deserialize() {
        let input_account = get_test_input_account();

        let mut bytes = Vec::new();
        input_account.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) =
            ZPackedCompressedAccountWithMerkleContext::deserialize_at(&bytes).unwrap();

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
    #[test]
    fn test_account_data_deserialize() {
        let test_data = CompressedAccountData {
            discriminator: 1u64.to_le_bytes(),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            data_hash: [1; 32],
        };

        let mut bytes = Vec::new();
        test_data.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = ZCompressedAccountData::deserialize_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(
            z_copy.discriminator.as_slice(),
            test_data.discriminator.as_slice()
        );
        assert_eq!(z_copy.data, test_data.data.as_slice());
        assert_eq!(z_copy.data_hash.as_slice(), test_data.data_hash.as_slice());
    }

    #[test]
    fn test_invoke_ix_data_deserialize() {
        let invoke_ref = InstructionDataInvoke {
            proof: Some(CompressedProof {
                a: [1; 32],
                b: [2; 64],
                c: [3; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![get_test_input_account(); 2],
            output_compressed_accounts: vec![get_test_output_account(); 2],
            relay_fee: None,
            new_address_params: vec![get_new_address_params(); 2],
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
        };
        let mut bytes = Vec::new();
        invoke_ref.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = ZInstructionDataInvoke::deserialize_at(&bytes).unwrap();
        assert!(bytes.is_empty());
        compare_instruction_data(&invoke_ref, &z_copy).unwrap();
    }

    fn compare_instruction_data(
        reference: &InstructionDataInvoke,
        z_copy: &ZInstructionDataInvoke,
    ) -> Result<(), SystemProgramError> {
        if reference.proof.is_some() && z_copy.proof.is_none() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.proof.is_none() && z_copy.proof.is_some() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.proof.is_some() && z_copy.proof.is_some() {
            if reference.proof.as_ref().unwrap().a != z_copy.proof.as_ref().unwrap().a
                || reference.proof.as_ref().unwrap().b != z_copy.proof.as_ref().unwrap().b
                || reference.proof.as_ref().unwrap().c != z_copy.proof.as_ref().unwrap().c
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
        }
        if reference
            .input_compressed_accounts_with_merkle_context
            .len()
            != z_copy.input_compressed_accounts_with_merkle_context.len()
        {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_input, z_input) in reference
            .input_compressed_accounts_with_merkle_context
            .iter()
            .zip(z_copy.input_compressed_accounts_with_merkle_context.iter())
        {
            compare_packed_compressed_account_with_merkle_context(ref_input, z_input)?;
        }
        if reference.output_compressed_accounts.len() != z_copy.output_compressed_accounts.len() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_output, z_output) in reference
            .output_compressed_accounts
            .iter()
            .zip(z_copy.output_compressed_accounts.iter())
        {
            compare_compressed_output_account(ref_output, z_output)?;
        }
        if reference.relay_fee != z_copy.relay_fee.map(|x| (*x).into()) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.new_address_params.len() != z_copy.new_address_params.len() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        for (ref_params, z_params) in reference
            .new_address_params
            .iter()
            .zip(z_copy.new_address_params.iter())
        {
            if ref_params.seed != z_params.seed {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_queue_account_index != z_params.address_queue_account_index {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_merkle_tree_account_index
                != z_params.address_merkle_tree_account_index
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
            if ref_params.address_merkle_tree_root_index
                != u16::from(z_params.address_merkle_tree_root_index)
            {
                return Err(SystemProgramError::InvalidArgument.into());
            }
        }
        Ok(())
    }

    fn compare_compressed_account_data(
        reference: &CompressedAccountData,
        z_copy: &ZCompressedAccountData,
    ) -> Result<(), SystemProgramError> {
        if reference.discriminator.as_slice() != z_copy.discriminator.as_slice() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.data != z_copy.data {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.data_hash.as_slice() != z_copy.data_hash.as_slice() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        Ok(())
    }

    fn compare_compressed_account(
        reference: &CompressedAccount,
        z_copy: &ZCompressedAccount,
    ) -> Result<(), SystemProgramError> {
        if reference.owner != z_copy.owner.into() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.lamports != u64::from(z_copy.lamports) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.address != z_copy.address.map(|x| *x) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.data.is_some() && z_copy.data.is_none() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.data.is_none() && z_copy.data.is_some() {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.data.is_some() && z_copy.data.is_some() {
            compare_compressed_account_data(
                &reference.data.as_ref().unwrap(),
                &z_copy.data.as_ref().unwrap(),
            )?;
        }
        Ok(())
    }

    fn compare_merkle_context(
        reference: PackedMerkleContext,
        z_copy: ZPackedMerkleContext,
    ) -> Result<(), SystemProgramError> {
        if reference.merkle_tree_pubkey_index != z_copy.merkle_tree_pubkey_index {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.nullifier_queue_pubkey_index != z_copy.nullifier_queue_pubkey_index {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.leaf_index != u32::from(z_copy.leaf_index) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        if reference.prove_by_index != (z_copy.prove_by_index == 1) {
            return Err(SystemProgramError::InvalidArgument.into());
        }
        Ok(())
    }

    fn compare_packed_compressed_account_with_merkle_context(
        reference: &PackedCompressedAccountWithMerkleContext,
        z_copy: &ZPackedCompressedAccountWithMerkleContext,
    ) -> Result<(), SystemProgramError> {
        compare_compressed_account(&reference.compressed_account, &z_copy.compressed_account)?;
        compare_merkle_context(reference.merkle_context, z_copy.merkle_context)?;
        if reference.root_index != u16::from(z_copy.root_index) {
            return Err(SystemProgramError::InvalidArgument.into());
        }

        Ok(())
    }
}
