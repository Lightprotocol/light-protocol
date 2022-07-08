use crate::groth16_verifier::parsers::*;
use crate::groth16_verifier::VerifierState;
use crate::utils::prepared_verifying_key::ALPHA_G1_BETA_G2;
use ark_bn254::Parameters;
use ark_ec::{models::bn::Bn, PairingEngine};
use ark_ff::Field;
use ark_std::Zero;
use solana_program::msg;
use std::cell::RefMut;
pub const NAF_VEC: [i64; 63] = [
    1, 0, 0, 0, 1, 0, 1, 0, 0, -1, 0, 1, 0, 1, 0, -1, 0, 0, 1, 0, 1, 0, -1, 0, -1, 0, -1, 0, 1, 0,
    0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, -1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, -1, 0, 0,
    0, 1,
];

pub fn final_exponentiation_process_instruction(tmp_account: &mut RefMut<'_, VerifierState>) {
    // This function wraps the final_exponentiation implemented in the ark_ec crate.
    // The original implementation is split up into steps which can be executed within 1.4m
    // compute units. Every step has compute costs assigned to it which were collected through
    // measurements. Every steps increments a global total compute used variable which is checked
    // before every compute step.
    // The flow is as follows:
    //
    // Check if transaction was already executed
    // if state.fe_instruction_index == 0 {
    //     Increment current_compute,
    //     check whether enough compute is left to execute the step,
    //      if not stop the computation and safe the current state.
    //     state.current_compute += 288464;
    //     if !state.check_compute_units() {
    //         return Ok(Some(self.f));
    //     }
    //     Unpack variables necessary in this compute step.
    //     FinalExponentiationComputeState::unpack(
    //         &mut state.current_compute,
    //         &mut self.f,
    //         state.f_bytes,
    //     );
    //
    //     self.f = self.f.inverse().unwrap(); //.map(|mut f2| {
    //
    //     Mark the compute step as executed.
    //     state.fe_instruction_index += 1;

    // }
    let mut compute_state = FinalExponentiationComputeState::new_state();
    compute_state.final_exponentiation(tmp_account).unwrap();
    // Pack all variables which were used into bytes after compute is stopped.
    compute_state.pack(tmp_account);
}

pub struct FinalExponentiationComputeState {
    f: <Bn<Parameters> as PairingEngine>::Fqk,
    f1: <Bn<Parameters> as PairingEngine>::Fqk,
    f2: <Bn<Parameters> as PairingEngine>::Fqk,
    f3: <Bn<Parameters> as PairingEngine>::Fqk,
    f4: <Bn<Parameters> as PairingEngine>::Fqk,
    f5: <Bn<Parameters> as PairingEngine>::Fqk,
    i: <Bn<Parameters> as PairingEngine>::Fqk,
}
impl FinalExponentiationComputeState {
    pub fn new(state: &VerifierState) -> FinalExponentiationComputeState {
        let f = parse_f_from_bytes(&state.f_bytes.to_vec());
        let mut f1 = f.clone();
        f1.conjugate();
        FinalExponentiationComputeState {
            f: f,
            f1: f1,
            f2: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f3: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f4: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f5: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            i: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
        }
    }

    pub fn new_state() -> FinalExponentiationComputeState {
        FinalExponentiationComputeState {
            f: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f1: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f2: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f3: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f4: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f5: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            i: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
        }
    }

    pub fn reset(&mut self) -> FinalExponentiationComputeState {
        FinalExponentiationComputeState {
            f: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f1: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f2: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f3: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f4: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            f5: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
            i: <Bn<Parameters> as PairingEngine>::Fqk::zero(),
        }
    }

    pub fn pack(&self, state: &mut VerifierState) {
        if self.f != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes = parse_f_to_bytes(self.f);
        }

