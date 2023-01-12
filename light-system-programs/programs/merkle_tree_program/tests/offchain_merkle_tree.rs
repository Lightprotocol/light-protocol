#![allow(unused)]
#![allow(clippy::needless_range_loop)]
mod merkle_tree;

use anchor_lang::prelude::Pubkey;
use ark_crypto_primitives::{
    crh::{TwoToOneCRH, CRH},
    Error,
};
use ark_ed_on_bn254;
use ark_ed_on_bn254::{Fq, Fr};
use ark_ff::{bytes::{FromBytes, ToBytes}, PrimeField, BigInteger};
use ark_ff::BigInteger256;
use ark_ff::Fp256;
use ark_serialize::{Read, Write};
use ark_std::vec::Vec;
use ark_std::{test_rng, UniformRand};

use arkworks_gadgets::poseidon::{
    circom::CircomCRH, sbox::PoseidonSbox, PoseidonError, PoseidonParameters, Rounds,
};
use arkworks_gadgets::utils::{
    get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3, parse_vec,
};
use merkle_tree::merkle_tree::{hash_64_to_vec, MerkleTree, Path, PoseidonCircomCRH3};
use merkle_tree_program;
use merkle_tree_program::poseidon_merkle_tree::state;
use merkle_tree_program::poseidon_merkle_tree::state::MerkleTree as MerkleTreeOnchain;
use merkle_tree_program::poseidon_merkle_tree::update_merkle_tree_lib::instructions::insert_last_double;
use merkle_tree_program::poseidon_merkle_tree::update_merkle_tree_lib::processor::compute_updated_merkle_tree;
use merkle_tree_program::poseidon_merkle_tree::update_merkle_tree_lib::MerkleTreeUpdateState;
use merkle_tree_program::utils::config;
use merkle_tree_program::utils::config::ENCRYPTED_UTXOS_LENGTH;
use solana_program::program_pack::Pack;
use std::cell::RefMut;
use std::convert::TryInto;
const INSTRUCTION_ORDER_POSEIDON_2_INPUTS: [u8; 3] = [0, 1, 2];

#[test]
#[ignore]
fn merkle_tree_new_test() {
    //test for the reference implementation to test the onchain merkletree against
    let tree_height = 4;

    let zero_value = hash_64_to_vec(vec![1u8; 64]).to_vec();
    println!("zero value: {:?}", zero_value);
    let leaves: Vec<Vec<u8>> = vec![zero_value; 16];
    let mut tree = MerkleTree::new(&leaves).unwrap();
    let leaf1: Vec<u8> = hash_64_to_vec(vec![2u8; 64]).to_vec();
    tree.update(0, &leaf1);
    let leaf2: Vec<u8> = hash_64_to_vec(vec![3u8; 64]).to_vec();
    tree.update(1, &leaf2);
    let leaf3: Vec<u8> = hash_64_to_vec(vec![4u8; 64]).to_vec();
    tree.update(2, &leaf3);
    let proof = tree.generate_proof(2).unwrap();
    println!("Proof: {:?}", proof);
    assert!(proof.verify(&tree.root(), &leaf3).unwrap());
}

#[test]
fn merkle_tree_verify_init_bytes_merkle_tree_18() {
    let mut zero_value = vec![1 as u8; 32];

    let rounds = get_rounds_poseidon_circom_bn254_x5_3::<Fq>();
    let mds = get_mds_poseidon_circom_bn254_x5_3::<Fq>();
    let params = PoseidonParameters::<Fq>::new(rounds, mds);

    //generating leaf hash from zero value
    let mut current_level_hash =
        <PoseidonCircomCRH3 as TwoToOneCRH>::evaluate(&params, &zero_value, &zero_value).unwrap();
    <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(
        &current_level_hash,
        &mut zero_value[..],
    );

    // looping over init bytes and asserting them with dynamically created poseidon hashes
    for (level, level_hash) in ZERO_BYTES_MERKLE_TREE_18.iter().enumerate() {
        current_level_hash =
            <PoseidonCircomCRH3 as TwoToOneCRH>::evaluate(&params, &zero_value, &zero_value)
                .unwrap();
        <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(
            &current_level_hash,
            &mut zero_value[..],
        );

        assert_eq!(
            zero_value, level_hash,
            "Verification of initbytes failed at level {}",
            level
        );
        assert_eq!(
            zero_value, ZERO_BYTES_MERKLE_TREE_18[level],
            "Verification of zero bytes failed at level {}",
            level
        );
    }
}

