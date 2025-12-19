use std::fmt;

use solana_sdk::{instruction::InstructionError, pubkey::Pubkey, transaction::TransactionError};
use thiserror::Error;

/// Matches `light_verifier::VerifierError::ProofVerificationFailed`.
const PROOF_VERIFICATION_FAILED_ERROR_CODE: u32 = 13006;

fn fmt_root_prefix(root: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        root[0], root[1], root[2], root[3]
    )
}

#[derive(Debug, Error)]
pub enum V2Error {
    #[error("{}", .0)]
    RootMismatch(#[from] RootMismatchError),

    #[error("{}", .0)]
    IndexerLag(#[from] IndexerLagError),

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

    #[error("transaction {signature} timed out: {context}")]
    TransactionTimeout { signature: String, context: String },
}

impl V2Error {
    pub fn from_transaction_error(tree: Pubkey, err: &TransactionError) -> Self {
        let message = format!("{:?}", err);
        let custom_code = match err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => Some(*code),
            _ => None,
        };

        if matches!(custom_code, Some(PROOF_VERIFICATION_FAILED_ERROR_CODE)) {
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

    pub fn root_mismatch(
        tree: Pubkey,
        expected: [u8; 32],
        indexer: [u8; 32],
        onchain: [u8; 32],
    ) -> Self {
        RootMismatchError {
            tree,
            expected,
            indexer,
            onchain,
        }
        .into()
    }

    pub fn indexer_lag(tree: Pubkey, expected: [u8; 32], indexer: [u8; 32]) -> Self {
        IndexerLagError {
            tree,
            expected,
            indexer,
        }
        .into()
    }
}

#[derive(Debug)]
pub struct RootMismatchError {
    pub tree: Pubkey,
    pub expected: [u8; 32],
    pub indexer: [u8; 32],
    pub onchain: [u8; 32],
}

impl fmt::Display for RootMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "root mismatch for tree {}: expected {}, indexer {}, onchain {}",
            self.tree,
            fmt_root_prefix(&self.expected),
            fmt_root_prefix(&self.indexer),
            fmt_root_prefix(&self.onchain)
        )
    }
}

impl std::error::Error for RootMismatchError {}

#[derive(Debug)]
pub struct IndexerLagError {
    pub tree: Pubkey,
    pub expected: [u8; 32],
    pub indexer: [u8; 32],
}

impl fmt::Display for IndexerLagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "indexer lag for tree {}: expected {}, indexer {}",
            self.tree,
            fmt_root_prefix(&self.expected),
            fmt_root_prefix(&self.indexer)
        )
    }
}

impl std::error::Error for IndexerLagError {}
