use std::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, errors::ZeroCopyError};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
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

    fn byte_len(&self) -> usize {
        32
    }
}

// We should not implement DeserializeMut for primitive types directly
// The implementation should be in the zero-copy crate

impl PartialEq<<Pubkey as Deserialize<'_>>::Output> for Pubkey {
    fn eq(&self, other: &<Pubkey as Deserialize<'_>>::Output) -> bool {
        self.0 == other.0
    }
}

#[derive(ZeroCopy, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone)]
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

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone, Copy,
)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone, Copy,
)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct ReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[derive(ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Clone, Copy)]
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
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
    /// Index of root used in inclusion validity proof.
    pub root_index: u16,
    /// Placeholder to mark accounts read-only unimplemented set to false.
    pub read_only: bool,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Default,
)]
pub struct MerkleContext {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct ReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: MerkleContext,
    pub root_index: u16,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct PackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[derive(
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Default,
)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

#[derive(ZeroCopy, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone)]
pub struct CompressedAccount {
    pub owner: [u8; 32],
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

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
    ZeroCopy, ZeroCopyEq, BorshDeserialize, BorshSerialize, Debug, PartialEq, Default, Clone,
)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[test]
fn readme() {
    use borsh::{BorshDeserialize, BorshSerialize};
    use light_zero_copy_derive::ZeroCopy;

    #[repr(C)]
    #[derive(Debug, PartialEq, BorshSerialize, BorshDeserialize, ZeroCopy, ZeroCopyEq)]
    pub struct MyStruct {
        pub a: u8,
        pub b: u16,
        pub vec: Vec<u8>,
        pub c: u64,
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
