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

pub mod errors;
pub mod escrow;
pub mod groth16_verifier;
pub mod last_transaction;
pub mod utils;
pub use errors::*;
pub use escrow::*;
pub use groth16_verifier::*;
pub use last_transaction::*;

use crate::last_transaction::{
    instructions_last_transaction::{LastTransactionDeposit, LastTransactionWithdrawal},
    processor_last_transaction::{
        process_last_transaction_deposit, process_last_transaction_withdrawal,
    },
};

use crate::escrow::{
    close_escrow_state::{process_close_escrow, CloseFeeEscrowPda},
    create_escrow_state::{process_create_escrow, CreateEscrowState},
};

use crate::groth16_verifier::{
    process_compute, process_create_verifier_state, Compute, CreateVerifierState, VerifierState,
};
use solana_address_lookup_table_program::{self, state::AddressLookupTable};

use anchor_lang::prelude::*;

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program {
    use super::*;

    /// Creates an escrow pda such that users do not have to execute any transaction.
    /// The escrow amount consists out of the transaction fees the relayer incurs as costs (tx_fee)
    /// plus the relayer fee the relayer charges (relayer fee)
    /// plus the amount the user wants to shield (amount).
    pub fn create_escrow<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, CreateEscrowState<'info>>,
        tx_integrity_hash: [u8; 32],
        tx_fee: u64,
        relayer_fee: [u8; 8],
        amount: u64,
        merkle_tree_index: u64
    ) -> Result<()> {
        process_create_escrow(ctx, tx_integrity_hash, tx_fee, relayer_fee, amount, merkle_tree_index)
    }

    /// Allows the user or relayer to close the escrow pda.
    /// The relayer can close the pda any time in case the transaction fails.
    /// In that case the relayer is reimbursed for the incurred costs for the transactions already
    /// sent. The relayer does not collect the relayer fee thus does not make a profit.
    /// Users can close the account either before the relayer started sending transactions or
    /// after a timeout period.
    pub fn close_escrow<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, CloseFeeEscrowPda<'info>>) -> Result<()> {
        process_close_escrow(ctx)
    }

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification.
    pub fn create_verifier_state<'a, 'b, 'c, 'info> (
        ctx: Context<'a, 'b, 'c, 'info, CreateVerifierState<'info>>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
        amount: [u8; 8],
        ext_data_hash: [u8; 32],
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        // leaf_right: [u8; 32],
        // leaf_left: [u8; 32],
        // recipient: [u8; 32], will just check agaist the pubkey bytes
        // ext_amount: i64,
        // _relayer: [u8; 32],
        fee_amount: [u8; 8],
        mint_pubkey: [u8;32],
        merkle_tree_index: u64,
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos0: [u8; 128],
        encrypted_utxos1: [u8; 64],
        encrypted_utxos2: [u8; 32],
        encrypted_utxos3: [u8; 14],
    ) -> Result<()> {
        msg!("encrypted utxos: {:?}", [encrypted_utxos0.to_vec(), encrypted_utxos1.to_vec(), encrypted_utxos2.to_vec(), encrypted_utxos3.to_vec()].concat());
        process_create_verifier_state(
            ctx,
            proof,
            merkle_root,
            [vec![0u8;24], amount.to_vec()].concat().try_into().unwrap(),
            ext_data_hash,
            nullifiers[0],
            nullifiers[1],
            leaves[0],
            leaves[1],
            0,//ext_amount,
            [vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
            mint_pubkey,
            [encrypted_utxos0.to_vec(), encrypted_utxos1.to_vec(), encrypted_utxos2.to_vec(), encrypted_utxos3.to_vec()].concat(),
            merkle_tree_index,
            relayer_fee
        )
    }

    /// The compute instruction is invoked repeadtly to verify a Groth16 ZKP.
    pub fn compute(ctx: Context<Compute>, _bump: u64) -> Result<()> {
        process_compute(ctx)
    }

    /// Transfers the deposit amount,
    /// inserts nullifiers and Merkle tree leaves.
    pub fn last_transaction_deposit<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, LastTransactionDeposit<'info>>) -> Result<()> {
        process_last_transaction_deposit(ctx)
    }

    /// Transfers the withdrawal amount, pays the relayer,
    /// inserts nullifiers and Merkle tree leaves.
    pub fn last_transaction_withdrawal(ctx: Context<LastTransactionWithdrawal>) -> Result<()> {
        process_last_transaction_withdrawal(ctx)
    }
}
