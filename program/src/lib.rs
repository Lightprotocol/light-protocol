#![allow(clippy::type_complexity, clippy::ptr_arg, clippy::too_many_arguments)]

pub mod groth16_verifier;
pub mod instructions;
pub mod nullifier_state;
pub mod poseidon_merkle_tree;
pub mod processor;
pub mod state;
pub mod user_account;
pub mod utils;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};

use crate::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY};
use crate::groth16_verifier::groth16_processor::Groth16Processor;
use crate::instructions::create_and_try_initialize_tmp_storage_pda;
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::state::InstructionIndex;
use crate::user_account::instructions::initialize_user_account;
use crate::utils::config;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

#[allow(clippy::clone_double_ref)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    // 0. `[]` signer
    let signer_account = next_account_info(account)?;
    if !signer_account.is_signer {
        msg!("signer account needs to be passed in first place");
        return Err(ProgramError::IllegalOwner);
    }
    // Initialize new merkle tree account.
    if _instruction_data.len() >= 9 && _instruction_data[8] == 240 {
        let merkle_tree_storage_acc = next_account_info(account)?;
        // Check whether signer is merkle_tree_init_authority.
        if *signer_account.key != Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY) {
            msg!("Signer is not merkle tree init authority.");
            return Err(ProgramError::IllegalOwner);
        }
        let rent_sysvar_info = next_account_info(account)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;
        if !rent.is_exempt(
            **merkle_tree_storage_acc.lamports.borrow(),
            merkle_tree_storage_acc.data.borrow().len(),
        ) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::AccountNotRentExempt);
        }
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(None, Some(merkle_tree_storage_acc), *program_id)?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&config::INIT_BYTES_MERKLE_TREE_18[..])
    }
    // Initialize new onchain user account.
    else if _instruction_data.len() >= 9 && _instruction_data[8] == 100 {
        let user_account = next_account_info(account)?;
        let rent_sysvar_info = next_account_info(account)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;
        initialize_user_account(user_account, *signer_account.key, *rent)
    }
    // Modify onchain user account with arbitrary number of new utxos.
    // else if _instruction_data.len() >= 9 && _instruction_data[8] == 101 {
    //     let user_account = next_account_info(account)?;
    //     let rent_sysvar_info = next_account_info(account)?;
    //     let rent = &Rent::from_account_info(rent_sysvar_info)?;
    //     modify_user_account(
    //         user_account,
    //         *signer_account.key,
    //         *rent,
    //         &_instruction_data[9..],
    //     )
    // }
    // Close onchain user account.
    // else if _instruction_data.len() >= 9 && _instruction_data[8] == 102 {
    //     let user_account = next_account_info(account)?;
    //     let rent_sysvar_info = next_account_info(account)?;
    //     let rent = &Rent::from_account_info(rent_sysvar_info)?;
    //     close_user_account(user_account, signer_account, *rent)
    // }
    // Transact with shielded pool.
    // This instruction has to be called 1502 times to perform all computation.
    // There are different instructions which have to be executed in a specific order.
    // The instruction order is hardcoded in IX_ORDER.
    // After every instruction the program increments an internal counter (current_instruction_index).
    // The current_instruction_index is stored in a temporary storage pda on-chain.
    else {
        // 1. `[writable]` tmp_storage_pda stores intermediate state.
        let tmp_storage_pda = next_account_info(account)?;

        // Unpack the current_instruction_index.
        let tmp_storage_pda_data = InstructionIndex::unpack(&tmp_storage_pda.data.borrow());
        // Check whether tmp_storage_pda is initialized, if not try create and initialize.
        // First instruction will always create and initialize a new tmp_storage_pda.
        match tmp_storage_pda_data {
            Err(ProgramError::InvalidAccountData) => {
                // Will enter here the first iteration because the account does not exist.
                // Creates a tmp_storage_pda to store state while verifying the zero-knowledge proof and
                // updating the merkle tree.
                // All data used during computation is passed in as instruction_data with this instruction.
                // No subsequent instructions read instruction_data.
                // instruction_data:
                //    [ root,
                //      public amount,
                //      external data hash,
                //      nullifier0,
                //      nullifier1,
                //      leaf_right,
                //      leaf_left,
                //      proof,
                //      recipient,
                //      ext_amount,
                //      relayer,
                //      fee]

                create_and_try_initialize_tmp_storage_pda(
                    program_id,
                    accounts,
                    3900u64 + ENCRYPTED_UTXOS_LENGTH as u64, // bytes
                    0_u64,                                   // lamports
                    false,                                   // rent_exempt
                    &_instruction_data[9..], // Data starts after instruction identifier.
                )
            }
            Err(_) => Err(ProgramError::InvalidInstructionData),
            Ok(tmp_storage_pda_data) => {
                // Check signer before starting a compute instruction.
                if tmp_storage_pda_data.signer_pubkey != *signer_account.key {
                    msg!("Wrong signer.");
                    Err(ProgramError::IllegalOwner)
                } else if *program_id != *tmp_storage_pda.owner {
                    msg!(
                        "Wrong owner. {:?} != {:?}",
                        *program_id,
                        *tmp_storage_pda.owner
                    );
                    Err(ProgramError::IllegalOwner)
                } else {
                    msg!(
                        "current ix index: {}",
                        tmp_storage_pda_data.current_instruction_index
                    );
                    // *ROOT_CHECK:*
                    // Checks whether root exists in Merkle tree history vec.
                    // Accounts:
                    // 2. `[]` Merkle tree
                    // *INSERT_LEAVES_NULLIFIER_AND_TRANSFER:*
                    // Inserts leaves, inserts nullifier, updates Merkle tree root and transfers
                    // funds to the recipient.
                    // For deposits the recipient is the merkle_tree_pda. For withdrawals the passed
                    // in recipient account receives the funds.
                    // This instruction will never be reached if proof verification fails.
                    // Accounts:
                    // 2. `[writable]` tmp_storage_pda
                    // 3. `[writable]` two_leaves_pda
                    // 4. `[writable]` nullifier0_pda
                    // 5. `[writable]` nullifier1_pda
                    // 6. `[writable]` merkle_tree_pda
                    // 7. `[writable]` merkle_tree_pda_token
                    // 8. `[]` spl_program
                    // 9. `[]` token_program_account
                    // 10. `[]` rent_sysvar_info
                    // 11. `[]` authority
                    // 12. `[writable]` user_pda_token
                    // 13. `[writable]` relayer_pda_token

                    if tmp_storage_pda_data.current_instruction_index == ROOT_CHECK
                        || tmp_storage_pda_data.current_instruction_index
                            == INSERT_LEAVES_NULLIFIER_AND_TRANSFER
                    {
                        processor::process_instruction(
                            program_id,
                            accounts,
                            tmp_storage_pda_data.current_instruction_index,
                        )?;
                        Ok(())
                    }
                    // Zero-knowledge proof verification.
                    // Accounts:
                    // 2. `[writable]` tmp_storage_pda
                    else if tmp_storage_pda_data.current_instruction_index > ROOT_CHECK
                        && tmp_storage_pda_data.current_instruction_index < VERIFICATION_END_INDEX
                    {
                        let mut groth16_processor = Groth16Processor::new(
                            tmp_storage_pda,
                            tmp_storage_pda_data.current_instruction_index,
                        )?;
                        groth16_processor.process_instruction_groth16_verifier()?;
                        Ok(())
                    }
                    //merkle tree insertion of new utxos
                    // Accounts:
                    // 2. `[writable]` tmp_storage_pda
                    // 3. `[]` merkle_tree_pda
                    else if tmp_storage_pda_data.current_instruction_index
                        >= VERIFICATION_END_INDEX
                    {
                        let mut merkle_tree_processor =
                            MerkleTreeProcessor::new(Some(tmp_storage_pda), None, *program_id)?;
                        merkle_tree_processor.process_instruction(accounts)?;
                        Ok(())
                    } else {
                        Err(ProgramError::InvalidArgument)
                    }
                }
            }
        }
    }
}

const ROOT_CHECK: usize = 1;
const INSERT_LEAVES_NULLIFIER_AND_TRANSFER: usize = 1501;
const VERIFICATION_END_INDEX: usize = 1266;
pub const NULLIFIER_0_START: usize = 320;
pub const NULLIFIER_0_END: usize = 352;
pub const NULLIFIER_1_START: usize = 352;
pub const NULLIFIER_1_END: usize = 384;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
pub const IX_ORDER: [u8; 1502] = [
    //init data happens before this array starts
    //check root
    1, //prepare inputs for verification
    /*40, */ 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 46, 41, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 46, 41, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 46, 41, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 46, 41, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 46, 41, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 46, 47, 48, //miller loop
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
    //perform last checks and transfer requested amount
    241,
];
