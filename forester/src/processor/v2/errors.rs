use solana_sdk::{instruction::InstructionError, pubkey::Pubkey, transaction::TransactionError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum V2Error {
    #[error(
        "root mismatch for tree {tree}: expected {expected:?}[..4], indexer {indexer:?}[..4], onchain {onchain:?}[..4]"
    )]
    RootMismatch {
        tree: Pubkey,
        expected: [u8; 32],
        indexer: [u8; 32],
        onchain: [u8; 32],
    },

    #[error("indexer lag for tree {tree}: expected {expected:?}[..4], indexer {indexer:?}[..4]")]
    IndexerLag {
        tree: Pubkey,
        expected: [u8; 32],
        indexer: [u8; 32],
    },

    #[error("stale tree for tree_id {tree_id}: {details}")]
    StaleTree { tree_id: String, details: String },

    #[error("proof patch failed for tree_id {tree_id}: {details}")]
    ProofPatchFailed { tree_id: String, details: String },

    #[error("hashchain mismatch for tree_id {tree_id}: {details}")]
    HashchainMismatch { tree_id: String, details: String },

    #[error("circuit constraint failure for tree {tree}: code={code:?} {message}")]
    CircuitConstraint {
        tree: Pubkey,
        code: Option<u32>,
        message: String,
    },

    #[error("transaction failed for tree {tree}: {message}")]
    TransactionFailed { tree: Pubkey, message: String },
}

impl V2Error {
    pub fn from_transaction_error(tree: Pubkey, err: &TransactionError) -> Self {
        let message = format!("{:?}", err);
        let custom_code = match err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => Some(*code),
            _ => None,
        };

        if matches!(custom_code, Some(13006)) {
            return V2Error::CircuitConstraint {
                tree,
                code: custom_code,
                message,
            };
        }

        V2Error::TransactionFailed { tree, message }
    }

    pub fn is_constraint(&self) -> bool {
        matches!(self, V2Error::CircuitConstraint { .. })
    }

    pub fn is_hashchain_mismatch(&self) -> bool {
        matches!(self, V2Error::HashchainMismatch { .. })
    }
}
