#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_verifier_program_one",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-onchain/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-onchain"
}

pub mod processor;
pub mod verifying_key;

pub use processor::*;

use crate::processor::TransactionConfig;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::state::VerifierState10Ins;
use merkle_tree_program::{
    program::MerkleTreeProgram, transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::TOKEN_AUTHORITY_SEED, RegisteredVerifier,
};

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
        public_amount_spl: [u8; 32],
        nullifiers: [[u8; 32]; 10],
        leaves: [[u8; 32]; 2],
        public_amount_sol: [u8; 32],
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        // let mut nullifiers_vec = Vec::<Vec<u8>>::new();
        // for nullifier in nullifiers {
        //     nullifiers_vec.push(nullifier.to_vec());
        // }
        let proof_a = [0u8; 64];
        let proof_b = [0u8; 128];
        let proof_c = [0u8; 64];
        process_transfer_10_ins_2_outs_first(
            ctx,
            &proof_a,
            &proof_b,
            &proof_c,
            &public_amount_spl,
            &nullifiers,
            &[[leaves[0], leaves[1]]; 1],
            &public_amount_sol,
            &encrypted_utxos,
            &root_index,
            &relayer_fee,
        )
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn shielded_transfer_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) -> Result<()> {
        process_transfer_10_ins_2_outs_second(ctx, &proof_a, &proof_b, &proof_c, [0u8; 32])
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, CloseVerifierState<'info>>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 3000/*8 + 32 * 6 + 10 * 32 + 2 * 32 + 512 + 16 + 128*/, payer = signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionConfig>>>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionSecond<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionConfig>>>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_spl: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub sender_sol: UncheckedAccount<'info>,
    /// CHECK:` Is checked depending on deposit or withdrawal.
    #[account(mut)]
    pub recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut)]
    pub relayer_recipient_sol: UncheckedAccount<'info>,
    /// CHECK:` Is checked when it is used during spl withdrawals.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Box<Account<'info, VerifierState10Ins<TransactionConfig>>>,
}
