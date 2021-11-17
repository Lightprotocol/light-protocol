use wasm_bindgen::prelude::*;
pub mod arkworks_merkle_tree_poseidon_sponge;
pub mod common;
pub mod other;
pub mod poseidon_parameters;
pub mod prepare_inputs;
pub mod hard_coded_verifying_key_pvk_new_ciruit;
pub mod provingkey_groth16_09_10;

use crate::poseidon_parameters::get_sponge_params;
use ark_ff::bytes::{ToBytes,FromBytes};
use ark_ff::biginteger::{BigInteger256,BigInteger384};
use ark_ff::Fp256;
use ark_serialize::CanonicalDeserialize;
use ark_ec::PairingEngine;
use ark_ff::QuadExtField;
use ark_groth16::generator::generate_parameters;

#[wasm_bindgen]
pub fn hash_slice_u8( input : &[u8] ) -> Vec<u8> {

    let mut sponge = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_sponge_params());
    for i in 0..input.len() {
        sponge.absorb(&input[i]);
    }

    let hash : Vec<ark_ed_on_bls12_381::Fq> = sponge.squeeze_field_elements(1).clone();

    let mut tmp = vec![0u8;32];
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&hash[0], &mut tmp[..]);
    tmp


}

#[wasm_bindgen]
pub fn bytes_to_int(bytes: &[u8]) -> u64 {
    let mut bytes_arr : [u8; 8] = [0;8];

    for i in 0..bytes.len() {
        bytes_arr[i] = bytes[i];
    }

    let u = u64::from_le_bytes(bytes_arr);

    return u as u64;
}

// -------------------------> start



use crate::common::*;
use crate::other::*;
use crate::{Root, SimplePath};
use ark_crypto_primitives::crh::{TwoToOneCRH, TwoToOneCRHGadget, CRH};
use ark_crypto_primitives::crh::{
    injective_map::{PedersenCRHCompressor, TECompressor},
};
use ark_crypto_primitives::merkle_tree::constraints::PathVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use ark_r1cs_std::prelude::*;
use ark_r1cs_std::uint8::UInt8;
use ark_r1cs_std::bits::ToBitsGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::bits::ToBytesGadget;
use ark_r1cs_std::alloc::AllocVar;

use ark_std::vec::Vec;
use ark_ed_on_bls12_381::{constraints::EdwardsVar, EdwardsProjective as JubJub, Fq as Fr};
use ark_crypto_primitives::crh::pedersen;
// use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ec::{models::twisted_edwards_extended::GroupAffine, ProjectiveCurve};
// use ark_ff::Fp256;

use ark_groth16::{
    create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof, prepare_inputs,
    data_structures::{VerifyingKey, Proof },
};
use ark_bls12_381::*;
// use ark_ff::biginteger::BigInteger256;

use ark_sponge::poseidon::PoseidonSponge;
use ark_sponge::constraints::CryptographicSpongeVar;
use ark_sponge::CryptographicSponge;
use ark_sponge::Absorb;
use ark_ff::Fp384;
use ark_relations::r1cs::Namespace;
use crate::arkworks_merkle_tree_poseidon_sponge::*;
use ark_std::UniformRand;
use ark_crypto_primitives::Error;
use crate::arkworks_merkle_tree_poseidon_sponge::{MerkleTree};
// (You don't need to worry about what's going on in the next two type definitions,
// just know that these are types that you can use.)

/// The R1CS equivalent of the the Merkle tree root.
pub type RootVar = <TwoToOneHashGadget as TwoToOneCRHGadget<TwoToOneHash, ConstraintF>>::OutputVar;

////////////////////////////////////////////////////////////////////////////////
#[derive(Clone)]
pub struct MerkleTreeVerification {
    // These are constants that will be embedded into the circuit
    pub poseidon_sponge_params: ark_sponge::poseidon::PoseidonParameters::<ark_ed_on_bls12_381::Fq>,

    // These are the public inputs to the circuit.
    pub root_poseidon: ark_ed_on_bls12_381::Fq,
    pub nullifier_hash: ark_ed_on_bls12_381::Fq,
    pub commit_hash: ark_ed_on_bls12_381::Fq,

    // public inputs to preserve integrity of the tx while relaying
    pub tx_integrity_hash: ark_ed_on_bls12_381::Fq,
    pub amount: [u8;8],//u64
    pub to_address: [u8;32],
    pub relayer_address: [u8;32],
    pub relayer_refund: [u8;8],//u64
    // These are the private witnesses to the circuit.
    pub secret: [u8;32],
    pub nullifier: [u8;32],
    pub poseidon_path: Vec<(bool, ark_ed_on_bls12_381::Fq)>,

}

