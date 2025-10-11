use light_compressed_account::{
    compressed_account::{hash_with_hashed_values, CompressedAccountData},
    instruction_data::{
        traits::{InputAccount, OutputAccount},
        zero_copy::ZPackedMerkleContext,
    },
    pubkey::Pubkey,
    CompressedAccountError,
};
use zerocopy::{
    little_endian::{U16, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
};

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned,
)]
pub struct CpiContextOutAccount {
    pub owner: Pubkey,
    pub has_data: u8,
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    pub output_merkle_tree_index: u8,
    /// Lamports.
    pub lamports: U64,
    // No data
    pub with_address: u8,
    pub address: [u8; 32],
}

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned,
)]
pub struct CpiContextInAccount {
    pub owner: Pubkey,
    pub has_data: u8,
    pub discriminator: [u8; 8],
    /// Data hash
    pub data_hash: [u8; 32],
    /// Merkle tree context.
    pub merkle_context: ZPackedMerkleContext,
    /// Root index.
    pub root_index: U16,
    /// Lamports.
    pub lamports: U64,
    pub with_address: u8,
    /// Optional address.
    pub address: [u8; 32],
}

impl InputAccount<'_> for CpiContextInAccount {
    fn owner(&self) -> &Pubkey {
        &self.owner
    }

    fn lamports(&self) -> u64 {
        self.lamports.get()
    }

    fn address(&self) -> Option<[u8; 32]> {
        if self.with_address == 1 {
            Some(self.address)
        } else {
            None
        }
    }

    fn merkle_context(&self) -> ZPackedMerkleContext {
        self.merkle_context
    }

    fn has_data(&self) -> bool {
        self.has_data != 0
    }

    fn data(&self) -> Option<CompressedAccountData> {
        if self.has_data() {
            Some(CompressedAccountData {
                discriminator: self.discriminator,
                data: Vec::new(),
                data_hash: self.data_hash,
            })
        } else {
            None
        }
    }

    fn skip(&self) -> bool {
        false
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        hash_with_hashed_values(
            &self.lamports.get(),
            self.address().as_ref().map(|x| x.as_slice()),
            Some((self.discriminator.as_slice(), self.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }

    fn root_index(&self) -> u16 {
        self.root_index.get()
    }
}

impl OutputAccount<'_> for CpiContextOutAccount {
    fn lamports(&self) -> u64 {
        self.lamports.get()
    }

    fn address(&self) -> Option<[u8; 32]> {
        if self.with_address == 1 {
            Some(self.address)
        } else {
            None
        }
    }

    fn has_data(&self) -> bool {
        self.has_data != 0
    }

    fn skip(&self) -> bool {
        false
    }

    fn data(&self) -> Option<CompressedAccountData> {
        if self.has_data() {
            Some(CompressedAccountData {
                discriminator: self.discriminator,
                data: Vec::new(),
                data_hash: self.data_hash,
            })
        } else {
            None
        }
    }

    fn owner(&self) -> Pubkey {
        self.owner
    }

    fn merkle_tree_index(&self) -> u8 {
        self.output_merkle_tree_index
    }

    fn hash_with_hashed_values(
        &self,
        owner_hashed: &[u8; 32],
        merkle_tree_hashed: &[u8; 32],
        leaf_index: &u32,
        is_batched: bool,
    ) -> Result<[u8; 32], CompressedAccountError> {
        hash_with_hashed_values(
            &self.lamports.get(),
            self.address().as_ref().map(|x| x.as_slice()),
            Some((self.discriminator.as_slice(), self.data_hash.as_slice())),
            owner_hashed,
            merkle_tree_hashed,
            leaf_index,
            is_batched,
        )
    }
}
