use anchor_lang::{prelude::*, solana_program::msg};
use ark_std::marker::PhantomData;

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::errors::VerifierSdkError;

use crate::light_transaction::Config;

#[derive(Clone)]
pub struct AppTransaction<'a, const NR_CHECKED_INPUTS: usize, T: Config> {
    pub checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
    pub proof_a: &'a [u8; 32],
    pub proof_b: &'a [u8; 64],
    pub proof_c: &'a [u8; 32],
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
    pub verified_proof: bool,
    pub invoked_system_verifier: bool,
}

impl<'a, const NR_CHECKED_INPUTS: usize, T: Config> AppTransaction<'a, NR_CHECKED_INPUTS, T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proof_a: &'a [u8; 32],
        proof_b: &'a [u8; 64],
        proof_c: &'a [u8; 32],
        checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> AppTransaction<'a, NR_CHECKED_INPUTS, T> {
        AppTransaction {
            proof_a,
            proof_b,
            proof_c,
            verified_proof: false,
            invoked_system_verifier: false,
            e_phantom: PhantomData,
            verifyingkey,
            checked_public_inputs,
        }
    }

    // /// Transact is a wrapper function which verifies the zero knowledge proof and cpi's to the selected verifier.
    // pub fn transact(&mut self) -> Result<()> {
    //     self.verify()?;
    //     self.send_transaction()?;
    //     self.check_completion()
    // }

    /// Verifies a Goth16 zero knowledge proof over the bn254 curve.
    pub fn verify(&mut self) -> Result<()> {
        let proof_a = groth16_solana::decompression::decompress_g1(self.proof_a, true).unwrap();
        let proof_b = groth16_solana::decompression::decompress_g2(self.proof_b).unwrap();
        let proof_c = groth16_solana::decompression::decompress_g1(self.proof_c, false).unwrap();
        let mut verifier = Groth16Verifier::new(
            &proof_a,
            &proof_b,
            &proof_c,
            &self.checked_public_inputs, // do I need to add the merkle tree? don't think so but think this through
            self.verifyingkey,
        )
        .unwrap();

        match verifier.verify() {
            Ok(_) => {
                self.verified_proof = true;
                Ok(())
            }
            Err(e) => {
                msg!("Public Inputs:");
                msg!("checked_public_inputs {:?}", self.checked_public_inputs);
                msg!("proof a: {:?}", self.proof_a);
                msg!("proof b: {:?}", self.proof_b);
                msg!("proof c: {:?}", self.proof_c);

                msg!("error {:?}", e);
                err!(VerifierSdkError::ProofVerificationFailed)
            }
        }
    }

    // TODO: implement, has to pass contex struct otherwise not really worth
    pub fn send_transaction() {
        // match for configured system verifier program id
    }

    // pub fn check_completion(&self) -> Result<()> {
    //     if self.invoked_system_verifier
    //         && self.verified_proof
    //     {
    //         return Ok(());
    //     }
    //     msg!("verified_proof {}", self.verified_proof);
    //     err!(VerifierSdkError::AppTransactionIncomplete)
    // }
}
