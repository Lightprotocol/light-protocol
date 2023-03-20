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

use light_macros::light_verifier_accounts;
use merkle_tree_program::program::MerkleTreeProgram;
use merkle_tree_program::utils::constants::TOKEN_AUTHORITY_SEED;
use merkle_tree_program::{poseidon_merkle_tree::state::MerkleTree, RegisteredVerifier};

declare_id!("GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8");
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

#[light_verifier_accounts]
pub struct LightInstruction<'info> {
    /// CHECK: Cannot be checked with Account because it assumes this program to be the owner
    // CHECK: Signer check to acertain the invoking program ID to be used as a public input.
    pub verifier_state: Signer<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK: Is the same as in integrity hash.
    pub system_program: Program<'info, System>,
}
