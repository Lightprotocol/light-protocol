use anchor_lang::{prelude::*, solana_program::msg};
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_std::{marker::PhantomData, vec::Vec};

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::{errors::VerifierSdkError, light_transaction::Proof, utils::change_endianness};

use std::ops::Neg;

type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>;
use crate::light_transaction::Config;

// #[derive(Clone)]
pub struct AppTransaction<'a, T: Config> {
    pub checked_public_inputs: Vec<Vec<u8>>,
    pub proof: Proof,
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
    pub verified_proof: bool,
    pub invoked_system_verifier: bool,
}

impl<'a, T: Config> AppTransaction<'a, T> {
    pub fn new(
        proof: &'a Proof,
        checked_public_inputs: Vec<Vec<u8>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> AppTransaction<'a, T> {
        let proof_a_neg_g1: G1 =
            <G1 as FromBytes>::read(&*[&change_endianness(&proof.a)[..], &[0u8][..]].concat())
                .unwrap();
        let mut proof_a_neg_buf = [0u8; 65];
        <G1 as ToBytes>::write(&proof_a_neg_g1.neg(), &mut proof_a_neg_buf[..]).unwrap();
        let mut proof_a_neg = [0u8; 64];
        proof_a_neg.copy_from_slice(&proof_a_neg_buf[..64]);

        let proof_a_neg: [u8; 64] = change_endianness(&proof_a_neg).try_into().unwrap();
        let proof = Proof {
            a: proof_a_neg,
            b: proof.b,
            c: proof.c,
        };

        AppTransaction {
            proof: proof,
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
        // do I need to add the merkle tree? don't think so but think this through
        let mut public_inputs = Vec::new();

        for input in self.checked_public_inputs.iter() {
            public_inputs.push(input.as_slice());
        }

        let mut verifier = Groth16Verifier::new(
            &self.proof.a,
            &self.proof.b,
            &self.proof.c,
            public_inputs.as_slice(),
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
