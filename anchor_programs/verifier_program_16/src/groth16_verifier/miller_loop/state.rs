#[allow(unused_imports)]
use crate::groth16_verifier::parsers::*;
use ark_ff::QuadExtField;
use std::cell::RefMut;

use crate::groth16_verifier::VerifierState;

#[derive(Debug)]
pub struct MillerLoopStateCompute {
    pub proof_b: ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
    pub pairs_0: [ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>; 3],
    pub r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>, //[u8;192],
    pub f: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    pub current_coeff: Option<(
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    )>,
}

impl MillerLoopStateCompute {
    pub fn new(tmp_account: &RefMut<'_, VerifierState>) -> Self {
        MillerLoopStateCompute {
            proof_b: parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec()),
            pairs_0: [
                parse_x_group_affine_from_bytes(&tmp_account.proof_a_bytes),
                parse_x_group_affine_from_bytes(&tmp_account.x_1_range),
                parse_x_group_affine_from_bytes(&tmp_account.proof_c_bytes),
            ],
            r: parse_r_from_bytes(&tmp_account.r_bytes.to_vec()),
            f: parse_f_from_bytes(&tmp_account.f_bytes.to_vec()), //<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero()
            current_coeff: MillerLoopStateCompute::unpack_coeff(
                tmp_account.current_coeff_bytes.to_vec(),
            ),
        }
    }

    pub fn pack(&self, tmp_account: &mut RefMut<'_, VerifierState>) {
        tmp_account.r_bytes = parse_r_to_bytes(self.r);
        tmp_account.f_bytes = parse_f_to_bytes(self.f);
        if self.current_coeff.is_some() {
            let mut tmp0 = vec![0u8; 64];
            let mut tmp1 = vec![0u8; 64];
            let mut tmp2 = vec![0u8; 64];

            parse_quad_to_bytes(self.current_coeff.unwrap().0, &mut tmp0);
            parse_quad_to_bytes(self.current_coeff.unwrap().1, &mut tmp1);
            parse_quad_to_bytes(self.current_coeff.unwrap().2, &mut tmp2);
            tmp_account.current_coeff_bytes = [tmp0, tmp1, tmp2].concat().try_into().unwrap();
        } else {
            tmp_account.current_coeff_bytes = [0u8; 192];
        }
    }
    fn unpack_coeff(
        coeffs: Vec<u8>,
    ) -> Option<(
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    )> {
        if coeffs != vec![0u8; 192] {
            Some((
                parse_quad_from_bytes(&coeffs[0..64].to_vec()),
                parse_quad_from_bytes(&coeffs[64..128].to_vec()),
                parse_quad_from_bytes(&coeffs[128..192].to_vec()),
            ))
        } else {
            None::<(
                QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
                QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
                QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
            )>
        }
    }
}
