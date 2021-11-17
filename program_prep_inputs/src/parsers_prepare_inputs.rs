
use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ff::{Fp384, Fp256};
use ark_ec;
use ark_bls12_381;
use ark_ed_on_bls12_381;
use ark_ff::fields::models::quadratic_extension::{QuadExtField, QuadExtParameters};
use num_traits::{One};


pub fn parse_fp256_to_bytes(fp256 : ark_ff::Fp256<ark_ed_on_bls12_381::FqParameters>, account: &mut Vec<u8>, range: [usize;2]){

    let start = range[0];
    let end = range[1];
    <Fp256::<ark_ed_on_bls12_381::FqParameters> as ToBytes>::write(&fp256, &mut account[start..end]);
}



pub fn parse_fp256_from_bytes(account: &Vec<u8>, range: [usize;2]) -> ark_ff::Fp256<ark_ed_on_bls12_381::FqParameters>{
    let fp256: ark_ff::Fp256<ark_ed_on_bls12_381::FqParameters>;
    let start = range[0];
    let end = range[1];
    fp256 = <Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&account[start..end]).unwrap();

    fp256
}


// ok ^




fn parse_fp384_to_bytes(fp384 : ark_ff::Fp384<ark_bls12_381::FqParameters>, account: &mut Vec<u8>, range: [usize;2]){

    let start = range[0];
    let end = range[1];
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&fp384, &mut account[start..end]);
}


fn parse_fp384_from_bytes(account: &Vec<u8>) -> ark_ff::Fp384<ark_bls12_381::FqParameters>{
    let fp384: ark_ff::Fp384<ark_bls12_381::FqParameters>;
    fp384 = <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&account[0..48]).unwrap();
    fp384
}




// x
pub fn parse_x_group_affine_from_bytes(account: &Vec<u8>) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters> {

    let x = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bls12_381::g1::Parameters>::new(
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&account[0..48]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&account[48..96]).unwrap(),
        false
    );
    x
}

pub fn parse_x_group_affine_to_bytes(x : ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>, account: &mut Vec<u8>){   
                    //println!("Parsing {:?}", c.c0);
    // parse_fp384_to_bytes(x.x, acc1, range1);
    // parse_fp384_to_bytes(x.y, acc2, range2); 
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&x.x, &mut account[0..48]);
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&x.y, &mut account[48..96]);

}






// res,g_ic

pub fn parse_group_projective_from_bytes(acc1: &Vec<u8>, acc2: &Vec<u8>, acc3: &Vec<u8>) -> ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bls12_381::g1::Parameters>{
    
    let res = ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bls12_381::g1::Parameters>::new(
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&acc1[0..48]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&acc2[0..48]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&acc3[0..48]).unwrap(),
    );
    res

}


pub fn parse_group_projective_to_bytes(res : ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bls12_381::g1::Parameters>, acc1: &mut Vec<u8>, acc2: &mut Vec<u8> ,acc3: &mut Vec<u8>){   
                    //println!("Parsing {:?}", c.c0);
    // parse_fp384_to_bytes(res.x, acc1, range1);
    // parse_fp384_to_bytes(res.y, acc2, range2); 
    // parse_fp384_to_bytes(res.z, acc3, range3); 

    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&res.x, &mut acc1[0..48]);
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&res.y, &mut acc2[0..48]);
    <Fp384::<ark_bls12_381::FqParameters> as ToBytes>::write(&res.z, &mut acc3[0..48]);

}
