use crate::VerifierState;
use ark_bn254;
use ark_ec;
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::Fp256;
use ark_ff::One;
use std::cell::RefMut;

pub fn parse_f_to_bytes(
    f: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    // range: &mut Vec<u8>,
) -> [u8; 384] {
    let mut range = [0u8; 384];
    let mut iter = 0;
    for i in 0..2_u8 {
        for j in 0..3_u8 {
            for z in 0..2_u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c0.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c1.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c2.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 0 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c0.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c1.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c2.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c0.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c1.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c2.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 0 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c0.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c1.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c2.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    range
}

pub fn parse_f_from_bytes(
    range: &Vec<u8>,
) -> <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk {
    let mut iter = 0; // should be 0
    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    for i in 0..2_u8 {
        for j in 0..3_u8 {
            for z in 0..2_u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        f.c0.c0.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 1 && z == 0 {
                        f.c0.c1.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 2 && z == 0 {
                        f.c0.c2.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 0 && z == 1 {
                        f.c0.c0.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 1 && z == 1 {
                        f.c0.c1.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 2 && z == 1 {
                        f.c0.c2.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        f.c1.c0.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 1 && z == 0 {
                        f.c1.c1.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 2 && z == 0 {
                        f.c1.c2.c0 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 0 && z == 1 {
                        f.c1.c0.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 1 && z == 1 {
                        f.c1.c1.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    } else if j == 2 && z == 1 {
                        f.c1.c2.c1 =
                            <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter])
                                .unwrap();
                    }
                }
            }
        }
    }
    f
}

pub fn parse_fp256_to_bytes(fp256: ark_ff::Fp256<ark_bn254::FqParameters>, range: &mut [u8; 32]) {
    let start = 0;
    let end = 32;
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&fp256, &mut range[start..end]).unwrap();
}

pub fn parse_fp256_from_bytes(range: &[u8; 32]) -> ark_ff::Fp256<ark_bn254::FqParameters> {
    let fp256: ark_ff::Fp256<ark_bn254::FqParameters>;
    let start = 0;
    let end = 32;
    fp256 = <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[start..end]).unwrap();
    fp256
}

pub fn parse_fp256_ed_to_bytes(
    fp256: ark_ff::Fp256<ark_ed_on_bn254::FqParameters>,
    account: &mut Vec<u8>,
) {
    let start = 0;
    let end = 32;
    <Fp256<ark_ed_on_bn254::FqParameters> as ToBytes>::write(&fp256, &mut account[start..end])
        .unwrap();
}

pub fn parse_fp256_ed_from_bytes(
    account: &[u8; 32],
) -> ark_ff::Fp256<ark_ed_on_bn254::FqParameters> {
    let fp256: ark_ff::Fp256<ark_ed_on_bn254::FqParameters>;
    let start = 0;
    let end = 32;
    fp256 =
        <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&account[start..end]).unwrap();

    fp256
}

// j: proof.b prep
pub fn parse_r_to_bytes(
    r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,
    //range: &mut Vec<u8>,
) -> [u8; 192] {
    let mut tmp1 = vec![0u8; 64];
    let mut tmp2 = vec![0u8; 64];
    let mut tmp3 = vec![0u8; 64];
    parse_quad_to_bytes(r.x, &mut tmp1);
    parse_quad_to_bytes(r.y, &mut tmp2);
    parse_quad_to_bytes(r.z, &mut tmp3);
    [tmp1, tmp2, tmp3].concat().try_into().unwrap()
}

pub fn parse_r_from_bytes(
    range: &Vec<u8>,
) -> ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters> {
    ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
        x: parse_quad_from_bytes(&range[0..64].to_vec()),
        y: parse_quad_from_bytes(&range[64..128].to_vec()),
        z: parse_quad_from_bytes(&range[128..].to_vec()),
    }
}

pub fn parse_proof_b_from_bytes(
    range: &Vec<u8>,
) -> ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters> {
    ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
        parse_quad_from_bytes(&range[..64].to_vec()),
        parse_quad_from_bytes(&range[64..].to_vec()),
        false,
    )
}

pub fn parse_proof_b_to_bytes(
    proof: ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
    range: &mut Vec<u8>,
) {
    let mut tmp0 = vec![0u8; 64];
    let mut tmp1 = vec![0u8; 64];
    parse_quad_to_bytes(proof.x, &mut tmp0);
    parse_quad_to_bytes(proof.y, &mut tmp1);
    *range = [tmp0, tmp1].concat();
}

pub fn parse_quad_to_bytes(
    q: ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    range: &mut Vec<u8>,
) {
    let mut iter = 0;

    for z in 0..2_u8 {
        let tmp = iter;
        iter += 32;
        if z == 0 {
            <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&q.c0, &mut range[tmp..iter])
                .unwrap();
        } else if z == 1 {
            <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&q.c1, &mut range[tmp..iter])
                .unwrap();
        }
    }
}

pub fn parse_quad_from_bytes(
    range: &Vec<u8>,
) -> ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>> {
    let start = 0;
    let end = 64;
    let iter = start + 32;

    QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[start..iter]).unwrap(),
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[iter..end]).unwrap(),
    )
}

