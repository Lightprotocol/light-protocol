use solana_program::log::sol_log_compute_units;

use solana_program::{msg, program_error::ProgramError};
use crate::utils::prepared_verifying_key::ALPHA_G1_BETA_G2;
use crate::groth16_verifier::parsers::*;
use crate::groth16_verifier::parse_f_from_bytes;
use crate::groth16_verifier::FinalExponentiationState;
use std::cell::RefMut;
use ark_ec;
use ark_ff::{
    fields::models::{
        cubic_extension::CubicExtParameters,
        quadratic_extension::{QuadExtField, QuadExtParameters},
    },
    Field,
};
use ark_std::Zero;
pub const NAF_VEC: [i64; 63] = [
    1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1, 0,
    0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1, 0, 0,
    0, 1,
];


pub fn final_exponentiation_process_instruction(
    tmp_account: &mut RefMut<'_, FinalExponentiationState>,
) {

    let mut compute_state = FinalExponentiationComputeState::new_state();

    compute_state.final_exponentiation(tmp_account);
    // assert!(tmp_account.current_compute <= tmp_account.max_compute);
    compute_state.pack(tmp_account);

}

pub struct FinalExponentiationComputeState {
    f:  <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    f1: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    f2: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    f3: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    f4: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    f5: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    i: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,

}
impl FinalExponentiationComputeState {
    pub fn new(state: &FinalExponentiationState ) -> FinalExponentiationComputeState {
        let f = parse_f_from_bytes(&state.f.to_vec());
        let mut f1 = f.clone();
        f1.conjugate();
        FinalExponentiationComputeState {
            f:  f,
            f1: f1,
            f2: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f3: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f4: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f5: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            i: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),

        }
    }

    pub fn new_state() -> FinalExponentiationComputeState {
        FinalExponentiationComputeState {
            f:  <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f1: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f2: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f3: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f4: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f5: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            i: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),

        }
    }

    pub fn reset(&mut self) -> FinalExponentiationComputeState {
        FinalExponentiationComputeState {
            f:  <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f1: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f2: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f3: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f4: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            f5: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),
            i: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero(),

        }
    }

    pub fn pack(&self, state: &mut FinalExponentiationState) {
        if self.f != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // sol_log_compute_units();
            // msg!("packing f {:?}", self.f);
            state.f = parse_f_to_bytes(self.f);
            // sol_log_compute_units();
        }

        if self.f1 != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing f1 {:?}", self.f1);
            state.f1 = parse_f_to_bytes(self.f1);
        }

        if self.f2 != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing f2 {:?}", self.f2);
            state.f2 = parse_f_to_bytes(self.f2);
        }

        if self.f3 != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing f3 {:?}", self.f3);
            state.f3 = parse_f_to_bytes(self.f3);
        }

        if self.f4 != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing f4 {:?}", self.f4);
            state.f4 = parse_f_to_bytes(self.f4);
        }

        if self.f5 != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing f5 {:?}", self.f5);
            state.f5 = parse_f_to_bytes(self.f5);
        }

        if self.i != <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
            // msg!("packing i {:?}", self.i);
            state.i = parse_f_to_bytes(self.i);
        }
        state.current_compute = 0;
    }

    pub fn unpack(current_compute: &mut u64, f: &mut <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
            f_bytes: [u8;384]
        ) {
            // sol_log_compute_units();
            // msg!("unpacking f");
            if *f == <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::zero() {
                *f = parse_f_from_bytes(&f_bytes.to_vec());
                // unpacking + packing
                *current_compute+=25268 + 14321;
            }
            // sol_log_compute_units();
        }
        #[allow(clippy::let_and_return)]
        pub fn final_exponentiation(
            &mut self,
            state: &mut FinalExponentiationState
        ) -> Result<Option<<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk>, ()> {
            // Easy part: result = elt^((q^6-1)*(q^2+1)).
            // Follows, e.g., Beuchat et al page 9, by computing result as follows:
            //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)

            // f1 = r.conjugate() = f^(p^6)
            //let mut f1 = *f;

            if state.current_instruction_index == 0 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                // sol_log_compute_units();
                // msg!("inverse");
                self.f = self.f.inverse().unwrap();//.map(|mut f2| {
                // sol_log_compute_units();

                state.current_compute+=288464;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }

            }


            if state.current_instruction_index == 1 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);

                // f2 = f^(-1);
                // r = f^(p^6 - 1)
                // sol_log_compute_units();
                // msg!("mul");
                self.f1 = self.f1 * self.f;
                // sol_log_compute_units();

                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }



            if state.current_instruction_index == 2 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);
                // f2 = f^(p^6 - 1)
                self.f = self.f1;
                // state.current_compute+=1;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }



            if state.current_instruction_index == 3 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);
                // r = f^((p^6 - 1)(p^2))
                // sol_log_compute_units();
                // msg!("frobenius_map");
                self.f1.frobenius_map(2);
                // sol_log_compute_units();

                state.current_compute+=54002;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }


            if state.current_instruction_index == 4 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);

                // r = f^((p^6 - 1)(p^2) + (p^6 - 1))
                // r = f^((p^6 - 1)(p^2 + 1))
                self.f1 *= self.f;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }



            // Hard part follows Laura Fuentes-Castaneda et al. "Faster hashing to G2"
            // by computing:
            //
            // result = elt^(q^3 * (12*z^3 + 6z^2 + 4z - 1) +
            //               q^2 * (12*z^3 + 6z^2 + 6z) +
            //               q   * (12*z^3 + 6z^2 + 4z) +
            //               1   * (12*z^3 + 12z^2 + 6z + 1))
            // which equals
            //
            // result = elt^( 2z * ( 6z^2 + 3z + 1 ) * (q^4 - q^2 + 1)/r )
            if state.current_instruction_index == 5 {
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);
                // msg!("state.current_instruction_index {}", state.current_instruction_index);

                if !cyclotomic_exp(&self.f1, &mut self.f,state) {
                    // msg!("cyclotomic_exp" );
                    return Ok(Some(self.f));
                }
                state.current_instruction_index +=1;

            }

            if state.current_instruction_index == 6 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);

                self.f2 = self.f.cyclotomic_square();
                state.current_compute+=46602;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }



            if state.current_instruction_index == 7 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);
                // sol_log_compute_units();
                // msg!("cyclotomic_square");
                self.f = self.f2.cyclotomic_square();
                // sol_log_compute_units();

                state.current_compute+=46602;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }
            // msg!("7self.f {:?}", self.f);


            if state.current_instruction_index == 8 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f = self.f * &self.f2;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }



            if state.current_instruction_index == 9 {
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);
                // msg!("state.current_instruction_index {}", state.current_instruction_index);

                if !cyclotomic_exp(&self.f, &mut self.f3,state) {
                    // msg!("cyclotomic_exp" );
                    return Ok(Some(self.f));
                }
                state.current_instruction_index +=1;

            }

            if state.current_instruction_index == 10 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);

                self.f4 = self.f3.cyclotomic_square();
                state.current_compute+=46602;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 11 {
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f5, state.f5);
                // msg!("state.current_instruction_index {}", state.current_instruction_index);
                if !cyclotomic_exp(&self.f4.clone(), &mut self.f5,state) {
                    // msg!("cyclotomic_exp" );
                    return Ok(Some(self.f));
                }

                state.current_instruction_index +=1;
                self.f4 = self.f5;
                self.f4.conjugate();
            }
            if state.current_instruction_index == 12 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);

                self.f4 = self.f4 * &self.f3;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 13 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                self.f.conjugate();

                self.f4 = self.f4 * &self.f;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 14 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f2 = self.f4 * &self.f2;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                // msg!("self.f2{:?}", self.f2);

                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }
            // msg!("14self.f2 {:?}", self.f2);

            if state.current_instruction_index == 15 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);

                self.f3 = self.f4 * &self.f3;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }
            if state.current_instruction_index == 16 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);

                self.f3 = self.f3 * &self.f1;
                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }
            if state.current_instruction_index == 17 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f = self.f2;
                self.f.frobenius_map(1);

                state.current_compute+=54002;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 18 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f, state.f);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);

                self.f3 = self.f * &self.f3;

                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 19 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);

                self.f4.frobenius_map(2);

                state.current_compute+=54002;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 20 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f3, state.f3);

                self.f4 = self.f4 * &self.f3;

                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }
            // msg!("20self.f4 {:?}", self.f4);
            if state.current_instruction_index == 21 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f1, state.f1);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f1.conjugate();

                self.f2 = self.f1 * &self.f2;

                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 22 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f2.frobenius_map(3);

                state.current_compute+=54002;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            if state.current_instruction_index == 23 && state.check_compute_units(){
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f4, state.f4);
                FinalExponentiationComputeState::unpack( &mut state.current_compute, &mut self.f2, state.f2);

                self.f2 = self.f2 * &self.f4;

                assert_eq!(self.f2,  parse_f_from_bytes(&ALPHA_G1_BETA_G2.to_vec()));

                state.current_compute+=125883;
                state.current_instruction_index+=1;
                if !state.check_compute_units() {
                    return Ok(Some(self.f));
                }
            }

            Ok(Some(self.f2))
            //})
        }


}
pub fn cyclotomic_exp(
    fe: &<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    res: &mut <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    state: &mut FinalExponentiationState
) -> bool {
    if state.initialized == 0 {
        *res = fe.clone();
        // msg!("initialized {:?}", res);

        state.initialized +=1;
    }

    // let naf = crate::biginteger::arithmetic::find_wnaf(exponent.as_ref());

    // skip first iteration for it is the assignment
    for i in (state.outer_loop as usize)..63 {
        if !state.check_compute_units() {
            return false;
        }
        if state.cyclotomic_square_in_place == 0 {
            // sol_log_compute_units();
            // msg!("cyclotomic_square_in_place");
            res.cyclotomic_square_in_place();
            // sol_log_compute_units();

            state.cyclotomic_square_in_place = 1;
            state.current_compute+=44606;
        }
        if !state.check_compute_units() {
            return false;
        }
        // msg!("beferoe naf");
        // // msg!("res {:?}", res);
        if NAF_VEC[i] != 0 {
            // msg!("naf {}", NAF_VEC[i]);

            if NAF_VEC[i] > 0 {
                *res *= fe;
            } else {
                let mut self_inverse = fe.clone();
                self_inverse.conjugate();
                *res *= &self_inverse;
            }

            state.current_compute+=125883;

        }
        state.cyclotomic_square_in_place = 0;

        state.outer_loop +=1;

    }

    res.conjugate();

    state.outer_loop = 1;
    state.first_exp_by_neg_x = 1;
    state.initialized =0;
    true
}


