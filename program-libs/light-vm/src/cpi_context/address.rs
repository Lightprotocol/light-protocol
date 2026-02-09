use light_compressed_account::instruction_data::traits::NewAddress;
use zerocopy::{little_endian::U16, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned,
)]
pub struct CpiContextNewAddressParamsAssignedPacked {
    pub owner: [u8; 32],
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: U16,
    pub assigned_to_account: u8, // bool
    pub assigned_account_index: u8,
}

impl NewAddress<'_> for CpiContextNewAddressParamsAssignedPacked {
    fn seed(&self) -> [u8; 32] {
        self.seed
    }

    fn address_queue_index(&self) -> u8 {
        self.address_queue_account_index
    }

    fn address_merkle_tree_account_index(&self) -> u8 {
        self.address_merkle_tree_account_index
    }

    fn address_merkle_tree_root_index(&self) -> u16 {
        self.address_merkle_tree_root_index.get()
    }

    fn assigned_compressed_account_index(&self) -> Option<usize> {
        if self.assigned_to_account == 1 {
            Some(self.assigned_account_index as usize)
        } else {
            None
        }
    }
    fn owner(&self) -> Option<&[u8; 32]> {
        Some(&self.owner)
    }
}
