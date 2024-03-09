use std::{collections::HashMap, io::Cursor, time::Instant};

use ark_bn254::{Bn254, Fr};
use ark_circom::{circom::Inputs, read_zkey, CircomReduction, WitnessCalculator};
use ark_crypto_primitives::snark::SNARK;
use ark_groth16::{Groth16, Proof, ProvingKey};
use ark_relations::r1cs::ConstraintMatrices;
use ark_std::rand::thread_rng;
use log::info;

use crate::{errors::CircuitsError, inclusion::merkle_tree_info::MerkleTreeInfo};

pub type ArkProof = (Proof<Bn254>, Vec<Fr>);
pub type ArkProvingKey = (ProvingKey<Bn254>, ConstraintMatrices<Fr>);

pub fn verify(pk: &ArkProvingKey, proof: &ArkProof) -> Result<bool, CircuitsError> {
    let pvk = Groth16::<Bn254>::process_vk(&pk.0.vk)?;
    let inputs = proof.1[1..pk.1.num_instance_variables].to_vec();
    let verified = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &inputs, &proof.0)?;
    Ok(verified)
}

pub fn read_zk(
    mt_height: &MerkleTreeInfo,
    nm_utxos: usize,
    zkey: &[u8],
) -> Result<ArkProvingKey, CircuitsError> {
    let start = Instant::now();
    let mut cursor = Cursor::new(zkey);
    let (params, matrices) = read_zkey(&mut cursor)?;
    let duration = start.elapsed();
    info!(
        "mt_{}_{} zkey loaded : {:?}",
        mt_height.height(),
        nm_utxos,
        duration
    );
    let num_inputs = matrices.num_instance_variables;
    let num_constraints = matrices.num_constraints;
    info!(
        "mt_{}_{} num_inputs={}",
        mt_height.height(),
        nm_utxos,
        num_inputs
    );
    info!(
        "mt_{}_{} num_constraints={}",
        mt_height.height(),
        nm_utxos,
        num_constraints
    );
    Ok((params, matrices))
}

pub fn prove(
    merkle_tree_height: u8,
    nr_utxos: usize,
    inputs: HashMap<String, Inputs>,
    pk: &ArkProvingKey,
    wasm_path: &str,
) -> Result<ArkProof, CircuitsError> {
    info!("generating witness...");
    let mut start = Instant::now();
    let mut wtns = WitnessCalculator::new(wasm_path)?;
    let full_assignment = wtns.calculate_witness_element::<Bn254, _>(inputs, false)?;
    let mut duration = start.elapsed();
    info!(
        "mt_{}_{} witness generated: {:?}",
        merkle_tree_height, nr_utxos, duration
    );

    info!("creating proof...");
    start = Instant::now();
    let mut rng = thread_rng();
    use ark_std::UniformRand;
    let rng = &mut rng;

    let r = Fr::rand(rng);
    let s = Fr::rand(rng);

    let proof = Groth16::<Bn254, CircomReduction>::create_proof_with_reduction_and_matrices(
        &pk.0,
        r,
        s,
        &pk.1,
        pk.1.num_instance_variables,
        pk.1.num_constraints,
        full_assignment.as_slice(),
    )?;
    duration = start.elapsed();
    info!(
        "mt_{}_{} proof created: {:?}",
        merkle_tree_height, nr_utxos, duration
    );

    Ok((proof, full_assignment))
}
