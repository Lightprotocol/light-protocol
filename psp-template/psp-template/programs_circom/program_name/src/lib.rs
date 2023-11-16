use anchor_lang::prelude::*;
use groth16_solana::groth16::Groth16Verifier;
pub mod errors;
pub mod utils;
pub mod verifying_key_{{circom-name}};

declare_id!("{{program-id}}");

#[constant]
pub const PROGRAM_ID: &str = "{{program-id}}";

#[program]
pub mod {{rust-name}} {
    use super::*;
    use crate::errors::VerifierError;
    use crate::utils::negate_proof_a;
    use crate::verifying_key_{{circom-name}}::VERIFYINGKEY_{{VERIFYING_KEY_NAME}};
    #[allow(clippy::result_large_err)]
    pub fn verify_proof(
        _ctx: Context<Verifier>,
        public_inputs: [[u8; 32]; 1],
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) -> Result<()> {
        msg!("Verifying proof...");

        let proof_a_neg = negate_proof_a(proof_a);

        let mut verifier = Groth16Verifier::new(
            &proof_a_neg,
            &proof_b,
            &proof_c,
            &public_inputs,
            &VERIFYINGKEY_{{VERIFYING_KEY_NAME}},
        )
        .unwrap();

        match verifier.verify() {
            Ok(_) => {
                msg!("Proof verified");
                Ok(())
            }
            Err(e) => {
                msg!("Proof verification failed: {:?}", e);
                Err(VerifierError::ProofVerificationFailed.into())
            }
        }
    }
}

#[derive(Accounts)]
pub struct Verifier {}
