#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_verifier_program_two",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-onchain/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-onchain"
}

pub mod processor;
pub mod verifying_key;
pub use processor::*;

use crate::processor::process_shielded_transfer;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use merkle_tree_program::program::MerkleTreeProgram;
use merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use merkle_tree_program::{
    transaction_merkle_tree::state::TransactionMerkleTree, RegisteredVerifier,
};
declare_id!("GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8");

#[constant]
pub const PROGRAM_ID: &'static str = "GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8";

#[error_code]
pub enum ErrorCode {
    #[msg("System program is no valid verifier.")]
    InvalidVerifier,
}

#[program]
pub mod verifier_program_two {
    use super::*;

    /// This instruction is used to invoke this system verifier and can only be invoked via cpi.
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        connecting_hash: [u8; 32],
    ) -> Result<()> {
        process_shielded_transfer(ctx, &proof_a, &proof_b, &proof_c, &connecting_hash)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct LightInstruction<'info> {
    /// CHECK: Cannot be checked with Account because it assumes this program to be the owner
    // CHECK: Signer check to acertain the invoking program ID to be used as a public input.
    pub verifier_state: Signer<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK: Is the same as in integrity hash.
    pub system_program: Program<'info, System>,
    /// CHECK: Is the same as in integrity hash.
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
    /// CHECK:` Is not checked the relayer has complete freedom.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump, seeds::program= MerkleTreeProgram::id())]
    pub token_authority: UncheckedAccount<'info>,
    /// Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.
    /// CHECK: Is the same as in integrity hash.
    #[account(mut, seeds= [__program_id.key().to_bytes().as_ref()], bump, seeds::program= MerkleTreeProgram::id())]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:` It get checked inside the event_call
    pub log_wrapper: UncheckedAccount<'info>,
}
