use ark_ff::{Fp256, FromBytes};
// Solana
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack,
};

// Light
use crate::groth16_verifier::{
    final_exponentiation,
    final_exponentiation::{
        instructions::verify_result,
        ranges::{FINAL_EXPONENTIATION_END_INDEX, FINAL_EXPONENTIATION_START_INDEX},
        state::FinalExponentiationState,
    },
    miller_loop,
    miller_loop::{ranges::*, state::*},
    parsers::*,
    prepare_inputs,
    prepare_inputs::{processor::CURRENT_INDEX_ARRAY, ranges::*, state::PrepareInputsState},
};
use crate::IX_ORDER;

pub struct Groth16Processor<'a, 'b> {
    main_account: &'a AccountInfo<'b>,
    current_instruction_index: usize,
}
impl<'a, 'b> Groth16Processor<'a, 'b> {
    pub fn new(
        main_account: &'a AccountInfo<'b>,
        current_instruction_index: usize,
    ) -> Result<Self, ProgramError> {
        Ok(Groth16Processor {
            main_account,
            current_instruction_index,
        })
    }
    // The groth16 verifier verifies proofs for the Groth16 zkSNARK construction that's used by Light Protocol.
    // This implements the ark-groth16 verifier in a way that can be executed by the Solana runtime.
    // As such, it's mostly broken up into many smaller computation pieces that each fit into a single instruction's compute budget.
    // The current implemenation relies on a 200k compute budget ix-wide. With that, the Groth16 processor currently processes
    // 1k+ ix calls for a single proof verification. The call order is hardcoded on-chain as [IX_ORDER].
    // There are some caveats that come with maintaining state across all those instructions, hence the increased code complexity.

    pub fn process_instruction_groth16_verifier(&mut self) -> Result<(), ProgramError> {
        if self.current_instruction_index < PREPARE_INPUTS_END_INDEX {
            self.prepare_inputs()?;
            Ok(())
        } else if self.current_instruction_index >= MILLER_LOOP_START_INDEX
            && self.current_instruction_index < MILLER_LOOP_END_INDEX
        {
            self.miller_loop()?;
            Ok(())
        } else if self.current_instruction_index >= FINAL_EXPONENTIATION_START_INDEX
            && self.current_instruction_index < FINAL_EXPONENTIATION_END_INDEX
        {
            self.final_exponentiation()?;
            Ok(())
        } else {
            msg!("should not enter here");
            Err(ProgramError::InvalidArgument)
        }
    }

    // Implements prepare_inputs as per: https://docs.rs/ark-groth16/0.3.0/src/ark_groth16/verifier.rs.html#20-36
    // in a way that can be executed by the solana runtime.
    fn prepare_inputs(&mut self) -> Result<(), ProgramError> {
        let mut account_data = PrepareInputsState::unpack(&self.main_account.data.borrow())?;

        let current_instruction_index = account_data.current_instruction_index;
        prepare_inputs::processor::_process_instruction(
            IX_ORDER[current_instruction_index],
            &mut account_data,
            usize::from(CURRENT_INDEX_ARRAY[current_instruction_index - 1]),
        );

        account_data.current_instruction_index += 1;
        PrepareInputsState::pack_into_slice(
            &account_data,
            &mut self.main_account.data.borrow_mut(),
        );
        Ok(())
    }

    // Implements miller_loop as per: https://docs.rs/ark-ec/latest/src/ark_ec/models/bn/mod.rs.html#85-148
    // in a way that it can be executed by the solana runtime.
    // We need to create G1,G2 pairs onchain.
    // The structure of those pairs is as follows:
    // (G1,G2) --> (p1,coeff1) --> (proof.a, proof.b)
    // (G1,G2) --> (p2,coeff2) --> (prepare_inputs(public_inputs), pvk.gamma_g2_neg_pc)
    // (G1,G2) --> (p3,coeff3) --> (proof.c), pvk.delta_g2_neg_pc)
    // and a single G2 look like this:
    // G2 --> (coeff0, coeff1,...coeff90)
    // coeff0 --> (c.0,c.1,c.3)
    // proof.b must be transformed into a G2. This transformation is performed in separate parts,
    // every part (coeff) is computed when it is used, to minimize memory use.
    // (transformation nstruction ids "doubling_step" (ix 7) or "addition_step" (ix 8 or 9 or 10 or 11))
    // All coeffs in pvk.gamma_g2_neg_pc and pvk.delta_g2_neg_pc are hardcoded in this program,
    // and are obtained with getter functions from utils/prepared_verifying_key.rs
    //
    // If you look closely at
    // the actual miller_loop implementation here: https://docs.rs/ark-ec/latest/src/ark_ec/models/bn/mod.rs.html#97-148
    // You find that it loop through each (G1,G2) pair serially and
    // with each loop it takes the same G1 value + the next G2 value out of 91 total coeff triples per (G1,G2) pair.
    // It then takes those values and calls the "ell" computation: https://docs.rs/ark-ec/latest/src/ark_ec/models/bn/mod.rs.html#57-74
    fn miller_loop(&mut self) -> Result<(), ProgramError> {
        let mut main_account_data = MillerLoopState::unpack(&self.main_account.data.borrow())?;
        // First ix (0): Parses g_ic_affine(proof.b) and more from prepared_inputs state to miller_loop state.
        if IX_ORDER[main_account_data.current_instruction_index] == 0 {
            let account_prepare_inputs_data =
                PrepareInputsState::unpack(&self.main_account.data.borrow())?;
            let g_ic_affine =
                parse_x_group_affine_from_bytes(&account_prepare_inputs_data.x_1_range); // 10k
            let p2: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
                ark_ec::bn::g1::G1Prepared::from(g_ic_affine);
            miller_loop::processor::move_proofs(
                &mut main_account_data,
                &account_prepare_inputs_data,
            );

            parse_fp256_to_bytes(p2.0.x, &mut main_account_data.p_2_x_range);
            parse_fp256_to_bytes(p2.0.y, &mut main_account_data.p_2_y_range);
            main_account_data.current_instruction_index += 1;

            // Partial pack to save compute budget.
            main_account_data.changed_variables[P_2_Y_RANGE_INDEX] = true;
            main_account_data.changed_variables[P_2_X_RANGE_INDEX] = true;
            MillerLoopState::pack_into_slice(
                &main_account_data,
                &mut self.main_account.data.borrow_mut(),
            );
            Ok(())
        } else {
            // main processor after 1st ix (0).

            miller_loop::processor::_process_instruction(
                IX_ORDER[main_account_data.current_instruction_index],
                &mut main_account_data,
            );
            main_account_data.current_instruction_index += 1;

            MillerLoopState::pack_into_slice(
                &main_account_data,
                &mut self.main_account.data.borrow_mut(),
            );
            Ok(())
        }
    }

