#![cfg(feature = "mut")]
use std::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, errors::ZeroCopyError};
use light_zero_copy_derive::{ByteLen, ZeroCopy, ZeroCopyConfig, ZeroCopyEq, ZeroCopyMut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

#[derive(
    Debug,
    Copy,
    PartialEq,
    Clone,
    Immutable,
    FromBytes,
    IntoBytes,
    KnownLayout,
    BorshDeserialize,
    BorshSerialize,
    Default,
    Unaligned,
)]
#[repr(C)]
pub struct Pubkey(pub(crate) [u8; 32]);

impl<'a> Deserialize<'a> for Pubkey {
    type Output = Ref<&'a [u8], Pubkey>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&'a [u8], Pubkey>::from_prefix(bytes)?)
    }
}

impl<'a> DeserializeMut<'a> for Pubkey {
    type Output = Ref<&'a mut [u8], Pubkey>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(Ref::<&'a mut [u8], Pubkey>::from_prefix(bytes)?)
    }
}

// We should not implement DeserializeMut for primitive types directly
// The implementation should be in the zero-copy crate

impl PartialEq<<Pubkey as Deserialize<'_>>::Output> for Pubkey {
    fn eq(&self, other: &<Pubkey as Deserialize<'_>>::Output) -> bool {
        self.0 == other.0
    }
}

impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for Pubkey {
    type Config = ();
    type Output = <Self as DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Self::zero_copy_at_mut(bytes)
    }
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for InstructionDataInvoke {
    type Config = InstructionDataInvokeConfig;
    type Output = <Self as light_zero_copy::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        use zerocopy::Ref;
        
        // First handle the meta struct (empty for InstructionDataInvoke)
        let (__meta, bytes) = Ref::<&mut [u8], ZInstructionDataInvokeMetaMut>::from_prefix(bytes)?;
        
        // Initialize each field using the corresponding config, following DeserializeMut order
        let (proof, bytes) = <Option<CompressedProof> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes, 
            (config.proof_config.is_some(), CompressedProofConfig {})
        )?;
        
        let input_configs: Vec<PackedCompressedAccountWithMerkleContextConfig> = config.input_accounts_configs
            .into_iter()
            .map(|compressed_account_config| PackedCompressedAccountWithMerkleContextConfig {
                compressed_account: compressed_account_config,
                merkle_context: PackedMerkleContextConfig {},
            })
            .collect();
        let (input_compressed_accounts_with_merkle_context, bytes) = <Vec<PackedCompressedAccountWithMerkleContext> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            input_configs
        )?;
        
        let output_configs: Vec<OutputCompressedAccountWithPackedContextConfig> = config.output_accounts_configs
            .into_iter()
            .map(|compressed_account_config| OutputCompressedAccountWithPackedContextConfig {
                compressed_account: compressed_account_config,
            })
            .collect();
        let (output_compressed_accounts, bytes) = <Vec<OutputCompressedAccountWithPackedContext> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            output_configs
        )?;
        
        let (relay_fee, bytes) = <Option<u64> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            (config.relay_fee_config.is_some(), ())
        )?;
        
        let new_address_configs: Vec<NewAddressParamsPackedConfig> = config.new_address_configs
            .into_iter()
            .map(|_| NewAddressParamsPackedConfig {})
            .collect();
        let (new_address_params, bytes) = <Vec<NewAddressParamsPacked> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            new_address_configs
        )?;
        
        let (compress_or_decompress_lamports, bytes) = <Option<u64> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            (config.decompress_lamports_config.is_some(), ())
        )?;
        
        let (is_compress, bytes) = <bool as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
            bytes,
            ()
        )?;
        
        Ok((
            ZInstructionDataInvokeMut {
                proof,
                input_compressed_accounts_with_merkle_context,
                output_compressed_accounts,
                relay_fee,
                new_address_params,
                compress_or_decompress_lamports,
                is_compress,
            },
            bytes,
        ))
    }
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    ZeroCopyConfig,
)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for OutputCompressedAccountWithPackedContext {
//     type Config = CompressedAccountZeroCopyConfig;
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZOutputCompressedAccountWithPackedContextMetaMut>::from_prefix(bytes)?;
//         let (compressed_account, bytes) = <CompressedAccount as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, config)?;
//         let (merkle_tree_index, bytes) = <u8 as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, ())?;
//         
//         Ok((
//             ZOutputCompressedAccountWithPackedContextMut {
//                 compressed_account,
//                 merkle_tree_index,
//             },
//             bytes,
//         ))
//     }
// }

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    Copy,
    ZeroCopyConfig,
)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for NewAddressParamsPacked {
//     type Config = ();
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         _config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZNewAddressParamsPackedMetaMut>::from_prefix(bytes)?;
//         Ok((ZNewAddressParamsPackedMut { __meta }, bytes))
//     }
// }

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    Copy,
)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct ReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Clone,
    Copy,
    ZeroCopyConfig,
)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for CompressedProof {
//     type Config = ();
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         _config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZCompressedProofMetaMut>::from_prefix(bytes)?;
//         Ok((ZCompressedProofMut { __meta }, bytes))
//     }
// }

#[derive(
    ZeroCopy,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to clear the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    ZeroCopyConfig,
)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    pub read_only: bool,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for PackedCompressedAccountWithMerkleContext {
//     type Config = CompressedAccountZeroCopyConfig;
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZPackedCompressedAccountWithMerkleContextMetaMut>::from_prefix(bytes)?;
//         let (compressed_account, bytes) = <CompressedAccount as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, config)?;
//         let (merkle_context, bytes) = <PackedMerkleContext as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, ())?;
//         let (root_index, bytes) = <zerocopy::little_endian::U16 as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, ())?;
//         let (read_only, bytes) = <bool as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(bytes, ())?;
//         
//         Ok((
//             ZPackedCompressedAccountWithMerkleContextMut {
//                 compressed_account,
//                 merkle_context, 
//                 root_index,
//                 read_only,
//             },
//             bytes,
//         ))
//     }
// }

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
    ZeroCopyConfig,
)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for MerkleContext {
//     type Config = ();
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         _config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZMerkleContextMetaMut>::from_prefix(bytes)?;
//         
//         Ok((
//             ZMerkleContextMut {
//                 __meta,
//             },
//             bytes,
//         ))
//     }
// }

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct ReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: MerkleContext,
    pub root_index: u16,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct PackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
    ZeroCopyConfig,
)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for PackedMerkleContext {
//     type Config = ();
//     type Output = <Self as DeserializeMut<'a>>::Output;
//     
//     fn new_zero_copy(
//         bytes: &'a mut [u8], 
//         _config: Self::Config
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZPackedMerkleContextMetaMut>::from_prefix(bytes)?;
//         Ok((ZPackedMerkleContextMut { __meta }, bytes))
//     }
// }

#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub struct CompressedAccountZeroCopyConfig {
    pub address_enabled: bool,
    pub data_enabled: bool,
    pub data_capacity: u32,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct InstructionDataInvokeConfig {
    pub proof_config: Option<()>, // None = no proof, Some(()) = include proof
    pub input_accounts_configs: Vec<CompressedAccountZeroCopyConfig>,
    pub output_accounts_configs: Vec<CompressedAccountZeroCopyConfig>, 
    pub relay_fee_config: Option<()>,
    pub new_address_configs: Vec<()>, // NewAddressParamsPacked needs no config
    pub decompress_lamports_config: Option<()>,
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ByteLen,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    ZeroCopyConfig,
)]
pub struct CompressedAccount {
    pub owner: [u8; 32],
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for CompressedAccount {
//     type Config = CompressedAccountZeroCopyConfig;
//     type Output = <Self as DeserializeMut<'a>>::Output;
// 
//     fn new_zero_copy(
//         bytes: &'a mut [u8],
//         config: Self::Config,
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZCompressedAccountMetaMut>::from_prefix(bytes)?;
// 
//         // Use generic Option implementation for address field
//         let (address, bytes) = <Option<[u8; 32]> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
//             bytes,
//             (config.address_enabled, ())
//         )?;
// 
//         // Use generic Option implementation for data field
//         let (data, bytes) = <Option<CompressedAccountData> as light_zero_copy::init_mut::ZeroCopyInitMut>::new_zero_copy(
//             bytes,
//             (config.data_enabled, CompressedAccountDataConfig { data: config.data_capacity })
//         )?;
// 
//         Ok((
//             ZCompressedAccountMut {
//                 __meta,
//                 address,
//                 data,
//             },
//             bytes,
//         ))
//     }
// }

impl<'a> From<ZCompressedAccount<'a>> for CompressedAccount {
    fn from(value: ZCompressedAccount<'a>) -> Self {
        Self {
            owner: value.__meta.owner,
            lamports: u64::from(value.__meta.lamports),
            address: value.address.map(|x| *x),
            data: value.data.as_ref().map(|x| x.into()),
        }
    }
}

impl<'a> From<&ZCompressedAccount<'a>> for CompressedAccount {
    fn from(value: &ZCompressedAccount<'a>) -> Self {
        Self {
            owner: value.__meta.owner,
            lamports: u64::from(value.__meta.lamports),
            address: value.address.as_ref().map(|x| **x),
            data: value.data.as_ref().map(|x| x.into()),
        }
    }
}

impl PartialEq<CompressedAccount> for ZCompressedAccount<'_> {
    fn eq(&self, other: &CompressedAccount) -> bool {
        if self.address.is_some()
            && other.address.is_some()
            && *self.address.unwrap() != other.address.unwrap()
        {
            return false;
        }
        if self.address.is_some() || other.address.is_some() {
            return false;
        }
        if self.data.is_some()
            && other.data.is_some()
            && self.data.as_ref().unwrap() != other.data.as_ref().unwrap()
        {
            return false;
        }
        if self.data.is_some() || other.data.is_some() {
            return false;
        }

        self.owner == other.owner && self.lamports == other.lamports
    }
}

// Commented out because mutable derivation is disabled
// impl PartialEq<CompressedAccount> for ZCompressedAccountMut<'_> {
//     fn eq(&self, other: &CompressedAccount) -> bool {
//         if self.address.is_some()
//             && other.address.is_some()
//             && **self.address.as_ref().unwrap() != *other.address.as_ref().unwrap()
//         {
//             return false;
//         }
//         if self.address.is_some() || other.address.is_some() {
//             return false;
//         }
//         if self.data.is_some()
//             && other.data.is_some()
//             && self.data.as_ref().unwrap() != other.data.as_ref().unwrap()
//         {
//             return false;
//         }
//         if self.data.is_some() || other.data.is_some() {
//             return false;
//         }

//         self.owner == other.owner && self.lamports == other.lamports
//     }
// }
impl PartialEq<ZCompressedAccount<'_>> for CompressedAccount {
    fn eq(&self, other: &ZCompressedAccount) -> bool {
        if self.address.is_some()
            && other.address.is_some()
            && self.address.unwrap() != *other.address.unwrap()
        {
            return false;
        }
        if self.address.is_some() || other.address.is_some() {
            return false;
        }
        if self.data.is_some()
            && other.data.is_some()
            && other.data.as_ref().unwrap() != self.data.as_ref().unwrap()
        {
            return false;
        }
        if self.data.is_some() || other.data.is_some() {
            return false;
        }

        self.owner == other.owner && self.lamports == other.lamports.into()
    }
}