impl ConstraintSynthesizer<ConstraintF> for MerkleTreeVerification {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintF>,
    ) -> Result<(), SynthesisError> {
        // First, we allocate the public inputs
        let commit_hash_var =       RootVar::new_input(ark_relations::ns!(cs, "commit_var"), || Ok(&self.commit_hash))?;
        let nullifier_hash_var =    RootVar::new_input(ark_relations::ns!(cs, "nullifier_var"), || Ok(&self.nullifier_hash))?;
        let root_poseidon_var =     RootVar::new_input(ark_relations::ns!(cs, "root_poseidon_var"), || Ok(self.root_poseidon))?;
        let tx_integrity_hash_var = RootVar::new_input(ark_relations::ns!(cs, "tx_integrity_hash_var"), || Ok(self.tx_integrity_hash))?;

        // Then, we allocate the public parameters as constants:
        let sponge_params = &self.poseidon_sponge_params;

        // allocate private witness variables:

        let mut constraint_sponge_nullifier = ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ark_ed_on_bls12_381::Fq>::new(cs.clone(), &sponge_params);
        for byte in self.nullifier.iter() {
            let absorb_var = UInt8::new_witness(cs.clone(),|| Ok(*byte))?;
            constraint_sponge_nullifier.absorb(&absorb_var).unwrap();
        }
        let mut result_nullifier_var = constraint_sponge_nullifier.squeeze_field_elements(1).clone().unwrap();
        result_nullifier_var.enforce_equal(&[nullifier_hash_var]);


        //check for tx integrity

        let mut constraint_sponge_tx_integrity = ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ark_ed_on_bls12_381::Fq>::new(cs.clone(), &sponge_params);

        /*
        for byte in [self.amount, self.relayer_refund].concat().iter() {
            let absorb_var = UInt8::new_witness(cs.clone(),|| Ok(*byte))?;
            constraint_sponge_tx_integrity.absorb(&absorb_var).unwrap();
        }
        for byte in [ self.to_address, self.relayer_address].concat().iter() {
            let absorb_var = UInt8::new_witness(cs.clone(),|| Ok(*byte))?;
            constraint_sponge_tx_integrity.absorb(&absorb_var).unwrap();
        }*/
        let absorb_var0 = UInt8::new_witness_vec(cs.clone(),&[ self.amount, self.relayer_refund].concat())?;

        let absorb_var1 = UInt8::new_witness_vec(cs.clone(),&[ self.to_address, self.relayer_address].concat())?;
        constraint_sponge_tx_integrity.absorb(&absorb_var0).unwrap();
        constraint_sponge_tx_integrity.absorb(&absorb_var1).unwrap();
        let mut tx_integrity_hash = constraint_sponge_tx_integrity.squeeze_field_elements(1).clone().unwrap();
        tx_integrity_hash.enforce_equal(&[tx_integrity_hash_var]);

        // allocate path as private witness variables:

        let mut constraint_sponge_leaf = ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ark_ed_on_bls12_381::Fq>::new(cs.clone(), &sponge_params);

        for byte in [self.nullifier, self.secret].concat().iter() {
            let absorb_var = UInt8::new_witness(cs.clone(),|| Ok(*byte))?;
            constraint_sponge_leaf.absorb(&absorb_var).unwrap();
        }

        let mut curr_hash = constraint_sponge_leaf.squeeze_field_elements(1).clone().unwrap();

        //check that the commitment corresponds to the nullifier and secret
        //curr_hash.enforce_equal(&[commit_hash_var]);

        let mut poseidon_path = Vec::new();
        for (is_right_child, tmp_path_var) in self.poseidon_path.iter() {
            let mut var : (Boolean::<Fr>, RootVar) = (Boolean::<Fr>::FALSE, ark_r1cs_std::fields::fp::FpVar::Constant(Fp256::new(BigInteger256([0, 0, 0, 0]))));

            var.1 = RootVar::new_witness(cs.clone(), || Ok(tmp_path_var)).unwrap();

            if *is_right_child == true {
                var.0 = Boolean::<Fr>::TRUE;
            }
            poseidon_path.push(var);
        }

        // verify merkle_proof

        for (cond, element) in poseidon_path.iter() {

            let mut constraint_sponge_loop = ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ark_ed_on_bls12_381::Fq>::new(cs.clone(), &sponge_params);

            let mut left_hash = cond.select(element, &curr_hash[0]).unwrap().clone();
            let mut right_hash = cond.select(&curr_hash[0], element).unwrap().clone();

            constraint_sponge_loop.absorb(&left_hash).unwrap();
            constraint_sponge_loop.absorb(&right_hash).unwrap();

            //squeeze_bytes for the actual hash
            curr_hash = constraint_sponge_loop.squeeze_field_elements(1).clone().unwrap();
        }

        curr_hash.enforce_equal(&[root_poseidon_var])?;

        Ok(())
    }
}



fn parse_quad_to_bytes_TEST(q : ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>, account: &mut [u8], range: [usize;2]){

    let mut iter = range[0];

        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 48;
                if z == 0 {
                    //println!("Parsing {:?}", c.c0);
                    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&q.c0, &mut account[tmp..iter]);
                } else if z == 1 {
                    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&q.c1, &mut account[tmp..iter]);
                }
        }
}

