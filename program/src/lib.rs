//constants for verifying key and poseidon
pub mod user_account;
pub mod utils;
//merkle tree
pub mod groth16_verifier;
pub mod instructions;
pub mod poseidon_merkle_tree;
pub mod pre_processor;
pub mod state;
pub mod state_check_nullifier;

use crate::instructions::*;
use crate::pre_processor::pre_process_instruction;
use crate::state::InstructionIndex;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use crate::groth16_verifier::groth16_processor::Groth16Processor;

use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::utils::init_bytes18;

use crate::user_account::instructions::{initialize_user_account, modify_user_account};

entrypoint!(process_instruction);

//use crate::state::MtConfig;

// #[derive(Clone)]
// struct MtInitConfig;
//
// impl MtConfig for MtInitConfig {
//     const INIT_BYTES: &'static[u8] = &init_bytes18::INIT_BYTES_MERKLE_TREE_18[..];
// }

// We use current_instruction_index to move through the call order as per [IX_ORDER].
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // initialize new merkle tree account
    if _instruction_data.len() >= 9 && _instruction_data[8] == 240 {
        let accounts_mut = accounts.clone();
        let account = &mut accounts_mut.iter();
        let _signer_account = next_account_info(account)?;
        let merkle_tree_storage_acc = next_account_info(account)?;
        //merkle_tree_tmp_account_data.initialize();
        //state::InitMerkleProcessor::<MtInitConfig>::new(merkle_tree_tmp_account_data, _instruction_data);
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(None, Some(merkle_tree_storage_acc))?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&init_bytes18::INIT_BYTES_MERKLE_TREE_18[..])
    } else if _instruction_data.len() >= 9 && _instruction_data[8] == 100 {
        msg!("in: {:?}", _instruction_data);
        let accounts_mut = accounts.clone();
        let account = &mut accounts_mut.iter();
        let signer_account = next_account_info(account)?;
        let user_account = next_account_info(account)?;

        initialize_user_account(user_account, *signer_account.key)
    } else if _instruction_data.len() >= 9 && _instruction_data[8] == 101 {
        let accounts_mut = accounts.clone();
        let account = &mut accounts_mut.iter();
        let signer_account = next_account_info(account)?;
        let user_account = next_account_info(account)?;
        modify_user_account(user_account, *signer_account.key, &_instruction_data[9..])
    }
    // transact with shielded pool
    else {
        let accounts_mut = accounts.clone();
        let account = &mut accounts_mut.iter();
        let signer_account = next_account_info(account)?;
        let account_main = next_account_info(account)?;
        //unpack helper struct to determine in which computational step the contract is in
        //if the account is not initialized, try to initialize, fails if data is not provided or
        //account is of the wrong size
        let account_main_data = InstructionIndex::unpack(&account_main.data.borrow());

        match account_main_data {
            Ok(account_main_data) => {
                //msg!("account_main_data.current_instruction_index {}", account_main_data.current_instruction_index);
                // do signer check etc before starting a compute instruction
                if account_main_data.signer_pubkey != *signer_account.key {
                    msg!("wrong signer");
                    Err(ProgramError::IllegalOwner)
                } else {
                    msg!(
                        "current ix index: {}",
                        account_main_data.current_instruction_index
                    );

                    if account_main_data.current_instruction_index == 1
                        || account_main_data.current_instruction_index == 1502
                    {
                        //TODO should check the nullifier and root in the beginning
                        //check tx data hash
                        //_args.publicAmount == calculatePublicAmount(_extData.ext_amount, _extData.fee)
                        //require(isKnownRoot(_args.root), "Invalid merkle root");
                        //_args.publicAmount == calculatePublicAmount(_extData.ext_amount, _extData.fee)
                        msg!("if pre_process_instruction if");
                        pre_process_instruction(
                            program_id,
                            accounts,
                            account_main_data.current_instruction_index,
                        )?;

                        Ok(())
                    }
                    // Main verification part
                    //prepare inputs for proof verification + miller loop + final exponentiation
                    else if account_main_data.current_instruction_index < 801 + 466 {
                        let mut groth16_processor = Groth16Processor::new(
                            account_main,
                            account_main_data.current_instruction_index,
                        )?;
                        groth16_processor.process_instruction_groth16_verifier()?;
                        Ok(())
                    }
                    //merkle tree insertion of new utxos
                    else if account_main_data.current_instruction_index >= 801 + 466 {
                        let mut merkle_tree_processor =
                            MerkleTreeProcessor::new(Some(account_main), None)?;
                        merkle_tree_processor.process_instruction(accounts)?;
                        Ok(())
                    } else {
                        Err(ProgramError::InvalidArgument)
                    }
                }
            }
            //Try to initialize the account if it's is not initialized yet
            Err(_) => {
                //initialize temporary storage account for shielded pool deposit, transfer or withdraw
                create_and_try_initialize_tmp_storage_account(
                    program_id,
                    accounts,
                    3900u64, //bytes
                    0_u64,   //lamports
                    true,    //rent_exempt
                    &_instruction_data[9..],
                )
            }
        }
    }
}

fn create_and_try_initialize_tmp_storage_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    let system_program_info = next_account_info(account)?;
    create_and_check_account(
        program_id,
        signer_account,
        account_main,
        system_program_info,
        &_instruction_data[96..128],
        &b"storage"[..],
        number_storage_bytes, //bytes
        lamports,             //lamports
        rent_exempt,          //rent_exempt
    )?;
    try_initialize_tmp_storage_account(account_main, _instruction_data, signer_account.key)
}

//instruction order
pub const IX_ORDER: [u8; 1503] = [
    //init data happens before this array starts
    //check root
    1, //prepare inputs for verification
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
    58, 58, 58, 58, 58, 58, 46, 47, 48, //miller loop
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
    6, 3, 7, 4, 5, 6, 10, 4, 5, 6, 11, 4, 5, 6, //final exp
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 10, 11, 14, 15, 15, 15, 15, 16, 17, 15, 15, 16,
    17, 15, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 15, 16, 17, 15,
    15, 16, 17, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 15, 15, 16,
    17, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15,
    15, 16, 17, 15, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 15, 15, 18, 19, 15, 15, 15,
    15, 16, 17, 20, 21, 22, 23, 24, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 28, 29, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29,
    25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 26, 27, 25, 25, 25, 25,
    25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 25, 25, 28, 29, 25, 25, 25, 25, 26, 27, 30, 31, 32, 32,
    32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35,
    36, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32,
    33, 34, 32, 32, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35,
    36, 32, 32, 33, 34, 32, 32, 32, 33, 34, 32, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32,
    32, 32, 35, 36, 32, 32, 32, 32, 33, 34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 38, 39, 52, 53, 54, 55, 42, 43, //merkle tree insertion height 18
    34, 14, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3,
    25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 2, 3, 25, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3,
    //16,
    //perform last checks and transfer requested amount
    241,
];
