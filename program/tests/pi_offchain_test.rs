// #[cfg(test)]
// pub mod tests {
// use crate::parse_verifyingkey_254::get_pvk_from_bytes_254;
// use crate::verifyingkey_254_hc::*;
use ark_ec::ProjectiveCurve; // Needed for into_affine()
use ark_ff::{Fp256, FromBytes};
use ark_groth16::{prepare_inputs, prepare_verifying_key};
use ark_std::{One, Zero};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use light_protocol_program::{
    groth16_verifier::{
        parsers::*,
        prepare_inputs,
        prepare_inputs::{processor::*, state::*},
    },
    utils::prepared_verifying_key::*,
};
mod fe_offchain_test;
use crate::fe_offchain_test::tests::get_pvk_from_bytes_254;

#[test]
#[ignore]
fn pi_254_test_with_7_inputs() {
    // 7 inputs á 32 bytes. For bn254 curve. Skip the first two bytes.
    // TODO: add random input testing
    let inputs_bytes = [
        40, 3, 139, 101, 98, 198, 106, 26, 157, 253, 217, 85, 208, 20, 62, 194, 7, 229, 230, 196,
        195, 91, 112, 106, 227, 5, 89, 90, 68, 176, 218, 172, 23, 34, 1, 0, 63, 128, 161, 110, 190,
        67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49,
        225, 114, 78, 100, 48, 224, 137, 70, 92, 255, 138, 142, 119, 60, 162, 100, 218, 34, 199,
        20, 246, 167, 35, 235, 134, 225, 54, 67, 209, 246, 194, 128, 223, 27, 115, 112, 25, 13,
        113, 159, 110, 133, 81, 26, 27, 23, 26, 184, 1, 175, 109, 99, 85, 188, 45, 119, 213, 233,
        137, 186, 52, 25, 2, 52, 160, 2, 122, 107, 18, 62, 183, 110, 221, 22, 145, 254, 220, 22,
        239, 208, 169, 202, 190, 70, 169, 206, 157, 185, 145, 226, 81, 196, 182, 29, 125, 181, 119,
        242, 71, 107, 10, 167, 4, 10, 212, 160, 90, 85, 209, 147, 16, 119, 99, 254, 93, 143, 137,
        91, 121, 198, 246, 245, 79, 190, 201, 63, 229, 250, 134, 157, 180, 3, 12, 228, 236, 174,
        112, 138, 244, 188, 161, 144, 60, 210, 99, 115, 64, 69, 63, 35, 176, 250, 189, 20, 28, 23,
        2, 19, 94, 196, 88, 14, 51, 12, 21,
    ];

    // TODO: currently switching types from fq to fr. double check this before production.
    let input1 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[2..34]).unwrap();
    let input2 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[34..66]).unwrap();
    let input3 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[66..98]).unwrap();
    let input4 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[98..130]).unwrap();
    let input5 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[130..162]).unwrap();
    let input6 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[162..194]).unwrap();

    let input7 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&inputs_bytes[194..226]).unwrap();

    let inputs: Vec<Fp256<ark_bn254::FrParameters>> =
        vec![input1, input2, input3, input4, input5, input6, input7];
    // parse vk and prepare it.
    // prepare inputs with pvk and inputs for bn254.
    // TODO: Add ark-groth16 support? how can we deal with this while using solana in the same repo?
    let vk = get_pvk_from_bytes_254().unwrap();
    let pvk = prepare_verifying_key(&vk);
    let prepared_inputs = prepare_inputs(&pvk, &inputs).unwrap();
    println!("prepared inputs from library: {:?}", prepared_inputs);

    // execute full onchain mock -> same results?
    // call processor with i_order
    // need local account struct those pass along and r/w to
    // mocking the parsing of instruction_data between 42-45 and 56,57,58  (current_index)

    // init local bytes array (mocking onchain account)
    let mock_account = [0; 3900];
    // ix_order_array
    // for each ix call processor. If applicable with extra instruction_data
    // let mut current_index = 99;
    let mut account_data = PrepareInputsState::unpack(&mock_account).unwrap();
    // replicate 1809 rpc calls
    for index in 0..464 {
        // 0..1809 @ last
        println!("c ixorderarr: {}", IX_ORDER_ARRAY[index]);
        println!("index: {:?}", index);
        prepare_inputs::processor::_process_instruction(
            // IX_ORDER_ARRAY[usize::from(index)],
            IX_ORDER_ARRAY[index],
            &mut account_data,
            //&inputs,
            usize::from(CURRENT_INDEX_ARRAY[index]), // usize::from(CURRENT_INDEX_ARRAY[usize::from(index)]),
        );
    }

    assert_eq!(
        parse_x_group_affine_from_bytes(&account_data.x_1_range),
        prepared_inputs.into_affine(),
        "library implementation differs from pi_instructions"
    );
    println!("pi_254_test (offchain) successful");
}
//}
