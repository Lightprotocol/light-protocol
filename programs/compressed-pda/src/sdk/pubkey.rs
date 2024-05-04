use anchor_lang::prelude::Pubkey;
use rkyv::{Archive, Deserialize, Fallible, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl<D: Fallible> Deserialize<WrappedPubkey, D> for ArchivedPubkey {
    fn deserialize(&self, _: &mut D) -> Result<WrappedPubkey, D::Error> {
        Ok(WrappedPubkey(Pubkey::new_from_array(self.0)))
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArchivedPubkey([u8; 32]);

#[cfg(test)]
mod tests {
    use std::vec;

    use rkyv::{ser::serializers::AlignedSerializer, ser::Serializer, AlignedVec, Infallible};

    #[derive(Debug, PartialEq, Default, Clone)]
    pub struct InstructionDataInvokeCpi {
        pub proof: Option<CompressedProof>,
        pub new_address_params: Vec<NewAddressParamsPacked>,
        pub input_root_indices: Vec<u16>,
        pub input_compressed_accounts_with_merkle_context:
            Vec<PackedCompressedAccountWithMerkleContext>,
        pub output_compressed_accounts: Vec<CompressedAccount>,
        /// The indices of the accounts in the output state merkle tree.
        pub output_state_merkle_tree_account_indices: Vec<u8>,
        pub relay_fee: Option<u64>,
        pub compression_lamports: Option<u64>,
        pub is_compress: bool,
        pub signer_seeds: Vec<Vec<u8>>,
        pub cpi_context: Option<CompressedCpiContext>,
    }

    #[test]
    fn test_serialize_deserialize_instruction_data_invoke_cpi() {
        // Create an instance of the struct with sample data
        let original_data = InstructionDataInvokeCpi {
            proof: None,
            new_address_params: vec![],
            input_root_indices: vec![1, 2, 3],
            input_compressed_accounts_with_merkle_context: vec![],
            output_compressed_accounts: vec![],
            output_state_merkle_tree_account_indices: vec![4, 5, 6],
            relay_fee: Some(1000),
            compression_lamports: Some(5000),
            is_compress: true,
            signer_seeds: vec![vec![7, 8, 9]],
            cpi_context: None,
        };

        // Serialize the struct
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(&original_data)
            .expect("Serialization failed");
        let bytes = serializer.into_inner();

        // Deserialize the struct
        let archived = unsafe { rkyv::archived_root::<InstructionDataInvokeCpi>(&bytes) };
        let deserialized: InstructionDataInvokeCpi = archived
            .deserialize(&mut Infallible)
            .expect("Deserialization failed");

        // Assert that the deserialized data matches the original
        assert_eq!(original_data, deserialized);
    }
    #[test]
    fn test_archive_pubkey() {
        // Example usage
        let pubkey = WrappedPubkey(Pubkey::new_unique());
        let mut serializer =
            rkyv::ser::serializers::AlignedSerializer::new(rkyv::AlignedVec::new());
        serializer
            .serialize_value(&pubkey)
            .expect("Serialization failed");
        let bytes = serializer.into_inner();

        let archived = unsafe { rkyv::archived_root::<WrappedPubkey>(&bytes) };
        let deserialized: WrappedPubkey = archived
            .deserialize(&mut rkyv::Infallible)
            .expect("Deserialization failed");

        assert_eq!(pubkey, deserialized);
    }
}
