use anchor_lang::{prelude::*, solana_program::msg};
use ark_std::marker::PhantomData;
use groth16_solana::{
    decompression::{decompress_g1, decompress_g2},
    groth16::{Groth16Verifier, Groth16Verifyingkey},
};

use crate::{
    errors::VerifierSdkError,
    light_transaction::{Config, Proof, ProofCompressed},
};

pub struct AppTransaction<'a, const NR_CHECKED_INPUTS: usize, T: Config> {
    pub checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
    pub proof: Proof,
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
    pub verified_proof: bool,
    pub invoked_system_verifier: bool,
}

impl<'a, const NR_CHECKED_INPUTS: usize, T: Config> AppTransaction<'a, NR_CHECKED_INPUTS, T> {
    pub fn new(
        proof: &'a ProofCompressed,
        checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> AppTransaction<'a, NR_CHECKED_INPUTS, T> {
        let proof_a = decompress_g1(&proof.a).unwrap();
        let proof_b = decompress_g2(&proof.b).unwrap();
        let proof_c = decompress_g1(&proof.c).unwrap();
        let proof = Proof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };

        AppTransaction {
            proof,
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
        let mut verifier = Groth16Verifier::new(
            &self.proof.a,
            &self.proof.b,
            &self.proof.c,
            self.checked_public_inputs, // do I need to add the merkle tree? don't think so but think this through
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
                msg!("proof a: {:?}", self.proof.a);
                msg!("proof b: {:?}", self.proof.b);
                msg!("proof c: {:?}", self.proof.c);

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
