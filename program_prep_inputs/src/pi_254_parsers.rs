use ark_bn254;
use ark_ec;
use ark_ed_on_bn254;
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::Fp256;

pub fn parse_fp256_to_bytes_254(
    fp256: ark_ff::Fp256<ark_ed_on_bn254::FqParameters>,
    account: &mut Vec<u8>,
    range: [usize; 2],
) {
    let start = range[0];
    let end = range[1];
    <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(&fp256, &mut account[start..end]);
}

pub fn parse_fp256_from_bytes_254(
    account: &Vec<u8>,
    range: [usize; 2],
) -> ark_ff::Fp256<ark_ed_on_bn254::FqParameters> {
    let fp256: ark_ff::Fp256<ark_ed_on_bn254::FqParameters>;
    let start = range[0];
    let end = range[1];
    fp256 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&account[start..end]).unwrap();

    fp256
}

// x
pub fn parse_x_group_affine_from_bytes_254(
    account: &Vec<u8>,
) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> {
    println!("account length should be 64 bytes: {:?}", &account.len());
    let x = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[0..32]).unwrap(), // i 0..48
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[32..64]).unwrap(), // i 48..96
        false,
    );
    x
}

pub fn parse_x_group_affine_to_bytes_254(
    x: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    account: &mut Vec<u8>,
) {
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.x, &mut account[0..32]); // i 0..48
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.y, &mut account[32..64]);
    // i 48..96
}

// res,g_ic
pub fn parse_group_projective_from_bytes_254(
    acc1: &Vec<u8>,
    acc2: &Vec<u8>,
    acc3: &Vec<u8>,
) -> ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> {
    let res = ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc1[0..32]).unwrap(), // i 0..48
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc2[0..32]).unwrap(), // i 0..48
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc3[0..32]).unwrap(), // i 0..48
    );
    res
}

pub fn parse_group_projective_to_bytes_254(
    res: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters>,
    acc1: &mut Vec<u8>,
    acc2: &mut Vec<u8>,
    acc3: &mut Vec<u8>,
) {
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.x, &mut acc1[0..32]); // i 0..48
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.y, &mut acc2[0..32]);
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.z, &mut acc3[0..32]);
    // i 0..48
}