    fn final_exponentiation(&mut self) -> Result<(), ProgramError> {
        let mut main_account_data =
            FinalExponentiationState::unpack(&self.main_account.data.borrow())?;
        final_exponentiation::processor::_process_instruction(
            &mut main_account_data,
            IX_ORDER[self.current_instruction_index],
        )?;

        if self.current_instruction_index == FINAL_EXPONENTIATION_END_INDEX - 1 {
            verify_result(&main_account_data)?;
        }
        main_account_data.current_instruction_index += 1;
        FinalExponentiationState::pack_into_slice(
            &main_account_data,
            &mut self.main_account.data.borrow_mut(),
        );
        Ok(())
    }

    pub fn try_initialize(&mut self, _instruction_data: &[u8]) -> Result<(), ProgramError> {
        let mut main_account_data = PrepareInputsState::unpack(&self.main_account.data.borrow())?;

        // get public_inputs from _instruction_data.
        //root
        let input1 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[0..32])
                .unwrap();
        //public amount
        let input2 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[32..64])
                .unwrap();
        //external data hash
        let input3 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[64..96])
                .unwrap();
        //inputNullifier0
        let input4 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[96..128])
                .unwrap();

        //inputNullifier1
        let input5 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[128..160])
                .unwrap();
        //inputCommitment0
        let input6 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[160..192])
                .unwrap();
        //inputCommitment1
        let input7 =
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&_instruction_data[192..224])
                .unwrap();

        let public_inputs: Vec<Fp256<ark_bn254::FrParameters>> =
            vec![input1, input2, input3, input4, input5, input6, input7];

        // Initialize prepare inputs
        prepare_inputs::instructions::init_pairs_instruction(
            &public_inputs,
            &mut main_account_data.i_1_range,
            &mut main_account_data.x_1_range,
            &mut main_account_data.i_2_range,
            &mut main_account_data.x_2_range,
            &mut main_account_data.i_3_range,
            &mut main_account_data.x_3_range,
            &mut main_account_data.i_4_range,
            &mut main_account_data.x_4_range,
            &mut main_account_data.i_5_range,
            &mut main_account_data.x_5_range,
            &mut main_account_data.i_6_range,
            &mut main_account_data.x_6_range,
            &mut main_account_data.i_7_range,
            &mut main_account_data.x_7_range,
            &mut main_account_data.g_ic_x_range,
            &mut main_account_data.g_ic_y_range,
            &mut main_account_data.g_ic_z_range,
        )?;
        let indices: [usize; 17] = [
            I_1_RANGE_INDEX,
            X_1_RANGE_INDEX,
            I_2_RANGE_INDEX,
            X_2_RANGE_INDEX,
            I_3_RANGE_INDEX,
            X_3_RANGE_INDEX,
            I_4_RANGE_INDEX,
            X_4_RANGE_INDEX,
            I_5_RANGE_INDEX,
            X_5_RANGE_INDEX,
            I_6_RANGE_INDEX,
            X_6_RANGE_INDEX,
            I_7_RANGE_INDEX,
            X_7_RANGE_INDEX,
            G_IC_X_RANGE_INDEX,
            G_IC_Y_RANGE_INDEX,
            G_IC_Z_RANGE_INDEX,
        ];
        for i in indices.iter() {
            main_account_data.changed_variables[*i] = true;
        }
        PrepareInputsState::pack_into_slice(
            &main_account_data,
            &mut self.main_account.data.borrow_mut(),
        );
        Ok(())
    }
}