fn parse_fp384_to_bytes_TEST(fp384 : ark_ff::Fp384<ark_bls12_381::FqParameters>, account: &mut [u8], range: [usize;2]){

    let start = range[0];
    let end = range[1];
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&fp384, &mut account[start..end]);
}

pub fn parse_x_group_affine_from_bytes(account: &Vec<u8>) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters> {

    let x = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bls12_381::g1::Parameters>::new(
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&account[0..48]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&account[48..96]).unwrap(),
        false
    );
    x
}

pub fn parse_x_group_affine_to_bytes(x : ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>, account: &mut Vec<u8>){
                    //println!("Parsing {:?}", c.c0);
    // parse_fp384_to_bytes(x.x, acc1, range1);
    // parse_fp384_to_bytes(x.y, acc2, range2);
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&x.x, &mut account[0..48]);
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&x.y, &mut account[48..96]);

}


pub fn parse_proof_b_to_bytes(proof: &ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>, range: &mut Vec<u8>) {
    let mut tmp0 = vec![0u8;96];
    let mut tmp1 = vec![0u8;96];
    parse_quad_to_bytes_TEST(proof.x,&mut tmp0, [0,96]);
    parse_quad_to_bytes_TEST(proof.y,&mut tmp1, [0,96]);
    *range = [tmp0, tmp1].concat();

}








#[test] // Adapt fixed commitment and leaf. Run wasm offchain for printing
fn merkle_tree_constraints_with_groth16_dynamic_merkleproof() {
    //     // test --->
    let commitment_slice = [21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21];
    let leaves_slice =  [151, 85, 62, 182, 26, 238, 149, 115, 117, 89, 25, 56, 176, 33, 124, 54, 229, 133, 85, 3, 220, 179, 228, 88, 14, 137, 72, 68, 230, 230, 25, 74];
    let recipient_slice = [0;32];
    let relayer = [0;32];
    //     // <--- test end

    let f_p_coeffs = create_f_p_coeffs(&commitment_slice[..], &leaves_slice[..], &recipient_slice[..], &recipient_slice[..]);
    // let f_bytes = f_p_coeffs[0..576].to_vec();
    // let p_bytes = f_p_coeffs[1440..1728].to_vec();
    // let coeffs_bytes = f_p_coeffs[1728..60480].to_vec();
    // let inputs_bytes = f_p_coeffs[61000..61128].to_vec();
    //println!("inputs_bytes: {:?} => {:?}",inputs_bytes.len(), inputs_bytes);
}

