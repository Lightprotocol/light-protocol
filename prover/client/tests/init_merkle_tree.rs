#![allow(dead_code)]
use std::sync::Mutex;

use ark_std::Zero;
use light_hasher::{
    hash_chain::{create_hash_chain_from_array, create_two_inputs_hash_chain},
    Hasher, Poseidon,
};
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    errors::ProverClientError,
    helpers::{big_int_to_string, bigint_to_u8_32},
    proof_types::{
        circuit_type::CircuitType,
        combined::{
            v1::CombinedJsonStruct as CombinedJsonStructV1,
            v2::CombinedJsonStruct as CombinedJsonStructV2,
        },
        inclusion::{
            v1::BatchInclusionJsonStruct as BatchInclusionJsonStructV1,
            v2::{BatchInclusionJsonStruct, InclusionJsonStruct, InclusionMerkleProofInputs},
        },
        merkle_tree_info::MerkleTreeInfo,
        non_inclusion::{
            v1::{
                BatchNonInclusionJsonStruct as BatchNonInclusionJsonStructV1,
                LegacyNonInclusionJsonStruct,
            },
            v2::{
                BatchNonInclusionJsonStruct as BatchNonInclusionJsonStructV2,
                NonInclusionJsonStruct as NonInclusionJsonStructV2, NonInclusionMerkleProofInputs,
            },
        },
    },
};
use num_bigint::{BigInt, Sign, ToBigUint};
use num_traits::cast::ToPrimitive;
use once_cell::{self, sync::Lazy};
use tracing::info;

pub static MT_PROOF_INPUTS_26: Lazy<Mutex<InclusionMerkleProofInputs>> =
    Lazy::new(|| Mutex::new(internal_inclusion_merkle_tree_inputs(26)));

pub static MT_PROOF_INPUTS_32: Lazy<Mutex<InclusionMerkleProofInputs>> =
    Lazy::new(|| Mutex::new(internal_inclusion_merkle_tree_inputs(32)));

pub fn inclusion_merkle_tree_inputs(mt_height: MerkleTreeInfo) -> InclusionMerkleProofInputs {
    match mt_height {
        MerkleTreeInfo::H26 => (*MT_PROOF_INPUTS_26.lock().unwrap()).clone(),
        MerkleTreeInfo::H32 => (*MT_PROOF_INPUTS_32.lock().unwrap()).clone(),
    }
}

fn internal_inclusion_merkle_tree_inputs(height: usize) -> InclusionMerkleProofInputs {
    const CANOPY: usize = 0;

    info!("initializing merkle tree");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // usafe of `once_cell::sync::Lazy` as a static variable.
    let mut merkle_tree = MerkleTree::<Poseidon>::new(height, CANOPY);
    info!("merkle tree initialized");

    info!("updating merkle tree");
    let mut bn_1: [u8; 32] = [0; 32];
    bn_1[31] = 1;
    let leaf: [u8; 32] = Poseidon::hash(&bn_1).unwrap();
    merkle_tree.append(&leaf).unwrap();
    let root1 = &merkle_tree.roots[1];
    info!("merkle tree updated");

    info!("getting proof of leaf");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // unsafe of `once_cell::sync::Lazy` as a static variable.
    let path_elements = merkle_tree
        .get_proof_of_leaf(0, true)
        .unwrap()
        .iter()
        .map(|el| BigInt::from_bytes_be(Sign::Plus, el))
        .collect::<Vec<_>>();
    info!("proof of leaf calculated");
    let leaf_bn = BigInt::from_bytes_be(Sign::Plus, &leaf);
    let root_bn = BigInt::from_bytes_be(Sign::Plus, root1);
    let path_index = BigInt::zero();

    InclusionMerkleProofInputs {
        root: root_bn,
        leaf: leaf_bn,
        path_index,
        path_elements,
    }
}

pub fn non_inclusion_merkle_tree_inputs(height: usize) -> NonInclusionMerkleProofInputs {
    const CANOPY: usize = 0;
    let mut indexed_tree = IndexedMerkleTree::<Poseidon, usize>::new(height, CANOPY).unwrap();
    let mut indexing_array = IndexedArray::<Poseidon, usize>::default();
    indexed_tree.init().unwrap();
    indexing_array.init().unwrap();

    let value = 1_u32.to_biguint().unwrap();

    let non_inclusion_proof = indexed_tree
        .get_non_inclusion_proof(&value, &indexing_array)
        .unwrap();

    NonInclusionMerkleProofInputs {
        root: BigInt::from_bytes_be(Sign::Plus, non_inclusion_proof.root.as_slice()),
        value: BigInt::from_bytes_be(Sign::Plus, &non_inclusion_proof.value),
        leaf_lower_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_lower_range_value,
        ),
        leaf_higher_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_higher_range_value,
        ),
        next_index: BigInt::from(non_inclusion_proof.next_index),
        merkle_proof_hashed_indexed_element_leaf: non_inclusion_proof
            .merkle_proof
            .iter()
            .map(|x| BigInt::from_bytes_be(Sign::Plus, x))
            .collect(),
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
    }
}