#[cfg(test)]
mod tests {
     use super::*;
    // use crate::groth16_verifier::{
    //     final_exponentiation::{ranges::*, state::FinalExponentiationState},
    //     parsers::{
    //         parse_cubic_from_bytes_sub, parse_cubic_to_bytes_sub, parse_f_from_bytes, parse_f_to_bytes,
    //         parse_fp256_from_bytes, parse_fp256_to_bytes, parse_quad_from_bytes, parse_quad_to_bytes,
    //     },
    // };
    use crate::utils::prepared_verifying_key::ALPHA_G1_BETA_G2;
    use crate::groth16_verifier::parsers::*;
    use crate::groth16_verifier::final_exponentiation::processor::*;
    use crate::groth16_verifier::final_exponentiation::state::*;

    use crate::groth16_verifier::parse_f_from_bytes;
    use ark_ec;
    use ark_ff::{
        fields::models::{
            cubic_extension::CubicExtParameters,
            quadratic_extension::{QuadExtField, QuadExtParameters},
        },
        Field,
    };
    use ark_std::Zero;
    pub const NAF_VEC: [i64; 63] = [
        1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1, 0,
        0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1, 0, 0,
        0, 1,
    ];

    #[test]
    pub fn test_final_exp() {
        let miller_loop_bytes = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];
        let mut state = FinalExponentiationState::new(miller_loop_bytes);
        let f = parse_f_from_bytes(&miller_loop_bytes.to_vec());
        let res_origin = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&f).unwrap();
        let mut res_custom;
        let mut compute_state = FinalExponentiationComputeState::new(&state);

        for i in 0..600 {
            res_custom = compute_state.final_exponentiation(&mut state);
            // assert!(state.current_compute <= state.max_compute);
            state.current_compute = 0;
            compute_state.pack(&mut state);
            compute_state.reset();
        }

        assert_eq!(res_origin,  parse_f_from_bytes(&ALPHA_G1_BETA_G2.to_vec()));
        assert_eq!(state.f2,  ALPHA_G1_BETA_G2);
        // assert_eq!(state.current_instruction_index, 5);

    }

    pub fn exp_by_neg_x(
        mut f: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    ) -> <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk {
        f = f.cyclotomic_exp(&<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X);
        if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
            println!("conjugate");
            f.conjugate();
        }
        f
    }

    #[test]
    fn test_cyclotomic_exp() {
        let miller_loop_bytes = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];

        let f = parse_f_from_bytes(&miller_loop_bytes.to_vec());
        let mut state = FinalExponentiationState::new(miller_loop_bytes);
        let mut compute_state = FinalExponentiationComputeState::new(&state);

        for i in 0..150 {
            cyclotomic_exp(&f, &mut compute_state.f1,&mut state);
            state.current_instruction_index += state.current_compute;
            state.current_compute = 0;
            if state.first_exp_by_neg_x == 1 {break}

        }
        println!("current_instruction_index: {:?}", state.current_instruction_index);
        println!("\n\n-------------------------------\n\n");
        assert_eq!(compute_state.f1,ark_ec::models::bn::Bn::<ark_bn254::Parameters>::exp_by_neg_x(f));

    }

}
