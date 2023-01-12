use anchor_lang::{
    prelude::*,
    solana_program::msg,
};
use ark_ff::{
    bytes::{FromBytes, ToBytes},
};
use ark_std::{marker::PhantomData, vec::Vec};

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};

use crate::{
    errors::VerifierSdkError,
    utils::change_endianness,
};

use std::ops::Neg;

type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>;
use crate::light_transaction::Config;
// pub trait Config {
//     /// Number of nullifiers to be inserted with the AppTransaction.
//     const NR_NULLIFIERS: usize;
//     /// Number of output utxos.
//     const NR_LEAVES: usize;
//     /// Number of checked public inputs.
//     const NR_CHECKED_PUBLIC_INPUTS: usize;
//     /// Program ID of the app verifier program.
//     const APP_ID: [u8; 32];
//     /// Program ID of the system verifier program.
//     const SYSTEM_VERIFIER_ID: [u8; 32];
// }

#[derive(Clone)]
pub struct AppTransaction<'a, T: Config> {
    // pub connecting_hash: Vec<u8>,
    pub checked_public_inputs: Vec<Vec<u8>>,
    pub proof_a: Vec<u8>,
    pub proof_b: Vec<u8>,
    pub proof_c: Vec<u8>,
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
    pub verified_proof: bool,
    pub invoked_system_verifier: bool
}

impl<T: Config> AppTransaction<'_, T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new<'a>(
        proof: Vec<u8>,
        checked_public_inputs: Vec<Vec<u8>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> AppTransaction<T> {

        msg!("commented negate proof a");
        let proof_a: G1 =
            <G1 as FromBytes>::read(&*[&change_endianness(&proof[0..64])[..], &[0u8][..]].concat())
                .unwrap();
        let mut proof_a_neg = [0u8; 65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();

        AppTransaction {
            proof_a: change_endianness(&proof_a_neg[..64]).to_vec(),
            proof_b: proof[64..64 + 128].to_vec(),
            proof_c: proof[64 + 128..256].to_vec(),
            verified_proof: false,
            invoked_system_verifier: false,
            e_phantom: PhantomData,
            verifyingkey,
            checked_public_inputs
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
            public_inputs.push(input.to_vec());
        }

        let mut verifier = Groth16Verifier::new(
            self.proof_a.clone(),
            self.proof_b.clone(),
            self.proof_c.clone(),
            public_inputs,
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
                // msg!("connecting_hash {:?}", self.connecting_hash);
                msg!("checked_public_inputs {:?}", self.checked_public_inputs);
                msg!("error {:?}", e);
                err!(VerifierSdkError::ProofVerificationFailed)
            }
        }
    }

    // pub fn send_transaction() {
    //     // match for configured system verifier program id
    //     verifier_program_two::instruction_second
    //     let (seed, bump) = utils::cpi_instructions::get_seeds(self.program_id, self.merkle_tree_program_id)?;
    //     let bump = &[bump];
    //     let seeds = &[&[seed.as_slice(), bump][..]];
    //     let accounts = verifier_program_two::cpi::accounts::InitializeNullifiers {
    //         authority: authority.clone(),
    //         system_program: system_program.clone(),
    //         registered_verifier_pda: registered_verifier_pda.clone(),
    //     };
    //
    //     let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
    //     cpi_ctx = cpi_ctx.with_remaining_accounts(nullifier_pdas);
    //
    //     verifier_program_two::cpi::initialize_nullifiers(cpi_ctx, nullifiers)
    //     self.invoked_system_verifier = true;
    // }


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
