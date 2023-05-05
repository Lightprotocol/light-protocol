use anchor_lang::{prelude::*, solana_program::msg};
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_std::{marker::PhantomData, vec::Vec};

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::{errors::VerifierSdkError, utils::change_endianness};

use std::ops::Neg;

type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>;
use crate::light_transaction::Config;

#[derive(Clone)]
pub struct AppTransaction<'a, T: Config> {
    pub checked_public_inputs: Vec<Vec<u8>>,
    pub proof_a: [u8; 64],
    pub proof_b: &'a [u8; 128],
    pub proof_c: &'a [u8; 64],
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
    pub verified_proof: bool,
    pub invoked_system_verifier: bool,
}

impl<'a, T: Config> AppTransaction<'a, T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proof_a: &'a [u8; 64],
        proof_b: &'a [u8; 128],
        proof_c: &'a [u8; 64],
        checked_public_inputs: Vec<Vec<u8>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> AppTransaction<'a, T> {
        msg!("app proof_a {:?}", proof_a);
        let proof_a_neg_g1: G1 = <G1 as FromBytes>::read(
            &*[&change_endianness(proof_a.as_slice())[..], &[0u8][..]].concat(),
        )
        .unwrap();
        let mut proof_a_neg = [0u8; 65];
        <G1 as ToBytes>::write(&proof_a_neg_g1.neg(), &mut proof_a_neg[..]).unwrap();

        let proof_a_neg: [u8; 64] = change_endianness(&proof_a_neg[..64]).try_into().unwrap();
        AppTransaction {
            proof_a: proof_a_neg,
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
        // do I need to add the merkle tree? don't think so but think this through
        let mut public_inputs = Vec::new();

        for input in self.checked_public_inputs.iter() {
            public_inputs.push(input.as_slice());
        }
        msg!("public_inputs: {:?}", public_inputs);

        let mut verifier = Groth16Verifier::new(
            &self.proof_a,
            self.proof_b,
            self.proof_c,
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
