use anchor_lang::prelude::Pubkey;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use borsh;
use rkyv::ser::serializers::{AlignedSerializer, CompositeSerializer, HeapScratch};
use rkyv::{AlignedVec, Archive, Deserialize, Fallible, Infallible, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, AnchorSerialize, AnchorDeserialize)]
pub struct WrappedPubkey(pub Pubkey);

impl WrappedPubkey {
    // Forward methods from Pubkey
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    // Forward other methods you need
    pub fn new_unique() -> Self {
        WrappedPubkey(Pubkey::new_unique())
    }
}

// Implementing conversion traits for interoperability
impl From<Pubkey> for WrappedPubkey {
    fn from(pubkey: Pubkey) -> Self {
        WrappedPubkey(pubkey)
    }
}

impl From<WrappedPubkey> for Pubkey {
    fn from(wrapped_pubkey: WrappedPubkey) -> Self {
        wrapped_pubkey.0
    }
}

impl Archive for WrappedPubkey {
    type Archived = ArchivedPubkey;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, _: usize, _: Self::Resolver, out: *mut Self::Archived) {
        (*out).0.copy_from_slice(&self.0.to_bytes());
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for WrappedPubkey {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible> Deserialize<ArchivedPubkey, D> for ArchivedPubkey {
    fn deserialize(&self, _: &mut D) -> Result<ArchivedPubkey, D::Error> {
        Ok(ArchivedPubkey(self.0))
    }
}
// TODO: remove once upgraded to anchor 0.30.0 (right now it's required for idl generation)
#[derive(
    Archive, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}
#[derive(
    Archive,
    Serialize,
    Deserialize,
    AnchorSerialize,
    AnchorDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct CompressedCpiContext {
    pub set_context: bool,
    pub cpi_context_account_index: u8,
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
// #[archive(
//     // This will generate a PartialEq impl between our unarchived and archived
//     // types:
//     compare(PartialEq),
//     // bytecheck can be used to validate your data if you want. To use the safe
//     // API, you have to derive CheckBytes for the archived type:
//     check_bytes,
// )]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_root_indices: Vec<u16>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    // pub output_compressed_accounts: Vec<CompressedAccount>,
    // /// The indices of the accounts in the output state merkle tree.
    // pub output_state_merkle_tree_account_indices: Vec<u8>,
    // pub relay_fee: Option<u64>,
    // pub compression_lamports: Option<u64>,
    // pub is_compress: bool,
    // pub signer_seeds: Vec<Vec<u8>>,
    // pub cpi_context: Option<CompressedCpiContext>,
}
#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}
#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct PackedCompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: PackedMerkleContext,
}

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    // PartialEq,
    Default,
    Clone,
    AnchorSerialize,
    AnchorDeserialize,
)]
// #[archive(
//     // This will generate a PartialEq impl between our unarchived and archived
//     // types:
//     compare(PartialEq),
//     // bytecheck can be used to validate your data if you want. To use the safe
//     // API, you have to derive CheckBytes for the archived type:
//     check_bytes,
// )]
pub struct CompressedAccountWithMerkleContext {
    pub compressed_account: CompressedAccount,
    pub merkle_context: MerkleContext,
}

// TODO: use in PackedCompressedAccountWithMerkleContext and rename to CompressedAccountAndMerkleContext
#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Default,
)]
// #[archive(
//     // This will generate a PartialEq impl between our unarchived and archived
//     // types:
//     compare(PartialEq),
//     // bytecheck can be used to validate your data if you want. To use the safe
//     // API, you have to derive CheckBytes for the archived type:
//     check_bytes,
// )]
pub struct MerkleContext {
    pub merkle_tree_pubkey: WrappedPubkey,
    pub nullifier_queue_pubkey: WrappedPubkey,
    pub leaf_index: u32,
}

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Default,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
}
#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct CompressedAccount {
    pub owner: WrappedPubkey,
    pub lamports: u64,
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
}

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Default,
    Clone,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
pub struct CompressedAccountData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}

#[repr(C)]
#[derive(
    Archive,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Debug,
    // Eq,
    AnchorDeserialize,
    AnchorSerialize,
    Default,
)]
// #[archive(
//     // This will generate a PartialEq impl between our unarchived and archived
//     // types:
//     compare(PartialEq),
//     // bytecheck can be used to validate your data if you want. To use the safe
//     // API, you have to derive CheckBytes for the archived type:
//     check_bytes,
// )]
pub struct ArchivedPubkey([u8; 32]);

// impl std::cmp::PartialEq for ArchivedPubkey {
//     fn eq(&self, other: &Self) -> bool {
//         self.0 == other.0
//     }
// }
impl PartialEq<WrappedPubkey> for ArchivedPubkey {
    fn eq(&self, other: &WrappedPubkey) -> bool {
        &self.0 == &other.0.to_bytes()
    }
}

