/*
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_market_place_verifier",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}
*/

pub mod processor;
pub mod verifying_key;
pub use processor::*;

use crate::processor::TransactionsConfig;
use crate::processor::{
    process_transfer_4_ins_4_outs_4_checked_first, process_transfer_4_ins_4_outs_4_checked_second,
};
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_verifier_sdk::{light_transaction::VERIFIER_STATE_SEED, state::VerifierState10Ins};
use merkle_tree_program::program::MerkleTreeProgram;
use merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use merkle_tree_program::{
    initialize_new_merkle_tree_18::PreInsertedLeavesIndex, poseidon_merkle_tree::state::MerkleTree,
};
use verifier_program_two::{self, program::VerifierProgramTwo};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum MarketPlaceError {
    #[msg("The offer expired.")]
    OfferExpired,
}
#[program]
pub mod mock_verifier {
    use anchor_lang::solana_program::keccak::hash;

    use super::*;

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn shielded_transfer_first<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
        public_amount: Vec<u8>,
        nullifiers: [[u8; 32]; 4],
        leaves: [[[u8; 32]; 2]; 2],
        fee_amount: Vec<u8>,
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
    ) -> Result<()> {
        let mut nullifiers_vec = Vec::<Vec<u8>>::new();
        for nullifier in nullifiers {
            nullifiers_vec.push(nullifier.to_vec());
        }

        let mut leaves_vec = Vec::<Vec<Vec<u8>>>::new();
        for leaves_pair in leaves {
            leaves_vec.push(vec![leaves_pair[0].to_vec(), leaves_pair[1].to_vec()]);
        }

        process_transfer_4_ins_4_outs_4_checked_first(
            ctx,
            vec![0u8; 256],
            public_amount,
            nullifiers_vec,
            leaves_vec,
            fee_amount,
            Vec::new(),
            encrypted_utxos.to_vec(),
            vec![0u8; 32],
            &root_index,
            &relayer_fee,
        )
    }

    /// This instruction is the second step of a shieled transaction.
    /// The proof is verified with the parameters saved in the first transaction.
    /// At successful verification protocol logic is executed.
    pub fn shielded_transfer_second<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
        proof_app: Vec<u8>,
        proof_verifier: Vec<u8>,
        connecting_hash: Vec<u8>,
    ) -> Result<()> {
        ctx.accounts.verifier_state.checked_public_inputs.insert(
            0,
            [
                vec![0u8],
                hash(&ctx.program_id.to_bytes()).try_to_vec()?[1..].to_vec(),
            ]
            .concat(),
        );
        ctx.accounts
            .verifier_state
            .checked_public_inputs
            .insert(1, connecting_hash);
        process_transfer_4_ins_4_outs_4_checked_second(ctx, proof_app, proof_verifier)
    }

    /// Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.
    pub fn close_verifier_state<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, CloseVerifierState<'info>>,
    ) -> Result<()> {
        Ok(())
    }
}

// Send and stores data.
#[derive(Accounts)]
pub struct LightInstructionFirst<'info> {
    /// First transaction, therefore the signing address is not checked but saved to be checked in future instructions.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(init, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, space= 2548/*8 + 32 * 6 + 10 * 32 + 2 * 32 + 512 + 16 + 128*/, payer = signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<TransactionsConfig>>,
}

/// Executes light transaction with state created in the first instruction.
#[derive(Accounts)]
pub struct LightInstructionSecond<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<TransactionsConfig>>,
    pub system_program: Program<'info, System>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    #[account(
        mut,seeds= [merkle_tree.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id()
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump, seeds::program= VerifierProgramTwo::id())]
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
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK: Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    #[account(mut, seeds= [VerifierProgramTwo::id().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: UncheckedAccount<'info>, //Account<'info, RegisteredVerifier>,
    pub verifier_program: Program<'info, VerifierProgramTwo>,
}

#[derive(Accounts)]
pub struct CloseVerifierState<'info> {
    #[account(mut, address=verifier_state.signer)]
    pub signing_address: Signer<'info>,
    #[account(mut, seeds = [&signing_address.key().to_bytes(), VERIFIER_STATE_SEED], bump, close=signing_address )]
    pub verifier_state: Account<'info, VerifierState10Ins<TransactionsConfig>>,
}
