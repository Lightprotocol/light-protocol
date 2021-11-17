use crate::prepare_inputs::*;
use crate:: hard_coded_verifying_key_pvk_new_ciruit::*;

#[test]
fn prepare_inputs_o() {
        // test --->
    let commitment_slice = [21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21];
    let leaves_slice =  [151, 85, 62, 182, 26, 238, 149, 115, 117, 89, 25, 56, 176, 33, 124, 54, 229, 133, 85, 3, 220, 179, 228, 88, 14, 137, 72, 68, 230, 230, 25, 74];
        // <--- test end
    let inputs_bytes = [0,0, 151, 85, 62, 182, 26, 238, 149, 115, 117, 89, 25, 56, 176, 33, 124, 54, 229, 133, 85, 3, 220, 179, 228, 88, 14, 137, 72, 68, 230, 230, 25, 74, 132, 1, 14, 72, 111, 54, 123, 94, 251, 147, 244, 75, 86, 228, 18, 126, 214, 240, 54, 15, 174, 215, 153, 99, 84, 160, 10, 189, 134, 166, 186, 7, 186, 11, 250, 107, 131, 86, 119, 78, 239, 31, 50, 120, 132, 189, 175, 67, 30, 6, 80, 159, 190, 145, 23, 2, 253, 30, 141, 111, 155, 114, 43, 46, 135, 53, 48, 239, 128, 88, 250, 198, 168, 133, 132, 213, 193, 140, 155, 186, 110, 136, 116, 194, 162, 215, 89, 167, 96, 40, 16, 127, 67, 203, 177, 47];
    println!("inputs_bytes: {:?} => {:?}",inputs_bytes.len(), inputs_bytes);

    // pvk
    let mut r = vec![get_gamma_abc_g1_0(), get_gamma_abc_g1_1(), get_gamma_abc_g1_2(), get_gamma_abc_g1_3(), get_gamma_abc_g1_4()];

    // to inputs
    let input1 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&inputs_bytes[2..34]).unwrap();
    let input2 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&inputs_bytes[34..66]).unwrap();
    let input3 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&inputs_bytes[66..98]).unwrap();
    let input4 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&inputs_bytes[98..130]).unwrap();
    let inputs :  Vec<ark_ff::Fp256<ark_bls12_381::FrParameters>>= vec![input1,input2,input3,input4];

    // call offchain custom function, get result
    let prepared_inputs_cus = prepare_inputs_custom(r, &inputs).unwrap(); // CUSTOM

    // call the instructions that make up the onchain version


    // compare results
}
