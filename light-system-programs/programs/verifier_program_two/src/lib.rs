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
use light_macros::light_verifier_accounts;
pub use processor::*;

use anchor_lang::prelude::*;

use merkle_tree_program::program::MerkleTreeProgram;
declare_id!("2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86");

#[constant]
pub const PROGRAM_ID: &str = "2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86";

#[error_code]
pub enum ErrorCode {
    #[msg("System program is no valid verifier.")]
    InvalidVerifier,
}

#[program]
pub mod verifier_program_two {
    use light_verifier_sdk::light_transaction::Proof;

    use super::*;

    /// This instruction is used to invoke this system verifier and can only be invoked via cpi.
    pub fn shielded_transfer_inputs<'info>(
        ctx: Context<'_, '_, '_, 'info, LightInstruction<'info>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        connecting_hash: [u8; 32],
    ) -> Result<()> {
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
        process_shielded_transfer(ctx, &proof, &connecting_hash)?;
        Ok(())
    }
}

#[light_verifier_accounts(sol, spl)]
#[derive(Accounts)]
pub struct LightInstruction<'info> {
    pub verifier_state: Signer<'info>,
}
