use crate::parsers_merkle_tree::*;

use ark_crypto_primitives::crh::{pedersen, poseidon, *};
use ark_ff::{fields, models::Fp256};
use ark_ed_on_bls12_381::EdwardsProjective as Edwards;
use ark_ec::twisted_edwards_extended::*;
use ark_ed_on_bls12_381;
use ark_sponge::poseidon::*;
use ark_sponge;
use ark_sponge::CryptographicSponge;
use ark_sponge::Absorb;

use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::fields::models::Fp256Parameters;
use ark_ff::biginteger::BigInteger256;
use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
};
use ark_ff::PrimeField;
use ark_ff::BigInteger;
use crate::MerkleTree;
use crate::HashBytes;
use crate::get_params;

// called 23 times
// 1 (runs after permute)
pub fn absorb_instruction_1(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &u8){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let elems = value.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();

    sponge.absorb_internal(0, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}


// 2, OR 22
// else: absorbindex != sponge.rate => doesnt permute
// Called 25 times with different inputs
pub fn absorb_instruction_2(abs_idx: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &u8){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let elems = value.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    sponge.absorb_internal(abs_idx, elems.as_slice());

    // store sponge
    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

//1, 2
pub fn absorb_instruction_squeeze_field_elem_22(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &Vec<u8>, index: usize){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);
    //println!("here22 1");
    //println!("hash left: {:?}", value);
    let parsed_field_element =  <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**value).unwrap();
    //println!("here22 2");

    let elems = parsed_field_element.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();

    sponge.absorb_internal(index, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

// 3 OR 4 (based on i) OR 20,21
// called twice at start and twice at end
pub fn permute_instruction_1_and_3(i: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);
    let mut state = sponge.state.clone();

    // round 1:
    sponge.apply_ark(&mut state, i as usize);
    sponge.apply_s_box(&mut state, true);
    sponge.apply_mds(&mut state);

    // round 2:
    sponge.apply_ark(&mut state, i+1 as usize);
    sponge.apply_s_box(&mut state, true);
    sponge.apply_mds(&mut state);

    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);
}

// 5,6,7,8,9,10,11,12,13,14,15,16,17,18
// call 70/5=14 times:
pub fn permute_instruction_2_x_5(i: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){
    //sol_log_compute_units();
    //msg!("-1");
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);
    //sol_log_compute_units();
    //msg!("0");
    let mut state = sponge.state.clone();
    //sol_log_compute_units();
    //msg!("1");
    sponge.apply_ark(&mut state, i as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);
    //sol_log_compute_units();
    //msg!("2");
    sponge.apply_ark(&mut state, i+1 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);
    //();
    //msg!("3");
    sponge.apply_ark(&mut state, i+2 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);
    //sol_log_compute_units();
    //msg!("4");
    sponge.apply_ark(&mut state, i+3 as usize);
    sponge.apply_s_box(&mut state, false);
    //sol_log_compute_units();
    //msg!("4.1");

    sponge.apply_mds(&mut state);
    //sol_log_compute_units();

    //msg!("5");

    sponge.apply_ark(&mut state, i+4 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);
    //sol_log_compute_units();
    //msg!("6");
    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);
    //sol_log_compute_units();



}


pub fn permute_instruction_2_x_4(i: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){

    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let mut state = sponge.state.clone();

    sponge.apply_ark(&mut state, i as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+1 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+2 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+3 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);


}

pub fn permute_instruction_2_x_2(i: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){

    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let mut state = sponge.state.clone();

    sponge.apply_ark(&mut state, i as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+1 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);


}

// 19
// call 1 time: this + #18 total up to 73 rounds
pub fn permute_instruction_2_x_3(i: usize, range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>){

    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);
    let mut state = sponge.state.clone();

    sponge.apply_ark(&mut state, i as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+1 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    sponge.apply_ark(&mut state, i+2 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);

}
pub fn squeeze_internal_custom( range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, res: &mut Vec<u8>,squeeze_index: usize){

    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let mut hash  = vec![0u8;32];
    for (i, element) in range_1.iter().enumerate() {
            hash[i] = *element;

    }

    *res = hash.clone();
    // store sponge
    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn insert_0(leaf: &Vec<u8>, merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    hash_bytes_account.currentIndex =  merkle_tree_account.nextIndex;
    assert!(hash_bytes_account.currentIndex != 2048/*2usize^merkle_tree_account.levels*/, "Merkle tree is full. No more leafs can be added");

    hash_bytes_account.currentLevelHash = leaf.clone();
    merkle_tree_account.leaves = leaf.clone();


    merkle_tree_account.inserted_leaf = true;
}

pub fn insert_1_inner_loop(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    //msg!("insert_1_inner_loop_0 level {:?}",hash_bytes_account.currentLevel);
    //msg!("currentLevelHash {:?}",hash_bytes_account.currentLevelHash);
    if(hash_bytes_account.currentIndex % 2 == 0) {
        hash_bytes_account.left = hash_bytes_account.currentLevelHash.clone();
        hash_bytes_account.right =  merkle_tree_account.zeros[ hash_bytes_account.currentLevel].clone();
        merkle_tree_account.filledSubtrees[ hash_bytes_account.currentLevel] = hash_bytes_account.currentLevelHash.clone();
    } else {
        hash_bytes_account.left =  merkle_tree_account.filledSubtrees[ hash_bytes_account.currentLevel].clone();
        hash_bytes_account.right = hash_bytes_account.currentLevelHash.clone();
    }
    hash_bytes_account.currentIndex /= 2;
    hash_bytes_account.currentLevel += 1;
    //merkle_tree_account.inserted_leaf = true;
    //msg!("insert_1_inner_loop_0 subtrees {:?}",merkle_tree_account.filledSubtrees);

}

pub fn insert_last(merkle_tree_account: &mut MerkleTree, hash_bytes_account:&mut HashBytes) {
    merkle_tree_account.currentRootIndex = ( merkle_tree_account.currentRootIndex + 1) %  merkle_tree_account.ROOT_HISTORY_SIZE;
    merkle_tree_account.nextIndex+= 1;

    //roots unpack only current root and write only this one
    merkle_tree_account.roots = hash_bytes_account.currentLevelHash.clone();
    merkle_tree_account.inserted_root = true;

}

pub fn init_sponge(hash_bytes_account:&mut HashBytes) {
    let mut sponge = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_params());
    parse_state_to_bytes(&sponge.state, &mut hash_bytes_account.state_range_1, &mut hash_bytes_account.state_range_2, &mut hash_bytes_account.state_range_3);

}


pub fn deposit(merkle_tree_account: &mut MerkleTree, account: &AccountInfo, account_tmp: &AccountInfo){
        //if the user actually deposited 1 sol increase current_total_deposits by one

        **account_tmp.try_borrow_mut_lamports().unwrap()                -= 1000000000; // 1 SOL

        **account.try_borrow_mut_lamports().unwrap()        += 1000000000;

        merkle_tree_account.current_total_deposits += 1;
        msg!("Deposit of 1 Sol successfull");
}
