use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct AddressQueueAccount {
    pub queue: [u8; 112008],
}

#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct AddressMerkleTreeAccount {
    pub merkle_tree: [u8; 2173568],
}
