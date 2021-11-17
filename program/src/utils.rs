use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::Field;
use ark_ec;
use crate::parsers_part_2::*;
use crate::ranges_part_2::*;
use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
};
use crate::verifyingkey::get_alpha_g1_beta_g2;

pub fn custom_frobenius_map_1_1(mut account: &mut Vec<u8>) {
    custom_frobenius_map_1(&mut account, 1);
}

pub fn custom_frobenius_map_1_2(mut account:  &mut Vec<u8>) {
    custom_frobenius_map_2(&mut account,1);
}

pub fn custom_frobenius_map_2_1(mut account:  &mut Vec<u8>){
    custom_frobenius_map_1(&mut account, 2);
}

pub fn custom_frobenius_map_2_2(mut account:  &mut Vec<u8>) {
    custom_frobenius_map_2(&mut account, 2);
}

pub fn custom_frobenius_map_1(account:  &mut Vec<u8>, power: usize) {
    let mut f = parse_f_from_bytes_new(&account);
    ////msg!"c0 map");
    f.c0.frobenius_map(power);  //                        cost 40641

    ////msg!"c1 map");
    f.c1.frobenius_map(power);//                          cost 40643
    parse_f_to_bytes_new(f, account);

}

pub fn custom_frobenius_map_2(account:  &mut Vec<u8>, power: usize) {
    let mut f = parse_f_from_bytes_new(&account);
    ////msg!"c0 map");
    <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::mul_base_field_by_frob_coeff(&mut f.c1, power);//  cost 41569

    parse_f_to_bytes_new(f, account);

}


pub fn custom_cyclotomic_square(_f_range: &Vec<u8>, _store_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(& _f_range);
    // cost 90496
    let y0 = f.cyclotomic_square();
    //assert!(f != y0);

    parse_f_to_bytes_new(y0,_store_f_range);
}

pub fn conjugate_wrapper(_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(_range);
    f.conjugate();
    ////msg!"{:?}", f);
    parse_f_to_bytes_new(f, _range);
}

pub fn assign_range_x_to_range_y(
    account: &mut [u8],
    range_x: [usize;2],
    range_y: [usize; 2]) {
    assert_eq!(range_x[1] - range_x[0], range_y[1] - range_y[0]);
    let mut start = range_y[0];
    for i in range_x[0]..range_x[1] {
        account[start] = account[i].clone();
        start +=1;
    }

    assert_eq!(parse_f_from_bytes_tmp(account, range_x), parse_f_from_bytes_tmp(account, range_y));
    //account[range_y[0]..range_y[1]] = account[range_x[0]..range_x[1]];
}

pub fn custom_square_in_place_instruction_else_1(
    f_range:  &Vec<u8>,
    cubic_v0_range: &mut Vec<u8>,
    //cubic_v2_range: &mut Vec<u8>,
    cubic_v3_range: &mut Vec<u8>
){
    let mut f = parse_f_from_bytes_new(f_range);  // 58284

    let mut v0 = f.c0 - &f.c1; // cost 768
    //println!("str v0 : {:?}", v0);
    let v3 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0, &f.c1); // cost 1108
    //println!("str v3 : {:?}", v3);

    //let v2 = f.c0 * &f.c1; // cost 86525
    // w v0,v2,v3

    parse_cubic_to_bytes_new(v0, cubic_v0_range, solo_cubic_0_range); // 15198
    assert_eq!(v0, parse_cubic_from_bytes_new(cubic_v0_range, solo_cubic_0_range));
    //parse_cubic_to_bytes_new(v2, cubic_v2_range, solo_cubic_0_range); // 15198
    parse_cubic_to_bytes_new(v3, cubic_v3_range, solo_cubic_0_range); // 15198
    assert_eq!(v3, parse_cubic_from_bytes_new(cubic_v3_range, solo_cubic_0_range));


 // no write f! instable: might overrun
 // total cost ~192403
}

pub fn custom_square_in_place_instruction_else_1_2(
    f_range:  &Vec<u8>,
    cubic_v2_range: &mut Vec<u8>,
){
    let mut f = parse_f_from_bytes_new(f_range);  // 58284

    let v2 = f.c0 * &f.c1; // cost 86525
    // w v0,v2,v3

    parse_cubic_to_bytes_new(v2, cubic_v2_range, solo_cubic_0_range);

 // no write f! instable: might overrun
 // total cost ~192403
}

