// use crate::parse_verifyingkey_254::*;
// use crate::verifyingkey_254_hc::*;
use ark_ff::{Fp256, FromBytes};
use ark_groth16::{verify_proof, prepare_inputs, prepare_verifying_key};
use ark_std::{One, Zero};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryInto;

#[test]
fn mo_254_test_offchain() {
    let ix_order_array_mock;
    // init local bytes array (mocking onchain account)
    let mock_account = [0; 4972];

    // let mut account_data = ::unpack(&mock_account).unwrap();

    

    for index in 0..1 {
        // println!("c ixorderarr: {}", ix_order_array_mock[index]);
        // println!("index: {:?}", index);

        // // process ix_millerloop file
        // _pi_254_process_instruction(
        //     // ix_order_array_mock[usize::from(index)],
        //     ix_order_array_mock[index],
        //     &mut account_data,
        //     &inputs,
        //     usize::from(current_index_mock[index]), // usize::from(current_index_mock[usize::from(index)]),
        // );
    }

    // call library for millerloop part...
}

// build here: state, processor,
// new file for: dedicated ix, ranges
// print from inside the lib call
