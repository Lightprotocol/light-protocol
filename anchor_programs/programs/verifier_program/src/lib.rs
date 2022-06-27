

/*
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_verifier_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}
*/

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
    self
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod verifier_program {
    use super::*;

    /// Creates an escrow pda such that users do not have to execute any transaction.
    /// The escrow amount consists out of the transaction fees the relayer incurs as costs (tx_fee)
    /// plus the relayer fee the relayer charges (relayer fee)
    /// plus the amount the user wants to shield (amount).
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

    /// Allows the user or relayer to close the escrow pda.
    /// The relayer can close the pda any time in case the transaction fails.
    /// In that case the relayer is reimbursed for the incurred costs for the transactions already
    /// sent. The relayer does not collect the relayer fee thus does not make a profit.
    /// Users can close the account either before the relayer started sending transactions or
    /// after a timeout period.
    pub fn close_fee_escrow_pda(ctx: Context<CloseFeeEscrowPda>) -> Result<()> {
        process_close_fee_escrow(ctx)
    }

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification.
    pub fn create_verifier_state(
        ctx: Context<CreateVerifierState>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
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
            merkle_root,
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

    /// The compute instruction is invoked repeadtly to verify a Groth16 ZKP.
    pub fn compute(ctx: Context<Compute>, _bump: u64) -> Result<()> {
        process_compute(ctx)
    }

    /// Transfers the deposit amount,
    /// inserts nullifiers and Merkle tree leaves.
    pub fn last_transaction_deposit(ctx: Context<LastTransactionDeposit>) -> Result<()> {
        process_last_transaction_deposit(ctx)
    }

    /// Transfers the withdrawal amount, pays the relayer,
    /// inserts nullifiers and Merkle tree leaves.
    pub fn last_transaction_withdrawal(ctx: Context<LastTransactionWithdrawal>) -> Result<()> {
        process_last_transaction_withdrawal(ctx)
    }
}
