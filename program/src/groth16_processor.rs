use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
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
        } else {
            panic!();
            Ok(())
        }
        /*
        //miller loop
        else if account_main_data.current_instruction_index >= 466 && account_main_data.current_instruction_index < 430+ 466 {
            msg!("else if _pre_process_instruction_miller_loop");
            _pre_process_instruction_miller_loop(&_instruction_data, accounts);
            Ok(())
        }
        //final exponentiation
        else if account_main_data.current_instruction_index >= 430 + 466  && account_main_data.current_instruction_index < 801 + 466{
            _pre_process_instruction_final_exp(program_id, accounts, &_instruction_data);
            Ok(())
        }*/

    }

    fn prepare_inputs(
        &mut self,
        // _instruction_data: &[u8],
        // accounts: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        let mut account_data = PiBytes::unpack(&self.main_account.data.borrow())?;
        //remove 40 from instruction array then remove this
        if account_data.current_instruction_index == 0 {
            account_data.current_instruction_index += 1;
            PiBytes::pack_into_slice(&account_data, &mut self.main_account.data.borrow_mut());
            return Ok(());
        }
        //let mut inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];
        msg!(
            "Executing instruction: {}",
            IX_ORDER[account_data.current_instruction_index]
        );

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

}