use merkle_tree_program::poseidon_merkle_tree::initialize_new_merkle_tree_18;
use merkle_tree_program::utils::config::{MERKLE_TREE_HISTORY_SIZE, ZERO_BYTES_MERKLE_TREE_18};
use std::cell::RefCell;

#[test]
#[ignore]
fn print_zero_values() {
    let tree_height = 18;

    let zero_value = vec![
        40, 66, 58, 227, 48, 224, 249, 227, 188, 18, 133, 168, 156, 214, 220, 144, 244, 144, 67,
        82, 76, 6, 135, 78, 64, 186, 52, 113, 234, 47, 27, 32,
    ]; //hash_64_to_vec(vec![1u8;64]).to_vec();//Fq::one().into_repr().to_bytes_le();
    let mut zero_values = Vec::new();
    let mut current_zero = zero_value.clone();
    zero_values.push(current_zero.clone());
    println!(
        "pub const ZERO_BYTES_MERKLE_TREE_18: [[u8;32];{}] = [\n \t {:?},",
        tree_height, current_zero
    );
    for i in 0..32 {
        current_zero = hash_64_to_vec([current_zero.clone(), current_zero].concat()).to_vec();
        zero_values.push(current_zero.clone());
        println!("\t {:?},", current_zero);
    }
    println!("]; ");
}

#[test]
fn test_initialize() {
    let tree_height = 8;

    let zero_value = vec![
        40, 66, 58, 227, 48, 224, 249, 227, 188, 18, 133, 168, 156, 214, 220, 144, 244, 144, 67,
        82, 76, 6, 135, 78, 64, 186, 52, 113, 234, 47, 27, 32,
    ];

    let mut mt = MerkleTreeOnchain {
        filled_subtrees: [[0u8; 32]; 18],
        current_root_index: 0u64,
        next_index: 0u64,
        roots: [[0u8; 32]; MERKLE_TREE_HISTORY_SIZE as usize],
        pubkey_locked: Pubkey::new(&[0u8; 32]),
        time_locked: 0u64,
        height: 0u64,
        merkle_tree_nr: 0u64,
        lock_duration: 20u64
    };
    let mt_index = 0;
    let binding = &mut RefCell::new(mt);
    let mut ref_mt = binding.borrow_mut();
    initialize_new_merkle_tree_18::process_initialize_new_merkle_tree_18(
        &mut ref_mt,
        tree_height,
        ZERO_BYTES_MERKLE_TREE_18.to_vec(),
        mt_index,
    );

    let leaves: Vec<Vec<u8>> =
        vec![zero_value.clone(); 2_usize.pow(tree_height.try_into().unwrap())];
    println!("starting to init arkworks_fork tree");
    let mut tree = MerkleTree::new(&leaves).unwrap();
    println!("root: {:?}", tree.root());
    // assert_eq!(ref_mt.height + 1, tree.height().try_into().unwrap());
    // assert_eq!(ref_mt.roots[0].to_vec(), tree.root());
    println!("1u8; 64] {:?}", hash_64_to_vec(vec![1u8; 64]));
    let new_leaf = vec![3u8;32];
    tree.update(0, &new_leaf);
    tree.update(1, &new_leaf);
    println!("{:?}", tree.root());
    // println!("{:?}", vec![[2u8;32], [1u8;32]].concat());

    // println!("[1u8;32], [2u8;32] {:?}", hash_64_to_vec(vec![[1u8;32], [2u8;32]].concat()));
    // println!("[1u8;32], [1u8;32] {:?}", hash_64_to_vec(vec![[1u8;64]].concat()));
    // use ark_bn254::Fq;
    // let input1 = Fq::from_be_bytes_mod_order(&[1u8; 32]);
    // let input2 = Fq::from_be_bytes_mod_order(&[2u8; 32]);
    // use ark_ed_on_bn254::Fq as FqEd;


    // println!("[1u8;32], [2u8;32] {:?}", hash_64_to_vec([input1.into_repr().to_bytes_be(), input2.into_repr().to_bytes_be()].concat()));
    use ark_bn254::Fr as FrBn;
    use ark_bn254::Fq as FqBn;

    let input1 = Fq::from_be_bytes_mod_order(&[3u8; 32]);
    println!("input 1 {:?}", input1);
    println!(" Fr {:?}", Fr::from_be_bytes_mod_order(&[3u8; 32]));
    println!(" FrBn {:?}", FrBn::from_be_bytes_mod_order(&[3u8; 32]));
    println!(" FqBn {:?}", FqBn::from_be_bytes_mod_order(&[3u8; 32]));

    // let input2 = Fq::from_be_bytes_mod_order(&[3u8; 32]);
    // let input1ED = FqEd::from_be_bytes_mod_order(&[3u8; 32]);
    // assert_eq!(input1.into_repr().to_bytes_le(), input1ED.into_repr().to_bytes_le());
    // println!("[3u8;32], [3u8;32] {:?}", hash_64_to_vec([input1.into_repr().to_bytes_le(), input2.into_repr().to_bytes_le()].concat()));

    // 40 7

}