pub fn parse_cubic_to_bytes(
    c: ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>,
    range: &mut Vec<u8>,
) {
    let mut iter = 0;
    for j in 0..3_u8 {
        for z in 0..2_u8 {
            let tmp = iter;
            iter += 32;
            if j == 0 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c0.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 1 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c1.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 2 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c2.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 0 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c0.c1, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 1 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c1.c1, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 2 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c2.c1, &mut range[tmp..iter])
                    .unwrap();
            }
        }
    }
}

pub fn parse_cubic_from_bytes(
    range: &Vec<u8>,
) -> ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>> {
    let mut iter = 0;
    let mut cubic =
        ark_ff::CubicExtField::<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>::one();
    for j in 0..3_u8 {
        for z in 0..2_u8 {
            let tmp = iter;
            iter += 32;
            if j == 0 && z == 0 {
                cubic.c0.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 1 && z == 0 {
                cubic.c1.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 2 && z == 0 {
                cubic.c2.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 0 && z == 1 {
                cubic.c0.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 1 && z == 1 {
                cubic.c1.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 2 && z == 1 {
                cubic.c2.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            }
        }
    }
    cubic
}

pub fn parse_cubic_to_bytes_sub(
    c: ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>,
    range: &mut Vec<u8>,
    sub: [usize; 2],
) {
    let mut iter = sub[0];
    for j in 0..3_u8 {
        for z in 0..2_u8 {
            let tmp = iter;
            iter += 32;
            if j == 0 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c0.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 1 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c1.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 2 && z == 0 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c2.c0, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 0 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c0.c1, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 1 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c1.c1, &mut range[tmp..iter])
                    .unwrap();
            } else if j == 2 && z == 1 {
                <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&c.c2.c1, &mut range[tmp..iter])
                    .unwrap();
            }
        }
    }
}

pub fn parse_cubic_from_bytes_sub(
    range: &Vec<u8>,
    sub: [usize; 2],
) -> ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>> {
    let mut iter = sub[0];
    let mut cubic =
        ark_ff::CubicExtField::<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>::one();
    for j in 0..3_u8 {
        for z in 0..2_u8 {
            let tmp = iter;
            iter += 32;
            if j == 0 && z == 0 {
                cubic.c0.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 1 && z == 0 {
                cubic.c1.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 2 && z == 0 {
                cubic.c2.c0 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 0 && z == 1 {
                cubic.c0.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 1 && z == 1 {
                cubic.c1.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            } else if j == 2 && z == 1 {
                cubic.c2.c1 =
                    <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[tmp..iter]).unwrap();
            }
        }
    }
    cubic
}

// x
pub fn parse_x_group_affine_from_bytes(
    account: &[u8; 64],
) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> {
    ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[0..32]).unwrap(),
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[32..64]).unwrap(),
        false,
    )
}

pub fn parse_x_group_affine_to_bytes(
    x: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    account: &mut [u8; 64],
) {
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.x, &mut account[0..32]).unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.y, &mut account[32..64]).unwrap();
}
pub fn fill_x_ranges(
    x_vec: Vec<ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>>,
    tmp_account: &mut RefMut<'_, VerifierState>,
) {
    parse_x_group_affine_to_bytes(x_vec[0], &mut tmp_account.x_1_range);
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[1].x,
        &mut tmp_account.x_2_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[1].y,
        &mut tmp_account.x_2_range[32..64],
    )
    .unwrap();

    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[2].x,
        &mut tmp_account.x_3_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[2].y,
        &mut tmp_account.x_3_range[32..64],
    )
    .unwrap();

    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[3].x,
        &mut tmp_account.x_4_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[3].y,
        &mut tmp_account.x_4_range[32..64],
    )
    .unwrap();

    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[4].x,
        &mut tmp_account.x_5_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[4].y,
        &mut tmp_account.x_5_range[32..64],
    )
    .unwrap();

    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[5].x,
        &mut tmp_account.x_6_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[5].y,
        &mut tmp_account.x_6_range[32..64],
    )
    .unwrap();

    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[6].x,
        &mut tmp_account.x_7_range[0..32],
    )
    .unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
        &x_vec[6].y,
        &mut tmp_account.x_7_range[32..64],
    )
    .unwrap();
}
/*
pub fn parse_x_group_affine_to_bytes(
    x: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,

) -> [u8;64]{
    let mut account: [u8;64] = [0u8;64];
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.x, &mut account[0..32]).unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&x.y, &mut account[32..64]).unwrap();
    account
}
*/

pub fn parse_group_projective_from_bytes_254(
    acc1: &[u8; 32],
    acc2: &[u8; 32],
    acc3: &[u8; 32],
) -> ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> {
    ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc1[0..32]).unwrap(), // i 0..48
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc2[0..32]).unwrap(), // i 0..48
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&acc3[0..32]).unwrap(), // i 0..48
    )
}

pub fn parse_group_projective_to_bytes_254(
    res: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters>,
    acc1: &mut [u8; 32],
    acc2: &mut [u8; 32],
    acc3: &mut [u8; 32],
) {
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.x, &mut acc1[0..32]).unwrap(); // i 0..48
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.y, &mut acc2[0..32]).unwrap();
    <Fp256<ark_bn254::FqParameters> as ToBytes>::write(&res.z, &mut acc3[0..32]).unwrap();
    // i 0..48
}
