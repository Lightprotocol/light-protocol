use crate::InstructionDataInvokeCpi;
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

/// Collects instruction data without executing a compressed transaction.
/// Signer checks are performed on instruction data.
/// Collected instruction data is combined with the instruction data of the executing cpi,
/// and executed as a single transaction.
/// This enables to use input compressed accounts that are owned by multiple programs,
/// with one zero-knowledge proof.
#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default)]
#[account]
pub struct CpiContextAccount {
    pub associated_merkle_tree: Pubkey,
    pub context: Vec<InstructionDataInvokeCpi>,
}

// TODO: feature gate and make non-default feature
// this is not secure
impl CpiContextAccount {
    pub fn init(&mut self, associated_merkle_tree: Pubkey) {
        self.associated_merkle_tree = associated_merkle_tree;
        self.context = Vec::new();
    }
}