#[derive(
    ZeroCopy,
    ZeroCopyMut,
    ZeroCopyEq,
    ByteLen,
    ZeroCopyConfig,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Default,
    Clone,
)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

// COMMENTED OUT: Now using ZeroCopyConfig derive macro instead
// impl<'a> light_zero_copy::init_mut::ZeroCopyInitMut<'a> for CompressedAccountData {
//     type Config = u32; // data_capacity
//     type Output = <Self as DeserializeMut<'a>>::Output;

//     fn new_zero_copy(
//         bytes: &'a mut [u8],
//         data_capacity: Self::Config,
//     ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
//         let (__meta, bytes) = Ref::<&mut [u8], ZCompressedAccountDataMetaMut>::from_prefix(bytes)?;
//         // For u8 slices we just use &mut [u8] so we init the len and the split mut separately.
//         {
//             light_zero_copy::slice_mut::ZeroCopySliceMutBorsh::<u8>::new_at(
//                 data_capacity.into(),
//                 bytes,
//             )?;
//         }
//         // Split off len for
//         let (_, bytes) = bytes.split_at_mut(4);
//         let (data, bytes) = bytes.split_at_mut(data_capacity as usize);
//         let (data_hash, bytes) = Ref::<&mut [u8], [u8; 32]>::from_prefix(bytes)?;
//         Ok((
//             ZCompressedAccountDataMut {
//                 __meta,
//                 data,
//                 data_hash,
//             },
//             bytes,
//         ))
//     }
// }

