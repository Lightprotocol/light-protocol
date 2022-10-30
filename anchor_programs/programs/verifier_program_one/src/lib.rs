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

pub mod processor;
pub mod verifying_key;

pub use processor::*;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::utils::create_pda::create_and_check_pda;
use merkle_tree_program::{
    errors::ErrorCode as MerkleTreeError, initialize_new_merkle_tree_18::PreInsertedLeavesIndex,
    poseidon_merkle_tree::state::MerkleTree, program::MerkleTreeProgram,
    utils::constants::MERKLE_TREE_AUTHORITY_SEED, MerkleTreeAuthority, RegisteredVerifier,
};

use crate::processor::process_shielded_transfer_first;
use crate::processor::TransactionConfig;
use light_verifier_sdk::state::VerifierStateTenNF;

declare_id!("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");

#[program]
pub mod verifier_program_one {
    use super::*;

    /// This instruction is the first step of a shielded transaction with 10 inputs and 2 outputs.
    /// It creates and initializes a verifier state account which stores public inputs and other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the verification and
    /// protocol logicin the second transaction.
    pub fn shielded_transfer_first<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
        amount: Vec<u8>,
        nullifiers: [[u8; 32]; 10],
        leaves: [[u8; 32]; 2],
        fee_amount: Vec<u8>,
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        let mut nfs = Vec::<Vec<u8>>::new();
        for nf in nullifiers {
            nfs.push(nf.to_vec());
        }
        process_shielded_transfer_first(
            ctx,
            vec![0u8; 256],
            amount,
            nfs,
            vec![vec![leaves[0].to_vec(), leaves[1].to_vec()]],
            fee_amount,
            encrypted_utxos,
            &root_index,
            &relayer_fee,
        )
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn shielded_transfer_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
        proof: Vec<u8>,
    ) -> Result<()> {
        process_shielded_transfer_second(ctx, proof, vec![0u8; 32])
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, CloseVerifierState<'info>>,
    ) -> Result<()> {
        Ok(())
    }
}

pub const VERIFIER_STATE_SEED: &[u8] = b"VERIFIER_STATE";

/// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [VERIFIER_STATE_SEED], bump, space= 8 + 2048 /*776*/, payer = signing_address )]
    pub verifier_state: Account<'info, VerifierStateTenNF<TransactionConfig>>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionSecond<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierStateTenNF<TransactionConfig>>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    pub rent: Sysvar<'info, Rent>,
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    #[account(
        mut,seeds= [merkle_tree.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id()
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_fee: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_fee: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient: UncheckedAccount<'info>,
    /// CHECK:` Is checked when it is used during sol deposits.
    #[account(mut)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK:` Is checked when it is used during spl withdrawals.
    #[account(mut)]
    pub token_authority: UncheckedAccount<'info>,
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierStateTenNF<TransactionConfig>>,
}
