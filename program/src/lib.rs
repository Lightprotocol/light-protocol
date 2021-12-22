pub mod instructions;
pub mod instructions_transform_g2_affine_to_g2_prepared;
pub mod ml_254_instructions;
pub mod ml_254_instructions_transform;
pub mod ml_254_parsers;
pub mod ml_254_pre_processor;
pub mod ml_254_processor;
pub mod ml_254_ranges;
pub mod ml_254_state;
pub mod parsers;
pub mod parsers_prepare_inputs;
pub mod pre_processor_final_exp;
pub mod ranges_part_2;
pub mod state_check_nullifier;
pub mod state_final_exp;

pub mod instructions_poseidon;
pub mod poseidon_round_constants_split;

pub mod state_miller_loop_transfer;

pub mod hard_coded_verifying_key_pvk_254;

pub mod init_bytes11;
pub mod init_bytes18;
pub mod instructions_merkle_tree;
pub mod processor_merkle_tree;
pub mod state_merkle_tree;

pub mod instructions_final_exponentiation;
pub mod parsers_part_2_254;
pub mod processor_final_exp;
use crate::pre_processor_final_exp::_pre_process_instruction_final_exp;
use crate::processor_merkle_tree::_pre_process_instruction_merkle_tree;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod state_miller_loop;
//mod pi_254_test;
use crate::ml_254_pre_processor::*;
//use crate::ml_254_state::ML254Bytes;
use crate::state_final_exp::InstructionIndex;
use solana_program::program_pack::Pack;


pub mod pi_instructions;
pub mod pi_254_parsers;
pub mod pi_processor;
pub mod pi_ranges;
pub mod pi_state;
pub mod pi_pre_processor;
use crate::pi_pre_processor::_pre_process_instruction;
pub mod state_merkle_tree_roots;
//pub mod verifyingkey_254_hc;

use crate::pi_ranges::*;
use ark_ff::{Fp256, FromBytes};
use crate::pi_state::PiBytes;


pub mod li_pre_processor;
use crate::li_pre_processor::li_pre_process_instruction;



entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    //msg!("instruction_data len: {}", &_instruction_data.len());
    // initialize new merkle tree account
    if _instruction_data.len() >= 9 && _instruction_data[8] == 240 {
        _pre_process_instruction_merkle_tree(&_instruction_data, accounts);
    }
    // transact with shielded pool
    else {

        let accounts_mut = accounts.clone();
        let account = &mut accounts_mut.iter();
        let signing_account = next_account_info(account)?;
        let account_main = next_account_info(account)?;
        //unpack helper struct to determine in which computational step the contract is in
        //if the account is not initialized, try to initialize, fails if data is not provided or
        //account is of the wrong size
        let mut account_main_data = InstructionIndex::unpack(&account_main.data.borrow());

        match account_main_data {
            Ok(account_main_data) => (
                //msg!("account_main_data.current_instruction_index {}", account_main_data.current_instruction_index);
                // do signer check etc before starting a compute instruction
                if account_main_data.signer_pubkey != *signing_account.key {
                    msg!("wrong signer");
                    Err(ProgramError::IllegalOwner)
                } else {
                    if account_main_data.current_instruction_index == 1 || account_main_data.current_instruction_index == 1502 {
                        //TODO should check the nullifier and root in the beginning
                        //check tx data hash
                        //_args.publicAmount == calculatePublicAmount(_extData.extAmount, _extData.fee)
                        //require(isKnownRoot(_args.root), "Invalid merkle root");
                        //_args.publicAmount == calculatePublicAmount(_extData.extAmount, _extData.fee)
                        msg!("if li_pre_process_instruction if");
                        li_pre_process_instruction(program_id, accounts, account_main_data.current_instruction_index);

                        Ok(())
                    }
                    //prepare inputs for proof verification with miller loop and final exponentiation
                    else if account_main_data.current_instruction_index < 466 {
                        //TODO the first instruction should check for root existence
                        _pre_process_instruction(_instruction_data, accounts);
                        Ok(())

                    }
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
                    }
                    //merkle tree insertion of new utxos
                    else if account_main_data.current_instruction_index >= 801+ 466 {

                        _pre_process_instruction_merkle_tree(&_instruction_data, accounts)?;
                        Ok(())

                    }
                    //do actual withdrawal
                    else if account_main_data.current_instruction_index == 1503 {
                        //TODO should check the nullifier again and insert it
                        Ok(())
                    } else {
                        Err(ProgramError::InvalidArgument)
                    }
                }
            ),
            //if account is not initialized yet try initialize
            Err(account_main_data) => (
                //initialize temporary storage account for shielded pool deposit, transfer or withdraw
                try_initialize_hash_bytes_account(account_main, &_instruction_data[9..], signing_account.key)

            ),
        };

    }

    Ok(())
}