#[test]
fn batch_update_smt_test() {
    //testing full arkforks_merkle tree vs sparse tornado cash fork tree for height 18
    let tree_height: u64 = 18;
    const ITERATIONS: usize = 100;

    println!("tree_height: {}", tree_height);

    let mut mt = MerkleTreeOnchain {
        filled_subtrees: [[0u8; 32]; 18],
        current_root_index: 0u64,
        next_index: 0u64,
        roots: [[0u8; 32]; 256],
        pubkey_locked: Pubkey::new(&[0u8; 32]),
        time_locked: 0u64,
        height: 0u64,
        merkle_tree_nr: 0u64,
        lock_duration: 0u64
    };
    let mt_index = 0;
    let binding = &mut RefCell::new(mt);
    let mut smt = binding.borrow_mut();
    initialize_new_merkle_tree_18::process_initialize_new_merkle_tree_18(
        &mut smt,
        tree_height,
        ZERO_BYTES_MERKLE_TREE_18.to_vec(),
        mt_index,
    );

    let initial_zero_hash = config::ZERO_BYTES_MERKLE_TREE_18[0].to_vec();
    let zero_value = vec![
        40, 66, 58, 227, 48, 224, 249, 227, 188, 18, 133, 168, 156, 214, 220, 144, 244, 144, 67,
        82, 76, 6, 135, 78, 64, 186, 52, 113, 234, 47, 27, 32,
    ]; //hash_64_to_vec(vec![1u8;64]).to_vec();//Fq::one().into_repr().to_bytes_le();
    assert_eq!(initial_zero_hash, zero_value);

    println!("init successful");
    let mut rng = test_rng();

    let leaves: Vec<Vec<u8>> =
        vec![initial_zero_hash.clone(); 2_usize.pow(tree_height.try_into().unwrap())];
    println!("starting to init arkworks_fork tree");
    let mut tree = MerkleTree::new(&leaves).unwrap();

    let mut j = 0;
    for i in 0..ITERATIONS {
        let mut filled_leaves = Vec::<[[u8; 32]; 2]>::new();
        // filled_leaves.push([vec![
        //    97, 182, 164, 119,  88, 188,   3,   6,
        //    86,  29,  64, 115, 216, 126,  65, 250,
        //    21, 183,  53, 226,  34, 204, 117, 118,
        //   133,  53, 186,  79,  60, 132,   0,  42
        // ]
        // .try_into()
        // .unwrap(), vec![
        //   135, 113, 171, 189, 221,  82, 238,  58,
        //   214, 114,  66, 168, 151, 143,  90,  65,
        //   252,  76, 206, 240, 108,  50, 222, 196,
        //    34,  77,  44,   1,  15, 243, 130,   5
        // ]
        // .try_into()
        // .unwrap()]);
        // println!("INSERT_INSTRUCTION_ORDER_18 {:?}", INSERT_INSTRUCTION_ORDER_18);
        //
        // filled_leaves.push([vec![
        //     177, 178, 52, 116, 232, 152, 188, 86, 170, 183, 5, 59, 51, 142, 44, 62, 78, 105, 95, 4,
        //     247, 13, 250, 27, 153, 208, 63, 76, 70, 159, 54, 10,
        // ]
        // .try_into()
        // .unwrap(), vec![
        // 218, 210, 112, 195, 148, 121, 95, 46, 107, 224, 46, 89, 100, 236, 202, 218, 164, 24,
        // 16, 25, 13, 235, 6, 65, 239, 70, 165, 32, 152, 43, 73, 18,
        // ]
        // .try_into()
        // .unwrap()]);

        for _ in 0..((i % 15) + 1) {
            let new_leaf_hash = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng);

            let mut new_leaf_hash_bytes = [0u8; 32];
            <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(
                &new_leaf_hash,
                &mut new_leaf_hash_bytes[..],
            );

            let new_leaf_hash_1 = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng);

            let mut new_leaf_hash_bytes_1 = [0u8; 32];

            <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(
                &new_leaf_hash_1,
                &mut new_leaf_hash_bytes_1[..],
            );
            filled_leaves.push([new_leaf_hash_bytes, new_leaf_hash_bytes_1]);
        }

        let mut tmp_pda = MerkleTreeUpdateState {
            node_left: [0u8; 32],
            node_right: [0u8; 32],
            leaf_left: [0u8; 32],
            leaf_right: [0u8; 32],
            merkle_tree_pda_pubkey: Pubkey::new(&[0u8; 32]),
            // verifier_tmp_pda: vec![0u8; 32],
            relayer: Pubkey::new(&[0u8; 32]),

            state: [0u8; 96],
            current_round: 0,
            current_round_index: 0,

            current_level_hash: [0u8; 32],
            current_index: 0u64,
            current_level: 0u64,
            current_instruction_index: 1u64,
            insert_leaves_index: 0,
            leaves: [[[0u8; 32]; 2]; 16],
            number_of_leaves: filled_leaves.len().try_into().unwrap(),
            tmp_leaves_index: smt.next_index,
            filled_subtrees: smt.filled_subtrees.clone(),
        };
        let tmp = RefCell::new(tmp_pda);
        let mut verifier_state_data: RefMut<'_, MerkleTreeUpdateState> = tmp.borrow_mut();

        for (j, leaves) in filled_leaves.iter().enumerate() {
            verifier_state_data.leaves[j][0] = leaves[0];
            verifier_state_data.leaves[j][1] = leaves[1];
        }

        print!("tmp_pda.leaves {:?}", tmp_pda.leaves);
        let mut counter = 0;
        while verifier_state_data.current_instruction_index != 56 {
            compute_updated_merkle_tree(
                merkle_tree_program::utils::constants::IX_ORDER
                    [verifier_state_data.current_instruction_index as usize],
                &mut verifier_state_data,
                &mut smt, /*new_leaf_hash_bytes.clone(), new_leaf_hash_bytes_1.clone()*/
            );
            verifier_state_data.current_instruction_index += 1;
            counter += 1;
            println!(
                "counter {} current_instruction_index {}",
                counter, verifier_state_data.current_instruction_index
            );
            println!(
                "counter {} current_instruction_index {}",
                counter, verifier_state_data.current_round
            );
            println!(
                "insert_leaves_index {} number_of_leaves {}",
                verifier_state_data.insert_leaves_index, verifier_state_data.number_of_leaves
            );
        }

        insert_last_double(&mut smt, &mut verifier_state_data);
        assert_eq!(smt.filled_subtrees, verifier_state_data.filled_subtrees);

        for leaves in filled_leaves.iter() {
            tree.update(j, &leaves[0].to_vec());
            tree.update(j + 1, &leaves[1].to_vec());
            j += 2;
        }
        // println!("iter {}",i );
        // println!("verifier_state_data nr leaves {}", verifier_state_data.number_of_leaves);
        // println!("smt {:?}", smt);
        println!("{:?}", smt.roots[i + 1].to_vec());
        assert_eq!(smt.roots[i + 1].to_vec(), tree.root());
    }
}
