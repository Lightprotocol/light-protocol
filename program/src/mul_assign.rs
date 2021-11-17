use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ec;
use crate::parsers_part_2::*;
use crate::ranges_part_2::*;
use solana_program::{
    msg,
    log::sol_log_compute_units,
};
pub fn mul_assign_1(
        _cubic: &Vec<u8>,
        _cubic_0_range: [usize;2],
        _cubic_other: &Vec<u8>,
        _range_other_c0: [usize; 2],
        _store_cubic: &mut Vec<u8>,
        _store_cubic_range: [usize; 2]
    ){
    let f_c0 = parse_cubic_from_bytes_new(&_cubic, _cubic_0_range);//30000
    //println!("str f_c0 = {:?}", f_c0);
    let other_c0 = parse_cubic_from_bytes_new(&_cubic_other, _range_other_c0);//30000
    //println!("str other_c0 = {:?}", other_c0);

    let v0 = f_c0 * &other_c0;//   cost 86548
    //println!("str v0 = {:?}", v0);
    parse_cubic_to_bytes_new(v0, _store_cubic, _store_cubic_range);//15000
    //total 162000
}

pub fn mul_assign_2(
    _cubic: &Vec<u8>,
    _cubic_1_range: [usize;2],
    _cubic_other: &Vec<u8>,
    range_other_c1: [usize; 2],
    _store_cubic: &mut Vec<u8>,
    _store_cubic_range: [usize; 2]) {
    let mut f_c1 = parse_cubic_from_bytes_new(&_cubic, _cubic_1_range);              //30000
    let other_c1 = parse_cubic_from_bytes_new(&_cubic_other, range_other_c1);      //30000
    let v1 = f_c1 * &other_c1;                                                //86827
    //println!("v1 = {:?}", v1);
    parse_cubic_to_bytes_new(v1, _store_cubic, _store_cubic_range);          //15000
    //total 193000
}

pub fn mul_assign_3(_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(&_f_range);              //60000
    f.c1 += &f.c0;                                                      //<1000
    parse_f_to_bytes_new(f, _f_range);                     //30000
    //total 93000
}

pub fn mul_assign_4(
        _cubic_1: &mut Vec<u8>,
        _cubic_1_range: [usize;2],
        _f_range_other: &Vec<u8>,

    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    let mut f_c1 = parse_cubic_from_bytes_new(&_cubic_1, _cubic_1_range); //30000
    let other = parse_f_from_bytes_new(&_f_range_other);          //60000

    //f.c1 += &f.c0;
    //
    //////msg!"4");
    f_c1 *= &(other.c0 + &other.c1);
                                              //87075
    parse_cubic_to_bytes_new(f_c1, _cubic_1, _cubic_1_range);           //15000
    //195000
}
pub fn mul_assign_4_1(
        _f_range_other: &Vec<u8>,
        _store_cubic_range: &mut Vec<u8>,
    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    //let mut f_c1 = parse_cubic_from_bytes(&account[..], _f1_r_cubic_1_range); //30000
    let other = parse_f_from_bytes_new(&_f_range_other);          //60000
    //println!("str Other res : {:?}", (other.c0 + other.c1));


    //f.c1 += &f.c0;
    //
    //////msg!"4");
    let tmp = &(other.c0 + &other.c1);                                           //87075
    parse_cubic_to_bytes_new(*tmp, _store_cubic_range, solo_cubic_0_range);           //15000
    //195000
}

pub fn mul_assign_4_2(
        _f1_r_range: &mut Vec<u8>,
        _f1_r_cubic_1_range: [usize;2],
        _store_cubic_range: &Vec<u8>
    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    let mut f_c1 = parse_cubic_from_bytes_new(&_f1_r_range, _f1_r_cubic_1_range); //30000
    let other_cs = parse_cubic_from_bytes_new(&_store_cubic_range, solo_cubic_0_range);          //60000
    //println!("str loaded other res : {:?}", other_cs);


    //f.c1 += &f.c0;
    //if these logs are removed a stackoverflow might occur in the overall program
    sol_log_compute_units();
    msg!("mulassing 4_2 0");
    f_c1 *= other_cs;
    sol_log_compute_units();
                              //87075
    parse_cubic_to_bytes_new(f_c1, _f1_r_range, _f1_r_cubic_1_range);
    msg!("mulassing 4_2 1");
    sol_log_compute_units();

    //assert_eq!(f_c1, parse_cubic_from_bytes_new(&_f1_r_range, _f1_r_cubic_1_range));           //15000
    //195000
}

pub fn mul_assign_5(
        _f1_r_range: &mut Vec<u8>,
        _cubic_range_0: &Vec<u8>,
        _cubic_range_1: &Vec<u8>
    ) {
    //--------------- additional split -------------------------
    let mut f = parse_f_from_bytes_new(&_f1_r_range);              //60000
    let v0 = parse_cubic_from_bytes_new(&_cubic_range_0, solo_cubic_0_range);       //30000
    let v1 = parse_cubic_from_bytes_new(&_cubic_range_1, solo_cubic_0_range);       //30000
    //////msg!"5");
    f.c1 -= &v0;                                                        //<1000
    //
    //////msg!"6");
    f.c1 -= &v1;                                                        //<1000
    //
    //println!("multi assign: {:?}", f);
    //////msg!"7");
    f.c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue(&v0, &v1); //1000

    parse_f_to_bytes_new(f, _f1_r_range);                     //30000
    //153000
}