fn try_initialize_hash_bytes_account(main_account: &AccountInfo,_instruction_data: &[u8], signing_address: &Pubkey) -> Result<(), ProgramError>{
    msg!("initing hash bytes account {}", main_account.data.borrow().len());
    //initing temporary storage account with bytes

    let mut main_account_data = PiBytes::unpack(&main_account.data.borrow())?;
    let mut public_inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];

    main_account_data.signing_address = signing_address.to_bytes().to_vec().clone();
    // get public_inputs from _instruction_data.
    //root
    main_account_data.root_hash = _instruction_data[0..32].to_vec().clone();
    let input1 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*main_account_data.root_hash,
    )
    .unwrap();
    //public amount
    let input2 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &_instruction_data[32..64],
    )
    .unwrap();
    main_account_data.amount = _instruction_data[32..40].to_vec().clone();
    //external data hash
    main_account_data.tx_integrity_hash = _instruction_data[64..96].to_vec().clone();
    let input3 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*main_account_data.tx_integrity_hash,
    )
    .unwrap();

    //inputNullifier0
    let inputNullifier0 = _instruction_data[96..128].to_vec().clone();
    let input4 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*inputNullifier0,
    )
    .unwrap();

    //inputNullifier1
    let inputNullifier1 = &_instruction_data[128..160];
    let input5 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        inputNullifier1,
    )
    .unwrap();
    //inputCommitment0
    let commitment_right = &_instruction_data[160..192];
    let input6 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        commitment_right,
    )
    .unwrap();
    //inputCommitment1
    let commitment_left = &_instruction_data[192..224];
    let input7 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        commitment_left,
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
    main_account_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[224..480].to_vec(), commitment_right.to_vec(), commitment_left.to_vec(), inputNullifier0.to_vec(), inputNullifier1.to_vec()].concat();
    main_account_data.changed_constants[11] = true;
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
    for i in 0..12 {
        main_account_data.changed_constants[i] = true;
    }
    main_account_data.current_instruction_index += 1;
    PiBytes::pack_into_slice(&main_account_data, &mut main_account.data.borrow_mut());
    msg!("packed successfully");
    Ok(())
}

//instruction order
pub const IX_ORDER: [u8; 1503] = [
    //init data happens before this array starts
    //check root
    1,
    //prepare inputs for verification
    40, 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 46, 41, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 46, 41, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    46, 41, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 46, 41, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 46, 41, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 46, 47, 48,
    //miller loop
    0, 1, 2, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8,
    4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6,
    8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5,
    6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4,
    5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 10, 4, 5, 6, 11, 4, 5, 6,
    //final exp
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 10, 11, 14, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 15, 15, 16, 17, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 15, 16, 17, 15, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 15, 15, 18, 19, 15, 15, 15, 15, 16, 17, 20, 21, 22, 23, 24, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 25, 25, 28, 29, 25, 25, 25, 25, 26, 27, 30, 31, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 32, 32, 35, 36, 32, 32, 32, 32, 33, 34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 38, 39, 52, 53, 54, 55, 42, 43,
    //merkle tree insertion height 18
    34, 14, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3,
    //16,
    //perform last checks and transfer requested amount
    240

];