        if self.f1 != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes1 = parse_f_to_bytes(self.f1);
        }

        if self.f2 != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes2 = parse_f_to_bytes(self.f2);
        }

        if self.f3 != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes3 = parse_f_to_bytes(self.f3);
        }

        if self.f4 != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes4 = parse_f_to_bytes(self.f4);
        }

        if self.f5 != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.f_bytes5 = parse_f_to_bytes(self.f5);
        }

        if self.i != <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            state.i_bytes = parse_f_to_bytes(self.i);
        }
        state.current_compute = 0;
    }
    // Checks whether an Fqk is already unpacked if not unpacks it and increments the costs counter.
    pub fn unpack(
        current_compute: &mut u64,
        f: &mut <Bn<Parameters> as PairingEngine>::Fqk,
        f_bytes: [u8; 384],
    ) {
        if *f == <Bn<Parameters> as PairingEngine>::Fqk::zero() {
            *f = parse_f_from_bytes(&f_bytes.to_vec());
            // unpacking + packing
            *current_compute += 25268 + 14321;
        }
    }
    #[allow(clippy::let_and_return)]
    pub fn final_exponentiation(
        &mut self,
        state: &mut VerifierState,
    ) -> Result<Option<<Bn<Parameters> as PairingEngine>::Fqk>, ()> {
        // Easy part: result = elt^((q^6-1)*(q^2+1)).
        // Follows, e.g., Beuchat et al page 9, by computing result as follows:
        //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)

        // f1 = r.conjugate() = f^(p^6)
        //let mut f1 = *f;

        if state.fe_instruction_index == 0 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            state.current_compute += 288464;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f = self.f.inverse().unwrap(); //.map(|mut f2| {

            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 1 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            // f2 = f^(-1);
            // r = f^(p^6 - 1)
            self.f1 = self.f1 * self.f;

            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 2 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );
            // f2 = f^(p^6 - 1)
            self.f = self.f1;
            state.fe_instruction_index += 1;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
        }

        if state.fe_instruction_index == 3 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );
            state.current_compute += 54002;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            // r = f^((p^6 - 1)(p^2))
            self.f1.frobenius_map(2);
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 4 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );

            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            // r = f^((p^6 - 1)(p^2) + (p^6 - 1))
            // r = f^((p^6 - 1)(p^2 + 1))
            self.f1 *= self.f;
            state.fe_instruction_index += 1;
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
        if state.fe_instruction_index == 5 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );

            if !cyclotomic_exp(&self.f1, &mut self.f, state) {
                return Ok(Some(self.f));
            }

            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 6 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            state.current_compute += 46602;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f2 = self.f.cyclotomic_square();
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 7 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 46602;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f = self.f2.cyclotomic_square();
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 8 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f = self.f * &self.f2;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 9 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );

            if !cyclotomic_exp(&self.f, &mut self.f3, state) {
                return Ok(Some(self.f));
            }
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 10 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            state.current_compute += 46602;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f4 = self.f3.cyclotomic_square();
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 11 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f5,
                state.f_bytes5,
            );

            if !cyclotomic_exp(&self.f4.clone(), &mut self.f5, state) {
                return Ok(Some(self.f));
            }

            self.f4 = self.f5;
            self.f4.conjugate();
            state.fe_instruction_index += 1;
        }
        if state.fe_instruction_index == 12 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f4 = self.f4 * &self.f3;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 13 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f.conjugate();
            self.f4 = self.f4 * &self.f;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 14 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f2 = self.f4 * &self.f2;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 15 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f3 = self.f4 * &self.f3;
            state.fe_instruction_index += 1;
        }
        if state.fe_instruction_index == 16 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f3 = self.f3 * &self.f1;
            state.fe_instruction_index += 1;
        }
        if state.fe_instruction_index == 17 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 54002;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f = self.f2;
            self.f.frobenius_map(1);
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 18 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f,
                state.f_bytes,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f3 = self.f * &self.f3;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 19 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            state.current_compute += 54002;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }

            self.f4.frobenius_map(2);
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 20 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f3,
                state.f_bytes3,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f4 = self.f4 * &self.f3;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 21 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f1,
                state.f_bytes1,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f1.conjugate();

            self.f2 = self.f1 * &self.f2;
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 22 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 54002;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f2.frobenius_map(3);
            state.fe_instruction_index += 1;
        }

        if state.fe_instruction_index == 23 && state.check_compute_units() {
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f4,
                state.f_bytes4,
            );
            FinalExponentiationComputeState::unpack(
                &mut state.current_compute,
                &mut self.f2,
                state.f_bytes2,
            );
            state.current_compute += 125883;
            if !state.check_compute_units() {
                return Ok(Some(self.f));
            }
            self.f2 = self.f2 * &self.f4;

            assert_eq!(self.f2, parse_f_from_bytes(&ALPHA_G1_BETA_G2.to_vec()));
            msg!("Proof Verification success.");
            state.last_transaction = true;
            state.computing_final_exponentiation = false;
            state.fe_instruction_index += 1;
        }

        Ok(Some(self.f2))
    }
}
pub fn cyclotomic_exp(
    fe: &<Bn<Parameters> as PairingEngine>::Fqk,
    res: &mut <Bn<Parameters> as PairingEngine>::Fqk,
    state: &mut VerifierState,
) -> bool {
    if state.initialized == 0 {
        *res = fe.clone();
        // Mark the cyclotomic_exp as initialized.
        state.initialized = 1;
    }

    // let naf = crate::biginteger::arithmetic::find_wnaf(exponent.as_ref());

    // Skip first iteration for it is the assignment
    for i in (usize::try_from(state.outer_loop).unwrap())..63 {
        if !state.check_compute_units() {
            return false;
        }
        if state.cyclotomic_square_in_place == 0 {
            res.cyclotomic_square_in_place();

            // Mark cyclotomic_square_in_place as executed in this iteration.
            state.cyclotomic_square_in_place = 1;
            state.current_compute += 44606;
        }
        if !state.check_compute_units() {
            return false;
        }

        if NAF_VEC[i] != 0 {
            if NAF_VEC[i] > 0 {
                *res *= fe;
            } else {
                let mut self_inverse = fe.clone();
                self_inverse.conjugate();
                *res *= &self_inverse;
            }

            state.current_compute += 125883;
        }
        // Resect cyclotomic_square_in_place marker since the iteration is over.
        state.cyclotomic_square_in_place = 0;
        // Increment loop counter to start from the correct iteration in the next transaction.
        state.outer_loop += 1;
    }

    res.conjugate();
    // Reset the loop counter for the next loop.
    state.outer_loop = 1;
    // Reset initialized marker.
    state.initialized = 0;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groth16_verifier::parse_f_from_bytes;
    use crate::groth16_verifier::VerifierState;
    use crate::utils::prepared_verifying_key::ALPHA_G1_BETA_G2;
    use solana_program::pubkey::Pubkey;

    impl VerifierState {
        pub fn new(f: [u8; 384]) -> VerifierState {
            VerifierState {
                current_instruction_index: 0,
                signing_address: Pubkey::new(&[0; 32]),
                f_bytes: f,
                f_bytes1: [0; 384],
                f_bytes2: [0; 384],
                f_bytes3: [0; 384],
                f_bytes4: [0; 384],
                f_bytes5: [0; 384],
                i_bytes: [0; 384],
                fe_instruction_index: 0,
                fe_max_compute: 1_250_000,
                current_compute: 0,
                initialized: 0,
                outer_loop: 1,
                cyclotomic_square_in_place: 0,
                merkle_tree_tmp_account: Pubkey::new(&[0; 32]),
                relayer_fee: 0,
                recipient: Pubkey::new(&[0; 32]),
                amount: [0; 32],
                nullifier_hash: [0; 32],
                merkle_root: [0; 32],
                tx_integrity_hash: [0; 32], // is calculated on-chain from recipient, amount, signing_address,
                proof_a_bytes: [0; 64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
                proof_b_bytes: [0; 128], //ark_ec::models::bn::g2::G2Affine<Parameters>,
                proof_c_bytes: [0; 64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,

                ext_amount: [0; 8],
                fee: [0; 8],
                leaf_left: [0; 32],
                leaf_right: [0; 32],
                nullifier0: [0; 32],
                nullifier1: [0; 32],

                i_1_range: [0; 32],
                x_1_range: [0; 64],
                i_2_range: [0; 32],
                x_2_range: [0; 64],
                i_3_range: [0; 32],
                x_3_range: [0; 64],
                i_4_range: [0; 32],
                x_4_range: [0; 64],
                i_5_range: [0; 32],
                x_5_range: [0; 64],
                i_6_range: [0; 32],
                x_6_range: [0; 64],
                i_7_range: [0; 32],
                x_7_range: [0; 64],

                res_x_range: [0; 32],
                res_y_range: [0; 32],
                res_z_range: [0; 32],

                g_ic_x_range: [0; 32],
                g_ic_y_range: [0; 32],
                g_ic_z_range: [0; 32],
                current_index: 0,

                // miller loop
                r_bytes: [0; 192],
                q1_bytes: [0; 128],
                current_coeff_bytes: [0; 192],

                outer_first_loop_coeff: 0,
                outer_second_coeff: 0,
                inner_first_coeff: 0,

                ml_max_compute: 0,
                outer_first_loop: 0,
                outer_second_loop: 0,
                outer_third_loop: 0,
                first_inner_loop_index: 0,
                second_inner_loop_index: 0,
                square_in_place_executed: 0,

                coeff_index: [0; 3],

                computing_prepared_inputs: false, // 0 prepare inputs // 1 miller loop //
                computing_miller_loop: false,
                computing_final_exponentiation: true,

                merkle_tree_index: 0,
                found_root: 0,
                current_instruction_index_prepare_inputs: 0,
                encrypted_utxos: [0u8; 222],
                last_transaction: false,
                merkle_tree_instruction_index: 0,
                updating_merkle_tree: false,
            }
        }
    }
    #[test]
    pub fn test_final_exp() {
        let miller_loop_bytes = [
            211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214,
            234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88,
            99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79,
            122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200,
            148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189,
            144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110,
            19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254,
            99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23,
            134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199,
            155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182,
            0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222,
            148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239,
            16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126,
            27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15,
            92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100,
            25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15,
            33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199,
            189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178,
            112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68,
            40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152,
            100, 65, 101, 253, 2, 182, 145, 39,
        ];
        let mut state = VerifierState::new(miller_loop_bytes);
        let f = parse_f_from_bytes(&miller_loop_bytes.to_vec());
        let res_origin = <Bn<Parameters> as PairingEngine>::final_exponentiation(&f).unwrap();

        let mut compute_state = FinalExponentiationComputeState::new(&state);

        for _ in 0..600 {
            compute_state.final_exponentiation(&mut state).unwrap();
            // assert!(state.current_compute <= state.fe_max_compute);
            state.current_compute = 0;
            compute_state.pack(&mut state);
            compute_state.reset();
        }

        assert_eq!(res_origin, parse_f_from_bytes(&ALPHA_G1_BETA_G2.to_vec()));
        assert_eq!(state.f_bytes2, ALPHA_G1_BETA_G2);
    }

    /*
    exp_by_neg_x which is a private associated function
    if
    #[test]
    fn test_cyclotomic_exp() {
        let miller_loop_bytes = [211, 231, 132, 182, 211, 183, 85, 93, 214, 230, 240, 197, 144, 18, 159, 29, 215, 214, 234, 67, 95, 178, 102, 151, 20, 106, 95, 248, 19, 185, 138, 46, 143, 162, 146, 137, 88, 99, 10, 48, 115, 148, 32, 133, 73, 162, 157, 239, 70, 74, 182, 191, 122, 199, 89, 79, 122, 26, 156, 169, 142, 101, 134, 27, 116, 130, 173, 228, 156, 165, 45, 207, 206, 200, 148, 179, 174, 210, 104, 75, 22, 219, 230, 1, 172, 193, 58, 203, 119, 122, 244, 189, 144, 97, 253, 21, 24, 17, 92, 102, 160, 162, 55, 203, 215, 162, 166, 57, 183, 163, 110, 19, 84, 224, 156, 220, 31, 246, 113, 204, 202, 78, 139, 231, 119, 145, 166, 15, 254, 99, 20, 11, 81, 108, 205, 133, 90, 159, 19, 1, 34, 23, 154, 191, 145, 244, 200, 23, 134, 68, 115, 80, 204, 3, 103, 147, 138, 46, 209, 7, 193, 175, 158, 214, 181, 81, 199, 155, 0, 116, 245, 216, 123, 103, 158, 94, 223, 110, 67, 229, 241, 109, 206, 202, 182, 0, 198, 163, 38, 130, 46, 42, 171, 209, 162, 32, 94, 175, 225, 106, 236, 15, 175, 222, 148, 48, 109, 157, 249, 181, 178, 110, 7, 67, 62, 108, 161, 22, 95, 164, 182, 209, 239, 16, 20, 128, 5, 48, 243, 240, 178, 241, 163, 223, 28, 209, 150, 111, 200, 93, 251, 126, 27, 14, 104, 15, 53, 159, 130, 76, 192, 229, 243, 32, 108, 42, 0, 125, 241, 245, 15, 92, 208, 73, 181, 236, 35, 87, 26, 191, 179, 217, 219, 68, 92, 3, 192, 99, 197, 100, 25, 51, 99, 77, 230, 151, 200, 46, 246, 151, 83, 228, 105, 44, 4, 147, 182, 120, 15, 33, 135, 118, 63, 198, 244, 162, 237, 56, 207, 180, 150, 87, 97, 43, 82, 147, 14, 199, 189, 17, 217, 254, 191, 173, 73, 110, 84, 4, 131, 245, 240, 198, 22, 69, 2, 114, 178, 112, 239, 3, 86, 132, 221, 38, 217, 88, 59, 174, 221, 178, 108, 37, 46, 60, 51, 59, 68, 40, 207, 120, 174, 184, 227, 5, 91, 175, 145, 131, 36, 165, 197, 98, 135, 77, 53, 152, 100, 65, 101, 253, 2, 182, 145, 39];

        let f = parse_f_from_bytes(&miller_loop_bytes.to_vec());
        let mut state = VerifierState::new(miller_loop_bytes);
        let mut compute_state = FinalExponentiationComputeState::new(&state);

        for _ in 0..150 {
            cyclotomic_exp(&f, &mut compute_state.f1,&mut state);
            state.fe_instruction_index += state.current_compute;
            state.current_compute = 0;
            if state.first_exp_by_neg_x == 1 {break}

        }
        println!("fe_instruction_index: {:?}", state.fe_instruction_index);
        println!("\n\n-------------------------------\n\n");
        assert_eq!(compute_state.f1,Bn::<Parameters>::exp_by_neg_x(f));

    }*/
}
