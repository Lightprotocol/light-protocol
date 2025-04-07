use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_merkle_tree_metadata::queue::QueueMetadata;

#[repr(C)]
#[aligned_sized(anchor)]
#[derive(Pod, Debug, Default, Zeroable, Clone, Copy)]
pub struct QueueAccount {
    pub metadata: QueueMetadata,
}