pub fn opt_non_inclusion_merkle_tree_inputs(height: usize) -> NonInclusionMerkleProofInputs {
    const CANOPY: usize = 0;
    let indexed_tree =
        light_merkle_tree_reference::indexed::IndexedMerkleTree::<Poseidon, usize>::new(
            height, CANOPY,
        )
        .unwrap();

    let value = 1_u32.to_biguint().unwrap();

    let non_inclusion_proof = indexed_tree.get_non_inclusion_proof(&value).unwrap();

    NonInclusionMerkleProofInputs {
        root: BigInt::from_bytes_be(Sign::Plus, non_inclusion_proof.root.as_slice()),
        value: BigInt::from_bytes_be(Sign::Plus, &non_inclusion_proof.value),
        leaf_lower_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_lower_range_value,
        ),
        leaf_higher_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_higher_range_value,
        ),
        next_index: BigInt::from(non_inclusion_proof.next_index),
        merkle_proof_hashed_indexed_element_leaf: non_inclusion_proof
            .merkle_proof
            .iter()
            .map(|x| BigInt::from_bytes_be(Sign::Plus, x))
            .collect(),
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
    }
}

pub fn non_inclusion_inputs_string_v1(
    number_of_utxos: usize,
) -> (String, NonInclusionMerkleProofInputs) {
    let (json_struct, public_inputs) = non_inclusion_new_with_public_inputs_v1(number_of_utxos);
    (json_struct.to_string(), public_inputs)
}

pub fn non_inclusion_new_with_public_inputs_v1(
    number_of_utxos: usize,
) -> (BatchNonInclusionJsonStructV1, NonInclusionMerkleProofInputs) {
    let merkle_inputs = non_inclusion_merkle_tree_inputs(26);

    let input = LegacyNonInclusionJsonStruct {
        root: big_int_to_string(&merkle_inputs.root),
        value: big_int_to_string(&merkle_inputs.value),
        path_elements: merkle_inputs
            .merkle_proof_hashed_indexed_element_leaf
            .iter()
            .map(big_int_to_string)
            .collect(),
        path_index: merkle_inputs
            .index_hashed_indexed_element_leaf
            .to_u32()
            .unwrap(),
        next_index: merkle_inputs.next_index.to_u32().unwrap(),
        leaf_lower_range_value: big_int_to_string(&merkle_inputs.leaf_lower_range_value),
        leaf_higher_range_value: big_int_to_string(&merkle_inputs.leaf_higher_range_value),
    };

    let inputs = vec![input; number_of_utxos];
    (
        BatchNonInclusionJsonStructV1 {
            circuit_type: CircuitType::NonInclusion.to_string(),
            address_tree_height: 26,
            inputs,
        },
        merkle_inputs,
    )
}

pub fn non_inclusion_inputs_string_v2(number_of_utxos: usize) -> String {
    let (json_struct, _) = non_inclusion_new_with_public_inputs_v2(number_of_utxos).unwrap();
    json_struct.to_string()
}

pub fn non_inclusion_new_with_public_inputs_v2(
    number_of_utxos: usize,
) -> Result<(BatchNonInclusionJsonStructV2, [u8; 32]), ProverClientError> {
    let merkle_inputs = opt_non_inclusion_merkle_tree_inputs(40);

    let input = NonInclusionJsonStructV2 {
        root: big_int_to_string(&merkle_inputs.root),
        value: big_int_to_string(&merkle_inputs.value),
        path_elements: merkle_inputs
            .merkle_proof_hashed_indexed_element_leaf
            .iter()
            .map(big_int_to_string)
            .collect(),
        path_index: merkle_inputs
            .index_hashed_indexed_element_leaf
            .to_u32()
            .unwrap(),
        leaf_lower_range_value: big_int_to_string(&merkle_inputs.leaf_lower_range_value),
        leaf_higher_range_value: big_int_to_string(&merkle_inputs.leaf_higher_range_value),
    };
    let inputs = vec![input; number_of_utxos];
    let public_input_hash = create_two_inputs_hash_chain(
        vec![bigint_to_u8_32(&merkle_inputs.root).unwrap(); number_of_utxos].as_slice(),
        vec![bigint_to_u8_32(&merkle_inputs.value).unwrap(); number_of_utxos].as_slice(),
    )?;
    let public_input_hash_string = big_int_to_string(&BigInt::from_bytes_be(
        num_bigint::Sign::Plus,
        &public_input_hash,
    ));
    Ok((
        BatchNonInclusionJsonStructV2 {
            circuit_type: CircuitType::NonInclusion.to_string(),
            address_tree_height: 40,
            public_input_hash: public_input_hash_string,
            inputs,
        },
        public_input_hash,
    ))
}