use rkyv::ser::Serializer;
#[test]
fn test() {
    // Example usage
    let pubkey = WrappedPubkey::new_unique();
    let mut serializer = rkyv::ser::serializers::AlignedSerializer::new(rkyv::AlignedVec::new());
    serializer
        .serialize_value(&pubkey)
        .expect("Serialization failed");
    let bytes = serializer.into_inner();

    let archived = unsafe { rkyv::archived_root::<ArchivedPubkey>(&bytes) };
    let deserialized: ArchivedPubkey = archived
        .deserialize(&mut rkyv::Infallible)
        .expect("Deserialization failed");

    assert_eq!(pubkey.0.to_bytes(), deserialized.0);
}

// use rkyv::ser::{serializers::AlignedSerializer, CompositeSerializer, HeapScratch, Serializer};
// use rkyv::{AlignedVec, Archive, Deserialize, Infallible, Serialize};

#[test]
fn test_serialize_deserialize_instruction_data_invoke_cpi() {
    let value = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![NewAddressParamsPacked {
            seed: [1; 32],
            address_queue_account_index: 2,
            address_merkle_tree_account_index: 3,
            address_merkle_tree_root_index: 4,
        }],
        input_root_indices: vec![1, 2, 3],
        input_compressed_accounts_with_merkle_context: vec![],
        // output_compressed_accounts: vec![],
        // output_state_merkle_tree_account_indices: vec![4, 5, 6],
        // relay_fee: Some(1000),
        // compression_lamports: Some(5000),
        // is_compress: true,
        // signer_seeds: vec![vec![7, 8, 9]],
        // cpi_context: None,
    };
    // Serializing is as easy as a single function call
    let size = std::mem::size_of::<MyDataVec>();
    let bytes = rkyv::to_bytes::<_, 512>(&value).unwrap();
    println!("bytes {:?}", bytes);
    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::ser::{serializers::AllocSerializer, Serializer};

    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&value).unwrap();
    let bytes = serializer.into_serializer().into_inner();

    // You can use the safe API for fast zero-copy deserialization
    // let archived = rkyv::check_archived_root::<InstructionDataInvokeCpi>(&bytes[..]).unwrap();
    // assert_eq!(archived, &value);

    // Or you can use the unsafe API for maximum performance
    let archived = unsafe { rkyv::archived_root::<InstructionDataInvokeCpi>(&bytes[..]) };
    assert_eq!(archived, &value);

    // And you can always deserialize back to the original type
    let deserialized: InstructionDataInvokeCpi =
        archived.deserialize(&mut rkyv::Infallible).unwrap();
    assert_eq!(deserialized, value);
    println!("deserialized {:?}", deserialized);
    // // Use CompositeSerializer with HeapScratch, AlignedSerializer, and Infallible
    // let mut serializer = rkyv::ser::serializers::AlignedSerializer::new(rkyv::AlignedVec::new());

    // // Serialize the struct
    // serializer
    //     .serialize_value(&original_data)
    //     .expect("Serialization failed");
    // let bytes = serializer.into_components().;

    // // Deserialize the struct
    // let archived = unsafe { rkyv::archived_root::<InstructionDataInvokeCpi>(&bytes) };
    // let deserialized: InstructionDataInvokeCpi = archived
    //     .deserialize(&mut Infallible)
    //     .expect("Deserialization failed")
    //     .into_inner();

    // Assert that the deserialized data matches the original
    // assert_eq!(original_data, deserialized);
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
// Derives can be passed through to the generated type:
#[archive_attr(derive(Debug))]
pub struct MyData {
    value: i32,
    label: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
// Derives can be passed through to the generated type:
#[archive_attr(derive(Debug))]
pub struct MyDataVec {
    vec: Vec<MyData>,
}

use rkyv::result::ArchivedResult;
#[test]
fn test_rkyv() {
    // Create an instance of `MyDataVec`
    let value = MyDataVec {
        vec: vec![
            MyData {
                value: 42,
                label: "first".to_string(),
            },
            MyData {
                value: 100,
                label: "second".to_string(),
            },
        ],
    };

    // Serializing is as easy as a single function call
    let size = std::mem::size_of::<MyDataVec>();
    let bytes = rkyv::to_bytes::<_, 512>(&value).unwrap();
    println!("bytes {:?}", bytes);
    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::ser::{serializers::AllocSerializer, Serializer};

    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&value).unwrap();
    let bytes = serializer.into_serializer().into_inner();

    // You can use the safe API for fast zero-copy deserialization
    let archived = rkyv::check_archived_root::<MyDataVec>(&bytes[..]).unwrap();
    assert_eq!(archived, &value);

    // Or you can use the unsafe API for maximum performance
    let archived = unsafe { rkyv::archived_root::<MyDataVec>(&bytes[..]) };
    assert_eq!(archived, &value);

    // And you can always deserialize back to the original type
    let deserialized: MyDataVec = archived.deserialize(&mut rkyv::Infallible).unwrap();
    assert_eq!(deserialized, value);
}