#[wasm_bindgen]
pub fn create_f_p_coeffs(commitment_slice: &[u8], leaves_slice: &[u8], recipient_slice: &[u8], relayer_slice: &[u8], length_leave_slice: usize) -> Vec<u8> { // usize {
    extern crate console_error_panic_hook;//
    console_error_panic_hook::set_once();



    use ark_relations::r1cs::{ConstraintLayer, ConstraintSystem, TracingMode};
    use tracing_subscriber::layer::SubscriberExt;
    use std::convert::{TryInto};

    assert_eq!(leaves_slice.len(), length_leave_slice, "leave slice length differs from expected length");

    let mut commit : [u8; 64] = [0;64];

    for i in 0..commitment_slice.len() {
        commit[i] = commitment_slice[i];
    }
    assert_eq!(commitment_slice.len(), 64);


    //inputs FIXED. CHANGE THIS
    let amount =   u64::to_le_bytes(1000000000u64);//1000000000u64,[0u8;8];//
    assert_eq!(amount.len(), 8, "invalid amount");

    let mut to_address =            [0u8;32];
    let mut relayer_address =       [0u8;32];
    let relayer_refund =            [0u8;8];//1000000u64,

    for i in 0..recipient_slice.len() {
        to_address[i] = recipient_slice[i];
    }
    assert_eq!(recipient_slice.len(), 32, "invalid address");

    for i in 0..relayer_slice.len() {
        relayer_address[i] = relayer_slice[i];
    }
    assert_eq!(recipient_slice.len(), 32, "invalid address");

    //no input
    let tree_height: u32 =              11;

    // Let's set up an RNG for use within tests. Note that this is *not* safe
    // for any production use.
    let mut rng = ark_std::test_rng();
    // let new_nullifier = Fp256::<ark_ed_on_bls12_381::FqParameters>::rand(&mut rng);
    // let new_secret =    Fp256::<ark_ed_on_bls12_381::FqParameters>::rand(&mut rng);

    // let mut nullifier =                 [0u8;32];
    // let mut secret =                    [0u8;32];
    // <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&new_nullifier, &mut nullifier[..]);
    // <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&new_secret, &mut secret[..]);
    // let commit = [nullifier.clone(),secret.clone() ].concat();
    let nullifier : [u8; 32] = commit[0..32].try_into().unwrap();
    // let nullifier_1 : [u8; 16] = commit[0..16].try_into().unwrap();
    // let nullifier_2  : [u8; 16] = commit[16..32].try_into().unwrap();
    let secret : [u8;32] = commit[32..64].try_into().unwrap();



    let mut sponge_commit = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_sponge_params());

    for byte in commit.iter() {
        sponge_commit.absorb(byte);
    }

    let mut commit_hash_fq = sponge_commit.squeeze_field_elements::<ark_ed_on_bls12_381::Fq>(1)[0].clone();
    let mut commit_hash_bytes = vec![0u8;32];

    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&commit_hash_fq, &mut commit_hash_bytes[..]);

    //zero_bytes poseidon hash of [1u8; 32]
    let zero_bytes = vec![19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 98];

    let mut filled_leaves : Vec<Vec<u8>> = vec![]; // right len? 32x
    for (i, leaf) in leaves_slice.chunks(32).enumerate() {
        filled_leaves.push(leaf.to_vec());
    }

    // assert_eq!(filled_leaves.len(),1);

    let fll = &filled_leaves.len();

    let leaves: Vec<Vec<u8>> = [filled_leaves.clone(), vec![zero_bytes.clone(); 2_usize.pow(tree_height)-fll]].concat();
    let zero_leaves: Vec<Vec<u8>> = vec![zero_bytes.clone(); 2_usize.pow(tree_height.try_into().unwrap())];
    // assert_eq!(leaves, vec![vec![0;32]]);
    let mut tree = MerkleTree::new(
        &zero_leaves,
    ).unwrap();
    let mut tree_ref = MerkleTree::new(
        &leaves,
    ).unwrap();
    for (i, elem) in filled_leaves.iter().enumerate() {
        tree.update(i, elem);
    }
    assert_eq!(tree_ref.root(), tree.root());

    //// keep bc rng:
    //generating and adding random leaves
    // let mut all_new_leaves: Vec<Vec<u8>> = Vec::new();
    // for i in 0..6 {
    //     let new_leaf_has = Fp256::<ark_ed_on_bls12_381::FqParameters>::rand(&mut rng);
    //     // let mut new_leaf_hash_bytes = vec![0u8;32];
    //     // <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&new_leaf_hash, &mut new_leaf_hash_bytes[..]);
    //     // all_new_leaves.push(new_leaf_hash_bytes.clone());
    //     //println!("{:?}", new_leaf_bytes);
    //     // tree.update(i, &new_leaf_hash_bytes);
    // }


    // find leaf (index_of(commit_hash_bytes))
    // assert_eq!(commit_hash_bytes, [151, 85, 62, 182, 26, 238, 149, 115, 117, 89, 25, 56, 176, 33, 124, 54, 229, 133, 85, 3, 220, 179, 228, 88, 14, 137, 72, 68, 230, 230, 25, 74]);

    let current_leaf_index = leaves.iter().position(|r| r == &commit_hash_bytes).unwrap(); // should handle "none" or duplicate! @jorrit
    // gen proof for current leaf
    let proof = tree.generate_proof(current_leaf_index).unwrap();

    // HC:
    //adding the commit leaf
    // tree.update(3, &commit_hash_bytes);
    // let proof = tree.generate_proof(3).unwrap();

    let proof_circuit_friendly = proof.convert_path_to_circuit_ready_merkle_proof(&tree.root(), &commit_hash_bytes).unwrap();

    let root_poseidon_test_fq = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&tree.root()[..]).unwrap();

    // First, let's sample the public parameters for the hash functions:
    // let leaf_crh_params = <LeafHash as CRH>::setup(&mut rng).unwrap();
    // let two_to_one_crh_params = <TwoToOneHash as TwoToOneCRH>::setup(&mut rng).unwrap();
    // let nullifier_crh_params = <NullifierHash as CRH>::setup(&mut rng).unwrap();


    let mut sponge_nullifier_hash = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_sponge_params());

    for byte in nullifier.iter() {
        sponge_nullifier_hash.absorb(byte);
    }

    let mut nullifier_fq = sponge_nullifier_hash.squeeze_field_elements::<ark_ed_on_bls12_381::Fq>(1)[0].clone();

    let mut sponge_tx_integrity = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_sponge_params());
    /*
    for byte in [ amount, relayer_refund].concat().iter() {
        sponge_tx_integrity.absorb(byte);
    }
    for byte in [ to_address, relayer_address].concat().iter() {
        sponge_tx_integrity.absorb(byte);
    }*/
    sponge_tx_integrity.absorb(&[amount, relayer_refund].concat());
    sponge_tx_integrity.absorb(&[ to_address, relayer_address].concat());

    let mut tx_integrity_hash = sponge_tx_integrity.squeeze_field_elements::<ark_ed_on_bls12_381::Fq>(1)[0].clone();


    let circuit = MerkleTreeVerification {
        // constants
        poseidon_sponge_params: get_sponge_params(),

        // public inputs
        nullifier_hash: nullifier_fq.clone(),
        //submitting the nullifier twice to not reveal the commit hash as public input
        commit_hash: nullifier_fq.clone(),
        tx_integrity_hash: tx_integrity_hash,
        
        // witnesses
        nullifier: nullifier.clone(),
        secret: secret,

        amount: amount,
        to_address: to_address,
        relayer_address: relayer_address,
        relayer_refund: relayer_refund,

        poseidon_path: proof_circuit_friendly,
        root_poseidon: root_poseidon_test_fq.clone(),
    };


    //test circuit with groth16
    /*
    let alpha = Fp256::<ark_ed_on_bls12_381::FqParameters>::new(BigInteger256::new([8857200643501407085, 15507506095515245052, 11161094398839230275, 1592882588375294331]));
    let beta = Fp256::<ark_ed_on_bls12_381::FqParameters>::new(BigInteger256::new([17685639249803389165, 271687294819712898, 3557815544314210444, 4551052814358914550]));
    let gamma = Fp256::<ark_ed_on_bls12_381::FqParameters>::new(BigInteger256::new([18058663966098534984, 15323108146379374369, 12697459184661139359, 4510570883895358373]));
    let delta = Fp256::<ark_ed_on_bls12_381::FqParameters>::new(BigInteger256::new([17093525261188170780, 6872081389052970578, 15137622433394305408, 5950973870047597667]));

    let g1_generator = ark_bls12_381::G1Projective {
        x: Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([17261076967276103974, 13426872676337726704, 3099676955916542750, 4062176631318868857, 15741656282444642897, 585690705043873621])),
        y: Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([895849711937902421, 4332842718009952729, 6762086343362947689, 9823784903618787240, 15214529144392295093, 110338211847558044])),
        z: Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13650937687617930235, 7122330172803586504, 12890179670962158525, 6546347356043524737, 12483318844555674343, 941493096456638488]))
    };

    let g2_generator = ark_bls12_381::G2Projective {
        x: QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([6812722269699009604, 12430709018539610695, 962822161471731376, 10174931906307092665, 8945851849017904849, 189321343485194311])),
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([11248077903624011735, 588507649257371870, 13626810847873907792, 2816942618118438387, 14167282175346239650, 1432768687758932514]))
        ),
        y: QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([3377818368187236006, 17397030679181040434, 15737439011570226755, 7072125027014389104, 10908059603942710624, 305371239398691796])),
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16056780986808206759, 14838602075966953505, 991823436821252962, 14738757498656907396, 9785985288891674649, 1077374764851608114]))
        ),
        z: QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([11710140347467210042, 8627559171206053430, 15183344154787726165, 3092280784171258261, 5148734300812213420, 1472035381055704187])),
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([10479360955019578456, 17226050556762216001, 15328724245386671912, 6162719489888917551, 15996812682616310766, 401747953419101486]))
        )
    }; */

    //let params_groth16 = generate_parameters::<ark_bls12_381::Bls12_381, _, _>(circuit.clone(), alpha, beta, gamma, delta, g1_generator, g2_generator,   &mut rng).unwrap();

    let params_groth16 = generate_random_parameters::<ark_bls12_381::Bls12_381, _, _>(circuit.clone(), &mut rng).unwrap();
    //let params_groth16_2 = generate_random_parameters::<ark_bls12_381::Bls12_381, _, _>(circuit.clone(), &mut rng).unwrap();
    //assert_eq!(params_groth16.vk, params_groth16_2.vk);
    //let params_groth16 = <ark_groth16::ProvingKey::<ark_ec::bls12::Bls12::<ark_bls12_381::Parameters>> as CanonicalDeserialize>::deserialize(&*provingkey_groth16_09_10::groth16_params_bytes.to_vec()).unwrap();

    let pvk = prepare_verifying_key(&params_groth16.vk);


    // println!("pvk GAMMA_ABC_G1 OLD: {:?}", pvk.vk.gamma_abc_g1);
    // println!("pvk alpha_g1_beta_g2 OLD: {:?}", pvk.alpha_g1_beta_g2);
    //println!("pvkhc: {:?} ", pvk);


    let mut nullifier_bytes = vec![0;32];
    let mut root_poseidon_test_bytes = vec![0;32];
    let mut tx_integrity_bytes = vec![0;32];

    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&nullifier_fq, &mut nullifier_bytes[..]);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&root_poseidon_test_fq, &mut root_poseidon_test_bytes[..]);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&tx_integrity_hash, &mut tx_integrity_bytes[..]);


    let inputs = [
        nullifier_fq,
        nullifier_fq,
        root_poseidon_test_fq,
        tx_integrity_hash
        ];

    // let mut inputs_bytes = [
    //     commit_hash_bytes, // 32
    //     nullifier_bytes, // 32
    //     root_poseidon_test_bytes, // 32
    //     tx_integrity_bytes // 32
    // ].concat(); // 128
    //assert_eq!((inputs_bytes.len()),128);

    let mut inputs_bytes = [
        nullifier_bytes.clone(),//commit_hash_bytes, // 32
        nullifier_bytes, // 32
        root_poseidon_test_bytes, // 32
        //put to zero since are generated on chain
        tx_integrity_bytes, // 32
        amount.to_vec(),  // 8
        relayer_refund.to_vec(),  // 8
        to_address.to_vec(), // 32
        relayer_address.to_vec(), // 32
    ].concat(); // 210

    assert_eq!((inputs_bytes.len()),208);





    use crate::prepare_inputs::*;

    use ark_ec::{ProjectiveCurve};
    use crate::   hard_coded_verifying_key_pvk_new_ciruit::*;

    // let prepared_inputs = prepare_inputs(&pvk, &inputs).unwrap(); // LIBRARY
    let i =   hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_0(); // Assert with new gamma_abc_g1??
    //assert_eq!(i.len(),inputs.len()+1);
    println!("{:?}",pvk);
    assert_eq!(pvk.vk.gamma_abc_g1[0],i);

    // CURRENT: same pvk as rest/coeffs/f etc
    let mut r = vec![  hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_0(),
          hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_1(),
          hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_2(),
          hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_3(),
          hard_coded_verifying_key_pvk_new_ciruit::get_gamma_abc_g1_4()
    ];

    // TEST
    let prepared_inputs_cus = prepare_inputs_custom(r, &inputs).unwrap().into_affine(); // CUSTOM
    // let prepared_inputs_lib = prepare_inputs(&pvk, &inputs).unwrap(); // library function

    // assert_eq!(prepared_inputs_cus, prepared_inputs_lib);
    let mut prepared_inputs_cus_bytes : Vec<u8> = vec![0;96];

    parse_x_group_affine_to_bytes(prepared_inputs_cus, &mut prepared_inputs_cus_bytes);
    println!("prepared_inputs_cus with fixed input, old pvk: {:?}", prepared_inputs_cus_bytes);

    let proof = create_random_proof(circuit, &params_groth16,&mut rng).unwrap(); // in client //

    // assert!(verify_proof(&pvk, &proof,
    //     &[
    //         nullifier_fq,
    //         nullifier_fq,
    //         root_poseidon_test_fq,
    //         tx_integrity_hash,
    //     ]).unwrap());

    // ABOVE:: OK

    // custom part: -------------------------------------->> this part onchain...
    // let mut account = [0u8; 61000]; // repl
    // let mut i = vec![];
    // let mut pairs = vec![];

    // println!("prp inputs RAW THEY:  {:?}",prepared_inputs);
    // assert_eq!(prepared_inputs, ark_ec::short_weierstrass_jacobian::GroupProjective::new(
    //     Fp384::new((BigInteger384([3245666635304568760, 6617930767841372967, 767240267611048163, 7403258176323836267, 7058241202448776382, 1707621016520071570]))),
    //     Fp384::new((BigInteger384([18138246523367676183, 5912929941103254729, 6682139321499362931, 15735890910157947839, 6068581380398957220, 806822383536876563]))),
    //     Fp384::new((BigInteger384([9895163454178055201, 1342772875698776517, 6664000958310233542, 6069591321821364346, 2167940937690056919, 497050691636071961])))
    // )
    // );

    // let mut prepared_inputs_affine = ark_ec::short_weierstrass_jacobian::GroupAffine::from(prepared_inputs); // 177k => store affine
    // let xx : ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters>= ark_ec::bls12::g1::G1Prepared::from((prepared_inputs_cus).into_affine());
    // println!("xx {:?}", prepared_inputs_cus.into_affine());
    // // The projective point X, Y, Z is represented in the affine
    // coordinates as X/Z^2, Y/Z^3.
    // impl<P: Parameters> From<GroupProjective<P>> for GroupAffine<P> {
    //     #[inline]
    //
    // fn from(p: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters>) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters> {
    //     // if num_traits::Zero::is_zero(&p) { //  p.is_zero()
    //     //    num_traits::Zero::zero()
    //     // } else if p.z.is_one() {
    //     //     // If Z is one, the point is already normalized.
    //     //     ark_ec::short_weierstrass_jacobian::GroupAffine::new(p.x, p.y, false)
    //     // } else {
    //         // Z is nonzero, so it must have an inverse in a field.
    //         let zinv = ark_ff::Field::inverse(&p.z).unwrap();// p.z.inverse().unwrap(); // inverse(p.z);
    //         let zinv_squared = ark_ff::Field::square(&zinv); // zinv.square();
    //
    //         // X/Z^2
    //         let x = p.x * &zinv_squared;
    //
    //         // Y/Z^3
    //         let y = p.y * &(zinv_squared * &zinv);
    //
    //         ark_ec::short_weierstrass_jacobian::GroupAffine::new(x, y, false)
    //     }
    // }
    // }




    // println!("CHECK FROM IMPL");
    // assert_eq!(prepared_inputs_affine, from(prepared_inputs));
    // assert_eq!(prepared_inputs.into_affine(), from(prepared_inputs));


    // Prepare proof for return
    // println!("proof.a: P1 {:?}",proof.a);

    // println!("proof.b: {:?}",proof.b);
    // println!("proof.c: P3 {:?}",proof.c);

    let mut proof_a_bytes : Vec<u8> = vec![0;96];
    let mut proof_b_bytes : Vec<u8> = vec![0;192];
    let mut proof_c_bytes : Vec<u8> = vec![0;96];

    parse_x_group_affine_to_bytes(proof.a, &mut proof_a_bytes);
    parse_proof_b_to_bytes(&proof.b, &mut proof_b_bytes);
    parse_x_group_affine_to_bytes(proof.c, &mut proof_c_bytes);




    // let one : (ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters>, ark_ec::bls12::G2Prepared<ark_bls12_381::Parameters>) = (ark_ec::bls12::g1::G1Prepared::from(proof.a), ark_ec::bls12::g2::G2Prepared::from(proof.b)); // proof.a "~"
    // let two : (ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters>, ark_ec::bls12::G2Prepared<ark_bls12_381::Parameters>) = (ark_ec::bls12::g1::G1Prepared::from((prepared_inputs_cus).into_affine()), pvk.gamma_g2_neg_pc.clone()); // prep i onchain => convert into affine onchain
    // let three : (ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters>, ark_ec::bls12::G2Prepared<ark_bls12_381::Parameters>) = (ark_ec::bls12::g1::G1Prepared::from(proof.c), pvk.delta_g2_neg_pc.clone()); // proof.c in client in bytes, pass in
    // create pairs:

    // println!("proof.a AFT:  {:?}",one.0);
    // println!("prp inputs (affine) AFT:  {:?}",two.0);
    // println!("proof.c AFT:  {:?}",three.0);

    // i.push(one);
    // i.push(two);
    // i.push(three);
    // for (p, q) in i.iter() {
    //     if !p.is_zero() && !q.is_zero() {
    //         pairs.push((p, q.ell_coeffs.iter()));
    //     }
    // }
    //
    // // parse p
    // let mut r: usize = 1728;
    // let mut w: usize = 1440;
    // for (p,ref mut q) in &mut pairs {
    //
    //     parse_fp384_to_bytes_TEST(p.0.x, &mut account[..], [w, w+48]);
    //     parse_fp384_to_bytes_TEST(p.0.y, &mut account[..], [w+48, w+96]);
    //     w += 96;
    //
    // };
    //
    //
    // // fill coeffs
    // for x in 0..68 {
    //     for (p, ref mut coeffs) in &mut pairs {
    //         let c = coeffs.next().unwrap();
    //
    //         parse_quad_to_bytes_TEST(c.0, &mut account[..], [r, r+96]);
    //         parse_quad_to_bytes_TEST(c.1, &mut account[..], [r+96, r+96+96]);
    //         parse_quad_to_bytes_TEST(c.2, &mut account[..], [r+96+96, r+288]);
    //         r += 288;
    //     }
    // }
    //
    //
    // let mut f = vec![0;576]; // create f manually
    // f[0] = 1;
    //
    // for i in 0..576{
    //     account[i] = f[i];
    // };

    // let coeffsbytes :Vec<u8>= account[1728..60480].to_vec();
    // let path = "get_hardcoded_coeffs.rs";
    // let mut input = File::create(path).unwrap();
    // // return proof.a, proof.b, proof.c in bytes
    // // write!(input, "{:?}" ,"fn get_hardcoded_coeffs() -> Vec<u8> {");
    // for i in coeffsbytes.chunks(288) {
    //     write!(input, " {:?}\n",i[0..96].to_vec());

    // }
    // write!(input,"{:?}", "}");
    // println!("last coeff: {:?}", account[60192..60480].to_vec());
    // let mut account = [0u8; 61000];
    // account[0] = 1;

    return [inputs_bytes, proof_a_bytes, proof_b_bytes, proof_c_bytes, prepared_inputs_cus_bytes].concat();


}