#[test]
fn test_compressed_account_data_new_at() {
    use light_zero_copy::init_mut::ZeroCopyInitMut;
    let mut bytes = vec![0u8; 100];
    let config = CompressedAccountDataConfig { data: 10 };
    let result = CompressedAccountData::new_zero_copy(&mut bytes, config);
    assert!(result.is_ok());
    let (mut mut_account, _remaining) = result.unwrap();

    // Test that we can set discriminator
    mut_account.__meta.discriminator = [1, 2, 3, 4, 5, 6, 7, 8];

    // Test that we can write to data
    mut_account.data[0] = 42;
    mut_account.data[1] = 43;

    // Test that we can set data_hash
    mut_account.data_hash[0] = 99;
    mut_account.data_hash[1] = 100;

    assert_eq!(mut_account.__meta.discriminator, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(mut_account.data[0], 42);
    assert_eq!(mut_account.data[1], 43);
    assert_eq!(mut_account.data_hash[0], 99);
    assert_eq!(mut_account.data_hash[1], 100);

    // Test deserializing the initialized bytes with zero_copy_at_mut
    let deserialize_result = CompressedAccountData::zero_copy_at_mut(&mut bytes);
    assert!(deserialize_result.is_ok());
    let (deserialized_account, _remaining) = deserialize_result.unwrap();

    // Verify the deserialized data matches what we set
    assert_eq!(
        deserialized_account.__meta.discriminator,
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_eq!(deserialized_account.data.len(), 10);
    assert_eq!(deserialized_account.data[0], 42);
    assert_eq!(deserialized_account.data[1], 43);
    assert_eq!(deserialized_account.data_hash[0], 99);
    assert_eq!(deserialized_account.data_hash[1], 100);
}

#[test]
fn test_compressed_account_new_at() {
    use light_zero_copy::init_mut::ZeroCopyInitMut;
    let mut bytes = vec![0u8; 200];
    let config = CompressedAccountZeroCopyConfig {
        address_enabled: true,
        data_enabled: true,
        data_capacity: 10,
    };
    let result = CompressedAccount::new_zero_copy(&mut bytes, config);
    assert!(result.is_ok());
    let (mut mut_account, _remaining) = result.unwrap();

    // Set values
    mut_account.__meta.owner = [1u8; 32];
    mut_account.__meta.lamports = 12345u64.into();
    mut_account.address.as_mut().unwrap()[0] = 42;
    mut_account.data.as_mut().unwrap().data[0] = 99;

    // Test deserialize
    let (deserialized, _) = CompressedAccount::zero_copy_at_mut(&mut bytes).unwrap();
    assert_eq!(deserialized.__meta.owner, [1u8; 32]);
    assert_eq!(u64::from(deserialized.__meta.lamports), 12345u64);
    assert_eq!(deserialized.address.as_ref().unwrap()[0], 42);
    assert_eq!(deserialized.data.as_ref().unwrap().data[0], 99);
}

#[test]
fn test_instruction_data_invoke_new_at() {
    use light_zero_copy::init_mut::ZeroCopyInitMut;
    let mut bytes = vec![0u8; 5000]; // Large buffer for complex structure with alignment
    
    // Create different configs to test various combinations
    let compressed_account_config1 = CompressedAccountZeroCopyConfig {
        address_enabled: true,
        data_enabled: true,
        data_capacity: 10,
    };
    
    let compressed_account_config2 = CompressedAccountZeroCopyConfig {
        address_enabled: false,
        data_enabled: true,
        data_capacity: 5,
    };
    
    let compressed_account_config3 = CompressedAccountZeroCopyConfig {
        address_enabled: true,
        data_enabled: false,
        data_capacity: 0,
    };
    
    let compressed_account_config4 = CompressedAccountZeroCopyConfig {
        address_enabled: false,
        data_enabled: false,
        data_capacity: 0,
    };
    
    let config = InstructionDataInvokeConfig {
        proof_config: Some(()), // Enable proof 
        input_accounts_configs: vec![compressed_account_config1, compressed_account_config2], // Length 2, different configs
        output_accounts_configs: vec![compressed_account_config3, compressed_account_config4], // Length 2, different configs
        relay_fee_config: Some(()), // Enable relay fee
        new_address_configs: vec![(), ()], // Length 2
        decompress_lamports_config: Some(()), // Enable decompress lamports
    };
    
    let result = InstructionDataInvoke::new_zero_copy(&mut bytes, config);
    if let Err(ref e) = result {
        eprintln!("Error: {:?}", e);
    }
    assert!(result.is_ok());
    let (mut instruction_data, _remaining) = result.unwrap();

    // Test deserialization round-trip first 
    let (mut deserialized, _) = InstructionDataInvoke::zero_copy_at_mut(&mut bytes).unwrap();
    
    // Now set values and test again
    deserialized.is_compress = 1;
    
    // Set proof values
    if let Some(proof) = &mut deserialized.proof {
        proof.a[0] = 42;
        proof.b[0] = 43;
        proof.c[0] = 44;
    }
    
    // Set relay fee value
    if let Some(relay_fee) = &mut deserialized.relay_fee {
        **relay_fee = 12345u64.into();
    }
    
    // Set decompress lamports value
    if let Some(decompress_lamports) = &mut deserialized.compress_or_decompress_lamports {
        **decompress_lamports = 67890u64.into();
    }
    
    // Set first input account values
    let first_input = &mut deserialized.input_compressed_accounts_with_merkle_context[0];
    first_input.compressed_account.__meta.owner[0] = 11;
    first_input.compressed_account.__meta.lamports = 1000u64.into();
    if let Some(address) = &mut first_input.compressed_account.address {
        address[0] = 22;
    }
    if let Some(data) = &mut first_input.compressed_account.data {
        data.__meta.discriminator[0] = 33;
        data.data[0] = 99;
        data.data_hash[0] = 55;
    }
    
    // Set first output account values
    let first_output = &mut deserialized.output_compressed_accounts[0];
    first_output.compressed_account.__meta.owner[0] = 77;
    first_output.compressed_account.__meta.lamports = 2000u64.into();
    if let Some(address) = &mut first_output.compressed_account.address {
        address[0] = 88;
    }

    // Verify basic structure with vectors of length 2
    assert_eq!(deserialized.input_compressed_accounts_with_merkle_context.len(), 2); // Length 2
    assert_eq!(deserialized.output_compressed_accounts.len(), 2); // Length 2
    assert_eq!(deserialized.new_address_params.len(), 2); // Length 2
    assert!(deserialized.proof.is_some()); // Enabled
    assert!(deserialized.relay_fee.is_some()); // Enabled
    assert!(deserialized.compress_or_decompress_lamports.is_some()); // Enabled
    assert_eq!(deserialized.is_compress, 1);
    
    // Test data access and modification
    if let Some(proof) = &deserialized.proof {
        // Verify we can access proof fields and our written values
        assert_eq!(proof.a[0], 42);
        assert_eq!(proof.b[0], 43);
        assert_eq!(proof.c[0], 44);
    }
    
    // Verify option integer values
    if let Some(relay_fee) = &deserialized.relay_fee {
        assert_eq!(u64::from(**relay_fee), 12345);
    }
    
    if let Some(decompress_lamports) = &deserialized.compress_or_decompress_lamports {
        assert_eq!(u64::from(**decompress_lamports), 67890);
    }
    
    // Test accessing first input account (config1: address=true, data=true, capacity=10)
    let first_input = &deserialized.input_compressed_accounts_with_merkle_context[0];
    assert_eq!(first_input.compressed_account.__meta.owner[0], 11); // Our written value
    assert_eq!(u64::from(first_input.compressed_account.__meta.lamports), 1000); // Our written value
    assert!(first_input.compressed_account.address.is_some()); // Should be enabled
    assert!(first_input.compressed_account.data.is_some()); // Should be enabled
    if let Some(address) = &first_input.compressed_account.address {
        assert_eq!(address[0], 22); // Our written value
    }
    if let Some(data) = &first_input.compressed_account.data {
        assert_eq!(data.data.len(), 10); // Should have capacity 10
        assert_eq!(data.__meta.discriminator[0], 33); // Our written value
        assert_eq!(data.data[0], 99); // Our written value
        assert_eq!(data.data_hash[0], 55); // Our written value
    }
    
    // Test accessing second input account (config2: address=false, data=true, capacity=5)
    let second_input = &deserialized.input_compressed_accounts_with_merkle_context[1];
    assert_eq!(second_input.compressed_account.__meta.owner[0], 0); // Should be zero (not written)
    assert!(second_input.compressed_account.address.is_none()); // Should be disabled
    assert!(second_input.compressed_account.data.is_some()); // Should be enabled
    if let Some(data) = &second_input.compressed_account.data {
        assert_eq!(data.data.len(), 5); // Should have capacity 5
    }
    
    // Test accessing first output account (config3: address=true, data=false, capacity=0)
    let first_output = &deserialized.output_compressed_accounts[0];
    assert_eq!(first_output.compressed_account.__meta.owner[0], 77); // Our written value
    assert_eq!(u64::from(first_output.compressed_account.__meta.lamports), 2000); // Our written value
    assert!(first_output.compressed_account.address.is_some()); // Should be enabled
    assert!(first_output.compressed_account.data.is_none()); // Should be disabled
    if let Some(address) = &first_output.compressed_account.address {
        assert_eq!(address[0], 88); // Our written value
    }
    
    // Test accessing second output account (config4: address=false, data=false, capacity=0)
    let second_output = &deserialized.output_compressed_accounts[1];
    assert_eq!(second_output.compressed_account.__meta.owner[0], 0); // Should be zero (not written)
    assert!(second_output.compressed_account.address.is_none()); // Should be disabled
    assert!(second_output.compressed_account.data.is_none()); // Should be disabled
}

#[test]
fn readme() {
    use borsh::{BorshDeserialize, BorshSerialize};
    use light_zero_copy_derive::{ByteLen, ZeroCopy, ZeroCopyEq, ZeroCopyMut};

    #[repr(C)]
    #[derive(
        Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ByteLen,
    )]
    pub struct MyStructOption {
        pub a: u8,
        pub b: u16,
        pub vec: Vec<Option<u64>>,
        pub c: Option<u64>,
    }

    #[repr(C)]
    #[derive(
        Debug,
        PartialEq,
        BorshSerialize,
        BorshDeserialize,
        ZeroCopy,
        ZeroCopyMut,
        ZeroCopyEq,
        ByteLen,
    )]
    pub struct MyStruct {
        pub a: u8,
        pub b: u16,
        pub vec: Vec<u8>,
        pub c: u64,
    }

    // Test the new ZeroCopyConfig functionality
    use light_zero_copy_derive::ZeroCopyConfig;

    #[repr(C)]
    #[derive(
        Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyMut, ByteLen,
    )]
    pub struct TestConfigStruct {
        pub a: u8,
        pub b: u16,
        pub vec: Vec<u8>,
        pub option: Option<u64>,
    }

    let my_struct = MyStruct {
        a: 1,
        b: 2,
        vec: vec![1u8; 32],
        c: 3,
    };
    // Use the struct with zero-copy deserialization
    let bytes = my_struct.try_to_vec().unwrap();
    // byte_len not available for non-mut derivations
    // assert_eq!(bytes.len(), my_struct.byte_len());
    let (zero_copy, _remaining) = MyStruct::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy.a, 1);
    let org_struct: MyStruct = zero_copy.into();
    assert_eq!(org_struct, my_struct);
    // {
    //     let (mut zero_copy_mut, _remaining) = MyStruct::zero_copy_at_mut(&mut bytes).unwrap();
    //     zero_copy_mut.a = 42;
    // }
    // let borsh = MyStruct::try_from_slice(&bytes).unwrap();
    // assert_eq!(borsh.a, 42u8);
}
