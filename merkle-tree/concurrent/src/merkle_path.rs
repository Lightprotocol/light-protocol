#[repr(C)]
pub struct MerklePath<const MAX_DEPTH: usize> {
    pub proof: [[u8; 32]; MAX_DEPTH],
    pub leaf: [u8; 32],
    pub index: u64,
}
