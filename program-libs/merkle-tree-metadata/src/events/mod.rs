pub mod batch;
pub mod concurrent;

use batch::BatchEvent;
use concurrent::*;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
#[repr(C)]
pub enum MerkleTreeEvent {
    V1(ChangelogEvent),
    V2(NullifierEvent),
    V3(IndexedMerkleTreeEvent),
    BatchAppend(BatchEvent),
    BatchNullify(BatchEvent),
    BatchAddressAppend(BatchEvent),
}
