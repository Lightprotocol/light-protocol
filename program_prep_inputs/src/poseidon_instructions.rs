use crate::poseidon_parsers::*;
use ark_ff::{fields, models::Fp256};
use ark_std;
use ark_ed_on_bls12_381::EdwardsProjective as Edwards;
use ark_ec::twisted_edwards_extended::*;
use ark_ed_on_bls12_381;
use ark_sponge::poseidon::*;
use crate::state_prep_inputs::{PoseidonHashBytesPrepInputs};

use ark_sponge;
use ark_sponge::CryptographicSponge;
use ark_sponge::Absorb;

use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::fields::models::Fp256Parameters;
use ark_ff::biginteger::BigInteger256;
use ark_ff::PrimeField;
use ark_ff::BigInteger;

// called 23 times
// 1 (runs after permute)
pub fn absorb_instruction_1(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &u8){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let elems = value.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();

    sponge.absorb_internal(0, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn absorb_instruction_squeeze_field_elem_22_0(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &Vec<u8>){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let parsed_field_element =  <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**value).unwrap();

    let elems = parsed_field_element.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    println!("elems {:?}", elems.as_slice());

    sponge.absorb_internal(0, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn absorb_instruction_squeeze_field_elem_22_1(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &Vec<u8>){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let parsed_field_element =  <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**value).unwrap();

    let elems = parsed_field_element.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    println!("elems {:?} ", elems.as_slice());

    sponge.absorb_internal(1, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn absorb_instruction_vec_22_0(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &Vec<u8>){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    //let parsed_field_element =  <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**value).unwrap();

    let elems = value.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    //println!("elems {:?}", elems.as_slice());

    sponge.absorb_internal(0, elems.as_slice());

    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn absorb_instruction_vec_22_1(range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>, value: &Vec<u8>){
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    //let parsed_field_element =  <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**value).unwrap();

    let elems = value.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    //println!("elems {:?} ", elems.as_slice());

    sponge.absorb_internal(1, elems.as_slice());

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

    sponge.apply_ark(&mut state, i+4 as usize);
    sponge.apply_s_box(&mut state, false);
    sponge.apply_mds(&mut state);

    // store sponge
    parse_state_to_bytes(&state, range_1, range_2, range_3);


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
    //changed to reproduce squeeze field element
    //hash[31] = range_2[0];
    //store bytes
    *res = hash.clone();
    // store sponge
    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
}

pub fn absorb_internal_custom_0(
        range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>,
        input0: &Vec<u8>, input1: &Vec<u8>,
        fp256_0: &mut Vec<u8>, fp256_1: &mut Vec<u8>

    ) {
    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);
    let rate_start_index = 1;
    let mut tmp_arr = vec![0u8; 64];
    /*
    let tmp_arr = tmp_arr.into_iter().enumerate().map(|(i, x) | {
        if i < 32 {
            x = input0[i];
        } else {
            x = input1[i%32];
        }
    }).collect::<Vec<_>>();*/
    for i in 0..64 {
        if i < 32 {
            tmp_arr[i] = input0[i];
        } else {
            tmp_arr[i] = input1[i%32];
        }
    }
    let elems = tmp_arr.to_sponge_field_elements_as_vec::<ark_ed_on_bls12_381::Fq>();
    let num_elements_absorbed = 1;
    println!("num_elements_absorbed: {} ", num_elements_absorbed);
    for (i, element) in elems
        .iter()
        .enumerate()
        .take(num_elements_absorbed)
    {
        sponge.state[i + rate_start_index] += element;
    }
    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&elems[1],&mut fp256_0[..]);
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&elems[2], &mut fp256_1[..]);

}


pub fn absorb_internal_custom_1(
        range_1 : &mut Vec<u8>, range_2: &mut Vec<u8>, range_3: &mut Vec<u8>,
        fp256_0: &mut Vec<u8>, fp256_1: &mut Vec<u8>
    ) {


    let mut sponge = parse_sponge_from_bytes(range_1, range_2, range_3);

    let mut remaining_elements = Vec::new();
    remaining_elements.push(<Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**fp256_0).unwrap());
    remaining_elements.push(<Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&**fp256_1).unwrap());

    let rate_start_index = 0;
    for (i, element) in remaining_elements.iter().enumerate() {
        sponge.state[i + rate_start_index] += *element;
    }
    parse_state_to_bytes(&sponge.state, range_1, range_2, range_3);


}

pub fn init_sponge(hash_bytes_account:&mut PoseidonHashBytesPrepInputs) {
    let mut sponge = ark_sponge::poseidon::PoseidonSponge::<ark_ed_on_bls12_381::Fq>::new(&get_params());
    parse_state_to_bytes(&sponge.state, &mut hash_bytes_account.state_range_1, &mut hash_bytes_account.state_range_2, &mut hash_bytes_account.state_range_3);

}