/*
pub fn get_gamma_abc_g1()->Vec<ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>> {
    // [
    //     GroupAffine {
    //         x: Fp384(BigInteger384([563294370412064037, 1715290422127468535, 18247809608841574549, 5177330868439990692, 3228070465427577421, 605946617072756854])),
    //         y: Fp384(BigInteger384([13920804639078038700, 2103078534984712670, 16765348787309578864, 11160079268490454535, 10344503905246139160, 1063850063302893615])),
    //         infinity: false
    //     },
    //     GroupAffine {
    //         x: Fp384(BigInteger384([10592294243429201356, 3052936832954966150, 15144811437358780756, 10401393416841291922, 7245740055162104126, 857591574061328392])),
    //         y: Fp384(BigInteger384([15315649812109931145, 9318379341740421808, 17829281982239064000, 15562114518178306835, 6338289258800639000, 172047970480976574])),
    //         infinity: false
    //     },
    //     GroupAffine {
    //         x: Fp384(BigInteger384([11638148556121603959, 12131210038757433928, 13406469831474887750, 14698032840405646962, 6014964175194711838, 298336780121273518])),
    //         y: Fp384(BigInteger384([13547309967451232990, 9535344545504142215, 16712208762832503113, 501922646055373775, 12300693734737576732, 601137693745339038])),
    //         infinity: false
    //     },
    //     GroupAffine {
    //         x: Fp384(BigInteger384([17439929153102104375, 392828205012811782, 8716778311869034164, 7637242120340872095, 3737847449637934698, 1493818450196326065])),
    //         y: Fp384(BigInteger384([16341597955080357699, 7396390417520231214, 13645443054631526344, 17255923328787893665, 17894011013525857439, 281529589702258940])),
    //         infinity: false
    //     },
    //     GroupAffine {
    //         x: Fp384(BigInteger384([13449289244975091591, 2896178142460479191, 10309383525695020272, 17997647925870942918, 17605722657634554002, 651418187308111208])),
    //         y: Fp384(BigInteger384([16026926828496853368, 16898327419026000456, 14876803927141650836, 1932555005519829970, 1531385234874156159, 1743213542576363609])),
    //         infinity: false }
    //     ];

    let gamma_abc_g1 = vec![
        ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([563294370412064037, 1715290422127468535, 18247809608841574549, 5177330868439990692, 3228070465427577421, 605946617072756854])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13920804639078038700, 2103078534984712670, 16765348787309578864, 11160079268490454535, 10344503905246139160, 1063850063302893615])),
        false

        ),
        ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([10592294243429201356, 3052936832954966150, 15144811437358780756, 10401393416841291922, 7245740055162104126, 857591574061328392])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15315649812109931145, 9318379341740421808, 17829281982239064000, 15562114518178306835, 6338289258800639000, 172047970480976574])),
        false
        ),
        ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([11638148556121603959, 12131210038757433928, 13406469831474887750, 14698032840405646962, 6014964175194711838, 298336780121273518])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13547309967451232990, 9535344545504142215, 16712208762832503113, 501922646055373775, 12300693734737576732, 601137693745339038])),
        false
        ),
        ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([17439929153102104375, 392828205012811782, 8716778311869034164, 7637242120340872095, 3737847449637934698, 1493818450196326065])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16341597955080357699, 7396390417520231214, 13645443054631526344, 17255923328787893665, 17894011013525857439, 281529589702258940])),
        false
        ),
        ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13449289244975091591, 2896178142460479191, 10309383525695020272, 17997647925870942918, 17605722657634554002, 651418187308111208])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16026926828496853368, 16898327419026000456, 14876803927141650836, 1932555005519829970, 1531385234874156159, 1743213542576363609])),
        false
        )
    ];



    // let gamma_abc_g1 = vec![
    // ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15285345879763721750,
    // 10383866863359034545,
    // 17494994028992716901,
    // 15565996683107878779,
    // 6488797219036609866,
    // 1640426329608997925])),
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15151366753314496353,
    // 7504811564346646273,
    // 13352800842688998409,
    // 8986566058725889653,
    // 15845199749310313042,
    // 1197749971501574160])),
    // false

    // ),
    // ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([574563756247280143,
    // 14916735687860084499,
    // 1828400743912870411,
    // 2480601975139058119,
    // 12223832130151483874,
    // 1360767051752910253])),
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([586954975744763626,
    // 12521573179978403737,
    // 17858078226367134162,
    // 3943502625556661835,
    // 16538231210768394816,
    // 456360699258238554])),
    // false
    // ),
    // ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([10352755207113475127,
    // 6222341395178274059,
    // 5948900506467271271,
    // 17759700220659532425,
    // 5358708322027770789,
    // 1762224544731447190])),
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([18265908051805499498,
    // 13474523804763333230,
    // 10284323266436795545,
    // 15580594481682407886,
    // 4715788046815015359,
    // 910777732194348786])),
    // false
    // ),
    // ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([7315437755233868580,
    // 4890058732863313335,
    // 15280545622872287188,
    // 8124627673087893789,
    // 15562468567288304893,
    // 111522076735502687])),
    // Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([9504792667513693671,
    // 6692328631063080804,
    // 12791078537218105518,
    // 1650413682266360461,
    // 12773598768837931740,
    // 245029159758338879])),
    // false
    // )];
    return gamma_abc_g1
}
*/