pub fn custom_square_in_place_instruction_else_2(
    cubic_v0_range: &mut Vec<u8>,
    cubic_v3_range: &Vec<u8>
){

    let mut v0 = parse_cubic_from_bytes_new(cubic_v0_range, solo_cubic_0_range);  // 29478
    let mut v3 = parse_cubic_from_bytes_new(cubic_v3_range, solo_cubic_0_range);  // 29478


    v0 *= &v3; // cost 86105

    parse_cubic_to_bytes_new(v0, cubic_v0_range, solo_cubic_0_range); // 15198

 // total cost to ~160259
}

pub fn custom_square_in_place_instruction_else_3(
    cubic_v0_range: &Vec<u8>,
    cubic_v2_range: &Vec<u8>,
    _f_range: &mut Vec<u8>,
    f_c0_range: [usize;2],
    f_c1_range: [usize;2]
){

    let v0 = parse_cubic_from_bytes_new(cubic_v0_range, solo_cubic_0_range);  // 29478
    let v2 = parse_cubic_from_bytes_new(cubic_v2_range, solo_cubic_0_range);  // 29478

    let c1 = v2.double(); // cost 378
    let c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue_plus_one(&v0, &v2); // cost 1550

    // write f:
    parse_cubic_to_bytes_new(c0, _f_range, f_c0_range); // 15198
    parse_cubic_to_bytes_new(c1, _f_range, f_c1_range); // 15198

     // total cost shorted to 91280
}


use ark_bls12_381::*;
use ark_ff::biginteger::BigInteger384;
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::{Fp256, Fp384};
use ark_ff::CubicExtField;
use ark_ff::fields::models::fp2::*;


pub fn verify_result_and_withdraw(_f1_r_range: &Vec<u8>, account_from: &AccountInfo, account_to: &AccountInfo) {

    let verifyingkey = get_alpha_g1_beta_g2(); // new verif key val
    let result = parse_f_from_bytes_new(_f1_r_range);


    assert_eq!(
        result,
        QuadExtField::<ark_ff::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>>::new(
        CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>>::new(
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([9450009823341693778, 7567869168370272456, 18197991852398659280, 14366248363560943150, 1846449964006099810, 679186062217654437])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([14423346876892333681, 6730281363705116644, 7424630442455670057, 8270520680913369906, 129074939192573571, 1189318454514002421]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15087111543975820103, 4158933278356405436, 18118066973472597158, 16470418406097340038, 7290763036760663413, 1432986065586497656])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13142509635494830254, 1063120006913468487, 9502319634834262306, 4432211635827318749, 14796654634504989696, 1613239985088916413]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([8540514875150680179, 14211944784949309652, 11503029521577459949, 7848771183824012788, 12423583728904914117, 214651426791184375])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([12184631478503290601, 3754734004613789564, 18017661725911600748, 2523454429955899187, 477613221123976217, 1872837268341302997]))
        )
        ),
        CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>>::new(
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([3978527081928063346, 16585342381569337288, 5542417872902215541, 14433809004088446654, 4002383117313680726, 1825637594027485805])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([12282572081966365605, 4130176373697380094, 11274978495552272244, 12480637929685418438, 8247324291841073554, 1292512561380557691]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([14568481364640261896, 13024973709310556877, 388942695468569842, 5165814615778255344, 5461422625881874726, 998729394980441256])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([2915405947247214443, 16476827183007826276, 397625141714225917, 7683936378748485792, 7647979677775928833, 1072503992333817859]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16050027345311255108, 10820284375719774599, 7851447608520829924, 16313935448547715969, 6657343977525303700, 224196233976581852])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16673765787484129697, 12992430675039131019, 2485465101194175871, 15715616086842934831, 1003336842210694614, 1058233574352048271]))
    ))), "VERIFICATION FAILED"
    );


    **account_from.try_borrow_mut_lamports().unwrap()   -= 1000000000; // 1 SOL

    **account_to.try_borrow_mut_lamports().unwrap()     += 1000000000;
    msg!("Final Exp successful");

    msg!("Transfer of 1 Sol successful to {:?}", account_to.key);

}
