use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::invoke_cpi::InstructionDataInvokeCpi;

/// Collects instruction data without executing a compressed transaction.
/// Signer checks are performed on instruction data.
/// Collected instruction data is combined with the instruction data of the executing cpi,
/// and executed as a single transaction.
/// This enables to use input compressed accounts that are owned by multiple programs,
/// with one zero-knowledge proof.
#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default)]
#[account]
#[repr(C)]
pub struct CpiContextAccount {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
    // Offset 72
    pub context: Vec<InstructionDataInvokeCpi>,
}

#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default)]
#[account]
#[repr(C)]
pub struct CpiContextAccount2 {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
}

impl CpiContextAccount {
    pub fn init(&mut self, associated_merkle_tree: Pubkey) {
        self.associated_merkle_tree = associated_merkle_tree;
        self.context = Vec::new();
    }
}
