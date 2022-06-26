pub mod groth16_verifier;
pub mod utils;
pub mod last_transaction;
pub mod errors;
pub mod escrow;
pub use groth16_verifier::*;
pub use escrow::*;
pub use errors::*;
pub use last_transaction::*;

use crate::last_transaction::{
    instructions_last_transaction::{
        LastTransactionDeposit,
        LastTransactionWithdrawal
    },
    processor_last_transaction::{
        process_last_transaction_deposit,
        process_last_transaction_withdrawal
    }
};

use crate::escrow::{
    close_escrow_state::{
        CloseFeeEscrowPda,
        process_close_fee_escrow
    },
    create_escrow_state::{
        CreateEscrowState,
        process_create_escrow_state
    }
};

use crate::groth16_verifier::{
    process_compute,
    Compute,
    process_create_verifier_state,
    VerifierState,
    CreateVerifierState
};

use anchor_lang::prelude::*;
use merkle_tree_program::{
    self,
    utils::config::STORAGE_SEED,
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
    use super::*;


    pub fn create_escrow_state(
        ctx: Context<CreateEscrowState>,
        tx_integrity_hash: [u8; 32],
        tx_fee: u64,
        relayer_fee: [u8;8],
        amount: u64
    ) -> Result<()> {
        process_create_escrow_state(
            ctx,
            tx_integrity_hash,
            tx_fee,
            relayer_fee,
            amount
        )
    }

    pub fn close_fee_escrow_pda(ctx: Context<CloseFeeEscrowPda>) -> Result<()> {
        process_close_fee_escrow(ctx)
    }

    // Creates and initializes a state account to save state of a verification for one transaction
    pub fn create_verifier_state(
        ctx: Context<CreateVerifierState>,
        proof: [u8; 256],
        root_hash: [u8; 32],
        amount: [u8; 32],
        tx_integrity_hash: [u8; 32],
        nullifier0: [u8; 32],
        nullifier1: [u8; 32],
        leaf_right: [u8; 32],
        leaf_left: [u8; 32],
        recipient: [u8; 32],
        ext_amount: [u8; 8],
        _relayer: [u8; 32],
        relayer_fee: [u8; 8],
        encrypted_utxos: [u8; 256],
        merkle_tree_index: [u8; 1]
    ) -> Result<()> {
        process_create_verifier_state(
            ctx,
            proof,
            root_hash,
            amount,
            tx_integrity_hash,
            nullifier0,
            nullifier1,
            leaf_right,
            leaf_left,
            recipient,
            ext_amount,
            _relayer,
            relayer_fee,
            encrypted_utxos,
            merkle_tree_index
        )
    }

    // Verifies a Groth16 ZKP
    pub fn compute(ctx: Context<Compute>, _bump: u64) -> Result<()> {
        process_compute(ctx)
    }

    // Transfers the deposit amount,
    // inserts nullifiers and Merkle tree leaves
    pub fn last_transaction_deposit(ctx: Context<LastTransactionDeposit>) -> Result<()> {
        process_last_transaction_deposit(ctx)
    }


    // Transfers the withdrawal amount, pays the relayer,
    // inserts nullifiers and Merkle tree leaves
    pub fn last_transaction_withdrawal(ctx: Context<LastTransactionWithdrawal>) -> Result<()> {
        process_last_transaction_withdrawal(ctx)
    }
}
