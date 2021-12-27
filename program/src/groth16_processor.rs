use solana_program::{
    account_info::{next_account_info, AccountInfo},
    //msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,

};
use crate::pi_processor::_pi_254_process_instruction;
use crate::pi_state::PiBytes;
use crate::IX_ORDER;
use crate::_pre_process_instruction_miller_loop;
use crate::pi_pre_processor::CURRENT_INDEX_ARRAY;
use crate::pi_instructions;
pub struct Groth16Processor<'a, 'b> {
    main_account: &'a AccountInfo<'b>,
    current_instruction_index: usize,
}

use crate::ml_instructions::*;
use crate::ml_parsers::*;
use crate::ml_processor::*;
use crate::ml_ranges::*;
use crate::ml_state::*;
use crate::pi_state::*;


use crate::fe_processor::_process_instruction_final_exp;

use crate::fe_state::{FinalExpBytes};
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
            self.final_exponentiation();
            Ok(())
        }
        else {
            panic!("should not enter here");
            Ok(())
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
        _pi_254_process_instruction(
            IX_ORDER[current_instruction_index],
            &mut account_data,
            &vec![],
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
        main_account_data.current_instruction_index +=1;
        FinalExpBytes::pack_into_slice(&main_account_data, &mut self.main_account.data.borrow_mut());
        Ok(())
    }

}
