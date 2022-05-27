#[allow(unused_imports)]

use crate::groth16_verifier::parsers::*;
use solana_program::msg;
use ark_ff::QuadExtField;
use std::cell::RefMut;
use anchor_lang::prelude::*;

use crate::groth16_verifier::prepare_inputs::VerifierState;

#[derive(Debug)]
pub struct MillerLoopStateCompute {
    pub proof_b:        ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
    pub pairs_0: [ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>;3],
    pub r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,//[u8;192],
    pub f: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    pub current_coeff: Option<(
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    )>
}

impl MillerLoopStateCompute {
    pub fn new(tmp_account: &RefMut<'_, VerifierState>)-> Self {
        MillerLoopStateCompute {
             proof_b: parse_proof_b_from_bytes(&tmp_account.proof_b_bytes.to_vec()),
             pairs_0: [
                parse_x_group_affine_from_bytes(&tmp_account.proof_a_bytes),
                parse_x_group_affine_from_bytes(&tmp_account.x_1_range),
                parse_x_group_affine_from_bytes(&tmp_account.proof_c_bytes),
             ],
             r: parse_r_from_bytes(&tmp_account.r_bytes.to_vec()),
             f: parse_f_from_bytes(&tmp_account.f_bytes.to_vec()),//<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero()
             current_coeff: MillerLoopStateCompute::unpack_coeff(tmp_account.current_coeff_bytes.to_vec()),
        }
    }

    pub fn pack(&self, tmp_account: &mut RefMut<'_, VerifierState>) {

        tmp_account.r_bytes = parse_r_to_bytes(self.r);
        tmp_account.f_bytes = parse_f_to_bytes(self.f);
        if self.current_coeff.is_some() {
            msg!("packing coeff");
            let mut tmp0 = vec![0u8;64];
            let mut tmp1 = vec![0u8;64];
            let mut tmp2 = vec![0u8;64];

            parse_quad_to_bytes(self.current_coeff.unwrap().0, &mut tmp0);
            parse_quad_to_bytes(self.current_coeff.unwrap().1, &mut tmp1);
            parse_quad_to_bytes(self.current_coeff.unwrap().2, &mut tmp2);
            tmp_account.current_coeff_bytes =[tmp0, tmp1, tmp2].concat().try_into().unwrap();
        } else {
            tmp_account.current_coeff_bytes = [0u8;192];
        }
    }
    fn unpack_coeff(coeffs: Vec<u8>) -> Option<(
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    )> {
        // msg!("coeffs: {:?}", coeffs);

        if coeffs != vec![0u8;192] {
            msg!("unpacking coeff");
            Some((
                parse_quad_from_bytes(&coeffs[0..64].to_vec()),
                parse_quad_from_bytes(&coeffs[64..128].to_vec()),
                parse_quad_from_bytes(&coeffs[128..192].to_vec()),
            ))
        } else {
            None::<(
                QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
                QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
                QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
            )>
        }
    }
}


#[derive(Debug)]
#[account(zero_copy)]
pub struct MillerLoopState {
    pub signing_address: Pubkey,
    pub proof_a_bytes:        [u8;64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    pub proof_b_bytes:        [u8;128],//ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
    pub proof_c_bytes:        [u8;64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    pub prepared_inputs_bytes: [u8;64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,

    // coeffs
    pub r_bytes: [u8;192],//ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,//[u8;192],
    pub q1_bytes: [u8;128],
    pub current_coeff_bytes: [u8;192],


    pub outer_first_loop_coeff: u64,
    pub outer_second_coeff: u64,
    pub inner_first_coeff: u64,

    // miller_loop
    pub f_bytes:  [u8;384],

    pub compute_max_miller_loop:  u64,
    pub outer_first_loop: u64,
    pub outer_second_loop: u64,
    pub outer_third_loop: u64,
    pub first_inner_loop_index: u64,
    pub second_inner_loop_index: u64,
    pub square_in_place_executed: u64,
    pub current_instruction_index:u64,
    pub coeff_index: [u8;3],
}
/*
impl MillerLoopState {
    pub fn new(
        proof_a_bytes:        [u8;64],
        proof_b_bytes:        [u8;128], //ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
        proof_c_bytes:        [u8;64],
        prepared_inputs_bytes: [u8;64],
        compute_max_miller_loop: u64,
    )-> Self {
        let proof_b = parse_proof_b_from_bytes(&proof_b_bytes.to_vec());
        let mut f = [0u8;384];
        f[0] = 1;
        MillerLoopState {
            signing_address: KeyPair::new().pubkey(),
            proof_a_bytes: proof_a_bytes,   //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            proof_b_bytes: proof_b_bytes,//[0;128],         //ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
            proof_c_bytes: proof_c_bytes,   //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            prepared_inputs_bytes: prepared_inputs_bytes, //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            f_bytes: f,
            r_bytes: parse_r_to_bytes(G2HomProjective {
                x: proof_b.x,
                y: proof_b.y,
                z: Fp2::one(),
            }),//ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,//[u8;192],
            q1_bytes: [0u8;128],
            coeff_index: [0;3],
            outer_first_loop_coeff: 0,
            outer_second_coeff: 0,
            inner_first_coeff: 0,
            compute_max_miller_loop:  compute_max_miller_loop,
            outer_first_loop: 0,
            outer_second_loop: 0,
            outer_third_loop: 0,
            first_inner_loop_index: 0,
            second_inner_loop_index: 0,
            square_in_place_executed: 0,
            current_instruction_index: 0,
        }
    }
}
*/