pub fn inclusion_new_with_public_inputs_v1(number_of_utxos: usize) -> BatchInclusionJsonStructV1 {
    let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H26);

    let input = InclusionJsonStruct {
        root: big_int_to_string(&merkle_inputs.root),
        leaf: big_int_to_string(&merkle_inputs.leaf),
        pathElements: merkle_inputs
            .path_elements
            .iter()
            .map(big_int_to_string)
            .collect(),
        pathIndex: merkle_inputs.path_index.to_u32().unwrap(),
    };

    let inputs = vec![input; number_of_utxos];
    BatchInclusionJsonStructV1 {
        circuit_type: CircuitType::Inclusion.to_string(),
        state_tree_height: 26,
        inputs,
    }
}

pub fn inclusion_inputs_string_v1(number_of_utxos: usize) -> String {
    let json_struct = inclusion_new_with_public_inputs_v1(number_of_utxos);
    json_struct.to_string()
}

pub fn inclusion_new_with_public_inputs_v2(
    number_of_utxos: usize,
) -> (BatchInclusionJsonStruct, [u8; 32]) {
    let merkle_inputs = inclusion_merkle_tree_inputs(MerkleTreeInfo::H32);

    let input = InclusionJsonStruct {
        root: big_int_to_string(&merkle_inputs.root),
        leaf: big_int_to_string(&merkle_inputs.leaf),
        pathElements: merkle_inputs
            .path_elements
            .iter()
            .map(big_int_to_string)
            .collect(),
        pathIndex: merkle_inputs.path_index.to_u32().unwrap(),
    };

    let inputs = vec![input; number_of_utxos];
    let public_input_hash = create_two_inputs_hash_chain(
        vec![bigint_to_u8_32(&merkle_inputs.root).unwrap(); number_of_utxos].as_slice(),
        vec![bigint_to_u8_32(&merkle_inputs.leaf).unwrap(); number_of_utxos].as_slice(),
    )
    .unwrap();
    let public_input_hash_string = big_int_to_string(&BigInt::from_bytes_be(
        num_bigint::Sign::Plus,
        &public_input_hash,
    ));
    (
        BatchInclusionJsonStruct {
            circuit_type: CircuitType::Inclusion.to_string(),
            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
            public_input_hash: public_input_hash_string,
            inputs,
        },
        public_input_hash,
    )
}

pub fn inclusion_inputs_string_v2(number_of_utxos: usize) -> String {
    let (json_struct, _) = inclusion_new_with_public_inputs_v2(number_of_utxos);
    json_struct.to_string()
}

fn combined_new_with_public_inputs_v1(
    num_inclusion: usize,
    num_non_inclusion: usize,
) -> CombinedJsonStructV1 {
    let inclusion = inclusion_new_with_public_inputs_v1(num_inclusion);
    let (non_inclusion, _) = non_inclusion_new_with_public_inputs_v1(num_non_inclusion);

    CombinedJsonStructV1 {
        circuit_type: CircuitType::Combined.to_string(),
        state_tree_height: inclusion.state_tree_height,
        address_tree_height: non_inclusion.address_tree_height,

        inclusion: inclusion.inputs,
        non_inclusion: non_inclusion.inputs,
    }
}

pub fn combined_inputs_string_v1(num_inclusion: usize, num_non_inclusion: usize) -> String {
    let json_struct = combined_new_with_public_inputs_v1(num_inclusion, num_non_inclusion);
    json_struct.to_string()
}

fn combined_new_with_public_inputs_v2(
    num_inclusion: usize,
    num_non_inclusion: usize,
) -> Result<CombinedJsonStructV2, ProverClientError> {
    let (inclusion, inclusion_public_input_hash) =
        inclusion_new_with_public_inputs_v2(num_inclusion);
    let (non_inclusion, non_inclusion_public_input_hash) =
        non_inclusion_new_with_public_inputs_v2(num_non_inclusion)?;

    let public_inputs_hash = create_hash_chain_from_array([
        inclusion_public_input_hash,
        non_inclusion_public_input_hash,
    ])?;

    Ok(CombinedJsonStructV2 {
        circuit_type: CircuitType::Combined.to_string(),
        state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
        address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
        public_input_hash: big_int_to_string(&BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            public_inputs_hash.as_slice(),
        )),
        inclusion: inclusion.inputs,
        non_inclusion: non_inclusion.inputs,
    })
}

pub fn combined_inputs_string_v2(num_inclusion: usize, num_non_inclusion: usize) -> String {
    let json_struct = combined_new_with_public_inputs_v2(num_inclusion, num_non_inclusion);
    json_struct.unwrap().to_string()
}
