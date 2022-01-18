use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
};

use crate::IX_ORDER;

use crate::Groth16_verifier::{
    parsers::*,
    prepare_inputs::{
        pi_instructions,
        pi_pre_processor::CURRENT_INDEX_ARRAY,
        pi_processor::_pi_process_instruction,
        pi_state::PiBytes,
        pi_ranges::*
    },
    miller_loop::{
        ml_processor::*,
        ml_ranges::*,
        ml_state::*,
    },
    final_exponentiation::{
        fe_state::{FinalExpBytes},
        fe_processor::_process_instruction_final_exp,
        fe_instructions::verify_result,
    }
};


use ark_ff::{Fp256, FromBytes};

use ark_ff::BigInteger256;


pub struct Groth16Processor<'a, 'b> {
    main_account: &'a AccountInfo<'b>,
    current_instruction_index: usize,
}
impl <'a, 'b> Groth16Processor <'a, 'b>{
    pub fn new(
        main_account: &'a AccountInfo<'b>,
        current_instruction_index: usize
        ) -> Result<Self, ProgramError>{

        Ok(Groth16Processor {
            main_account: main_account,
            current_instruction_index: current_instruction_index
        })

    }

    pub fn process_instruction_groth16_verifier(
            &mut self,
            //accounts: &[AccountInfo]
            ) -> Result<(),ProgramError> {

        if self.current_instruction_index < 466 {
            self.prepare_inputs()?;
            Ok(())
        } else if self.current_instruction_index >= 466 && self.current_instruction_index < 430+ 466 {
            self.miller_loop()?;
            Ok(())
        } else if self.current_instruction_index >= 430 + 466  && self.current_instruction_index < 801 + 466{
            self.final_exponentiation()?;
            Ok(())
        } else {
            msg!("should not enter here");
            Err(ProgramError::InvalidArgument)
        }
    }

    fn prepare_inputs(
        &mut self,
    ) -> Result<(), ProgramError> {
        let mut account_data = PiBytes::unpack(&self.main_account.data.borrow())?;
        //remove 40 from instruction array then remove this
        if account_data.current_instruction_index == 0 {
            account_data.current_instruction_index += 1;
            PiBytes::pack_into_slice(&account_data, &mut self.main_account.data.borrow_mut());
            return Ok(());
        }
        //let mut inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];
        // //msg!(
        //     "Executing instruction: {}",
        //     IX_ORDER[account_data.current_instruction_index]
        // );

        let current_instruction_index = account_data.current_instruction_index;
        _pi_process_instruction(
            IX_ORDER[current_instruction_index],
            &mut account_data,
            usize::from(CURRENT_INDEX_ARRAY[current_instruction_index - 1]),
        );

        account_data.current_instruction_index += 1;
        PiBytes::pack_into_slice(&account_data, &mut self.main_account.data.borrow_mut());
        //}
        Ok(())
    }

    fn miller_loop(
        &mut self,
        // _instruction_data: &[u8],
        // accounts: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        //msg!("entered _pre_process_instruction_miller_loop");

        let mut main_account_data = ML254Bytes::unpack(&self.main_account.data.borrow())?;
        // First ix: "0" -> Parses g_ic_affine from prepared_inputs.
        // Hardcoded for test purposes.
        if IX_ORDER[main_account_data.current_instruction_index] == 0 {
            // //msg!("parsing state from prepare inputs to ml");
            // //msg!("here0");
            let account_prepare_inputs_data = PiBytes::unpack(&self.main_account.data.borrow())?;

            let g_ic_affine = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.x_1_range); // 10k
            //msg!("here3");

            let p2: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
                ark_ec::bn::g1::G1Prepared::from(g_ic_affine);

            move_proofs(&mut main_account_data, &account_prepare_inputs_data);

            //msg!("here4");

            parse_fp256_to_bytes(p2.0.x, &mut main_account_data.p_2_x_range);
            //msg!("here5");
            parse_fp256_to_bytes(p2.0.y, &mut main_account_data.p_2_y_range);
            main_account_data.current_instruction_index += 1;

            main_account_data.changed_variables[P_2_Y_RANGE_INDEX] = true;
            main_account_data.changed_variables[P_2_X_RANGE_INDEX] = true;

            ML254Bytes::pack_into_slice(&main_account_data, &mut self.main_account.data.borrow_mut());
            //msg!("here6");
            return Ok(());
        } else {
            // Empty vecs that pass data from the client if called with respective ix.
            _process_instruction(
                IX_ORDER[main_account_data.current_instruction_index],
                &mut main_account_data,
            );
            main_account_data.current_instruction_index += 1;

            //msg!("packing");
            ML254Bytes::pack_into_slice(&main_account_data, &mut self.main_account.data.borrow_mut());
            //msg!("packed");
            Ok(())
        }
    }

    fn final_exponentiation(
        &mut self,
    ) -> Result<(), ProgramError> {
        let mut main_account_data = FinalExpBytes::unpack(&self.main_account.data.borrow())?;
        _process_instruction_final_exp(&mut main_account_data, IX_ORDER[self.current_instruction_index]);

        if self.current_instruction_index == 1266 {
            verify_result(&main_account_data)?;
        }
        main_account_data.current_instruction_index +=1;
        FinalExpBytes::pack_into_slice(&main_account_data, &mut self.main_account.data.borrow_mut());
        Ok(())
    }

    pub fn try_initialize(
        &mut self,
        _instruction_data: &[u8])
        -> Result<(), ProgramError>{
        let mut main_account_data = PiBytes::unpack(&self.main_account.data.borrow())?;

        let mut public_inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];
        msg!("_instruction_data: {:?}", _instruction_data[0..100].to_vec());
        // get public_inputs from _instruction_data.
        //root
        let input1 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[0..32],
        )
        .unwrap();
        //public amount
        let input2 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[32..64],
        )
        .unwrap();
        //external data hash
        //let input3 = Fp256::<ark_ed_on_bn254::FqParameters>::new(BigInteger256::new([0,0,0,0]));
        let input3 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[64..96],
        )
        .unwrap();

        //inputNullifier0
        let input4 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            & _instruction_data[96..128],
        )
        .unwrap();

        //inputNullifier1
        let input5 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[128..160],
        )
        .unwrap();
        //inputCommitment0
        let input6 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[160..192],
        )
        .unwrap();
        //inputCommitment1
        let input7 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
            &_instruction_data[192..224],
        )
        .unwrap();

        public_inputs = vec![input1, input2, input3, input4, input5, input6, input7];

        pi_instructions::init_pairs_instruction(
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
        );
        msg!("len _instruction_data{}", _instruction_data.len());
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
        PiBytes::pack_into_slice(&main_account_data, &mut self.main_account.data.borrow_mut());
        Ok(())
    }

}
