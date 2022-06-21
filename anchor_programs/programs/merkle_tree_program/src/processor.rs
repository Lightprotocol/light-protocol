use crate::instructions::{close_account, sol_transfer};
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::poseidon_merkle_tree::state_roots::check_root_hash_exists;
use crate::state::MerkleTreeTmpPda;
use crate::utils::create_pda::create_and_check_pda;

use anchor_lang::solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use crate::UpdateMerkleTree;
use std::convert::TryInto;
use std::cell::RefMut;
use crate::TWO_LEAVES_PDA_SIZE;
use anchor_lang::prelude::*;
use crate::ErrorCode;
// Processor for deposit and withdraw logic.
#[allow(clippy::comparison_chain)]
pub fn process_instruction(
    ctx: Context<UpdateMerkleTree>,
) -> Result<()>{
    let tmp_storage_pda_data = ctx.accounts.merkle_tree_tmp_storage.load()?.clone();
    msg!("\nprior process_instruction {}\n",tmp_storage_pda_data.current_instruction_index );

    if tmp_storage_pda_data.current_instruction_index > 0
        && tmp_storage_pda_data.current_instruction_index < 73
    {
        let mut merkle_tree_processor = MerkleTreeProcessor::new(None)?;

        merkle_tree_processor.process_instruction(ctx)?;
    }
    // Checks and inserts nullifier pdas, two Merkle tree leaves (output utxo hashes),
    // executes transaction, deposit or withdrawal, and closes the tmp account.
    else if tmp_storage_pda_data.current_instruction_index == 73 {
        // TODO make this its own instruction


        // if *merkle_tree_pda.owner != *program_id {
        //     msg!("Invalid merkle tree owner.");
        //     return Err(ProgramError::IllegalOwner);
        // }



        msg!("Inserting new merkle root.");
        let mut merkle_tree_processor = MerkleTreeProcessor::new(None)?;
        let close_acc = &ctx.accounts.merkle_tree_tmp_storage.to_account_info();
        let close_to_acc = &ctx.accounts.authority.to_account_info();
        merkle_tree_processor.insert_root(ctx)?;
        // Close tmp account.
        close_account(close_acc, close_to_acc).unwrap();
    }

    Ok(())
}


pub fn process_sol_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()>{
    const DEPOSIT: u8 = 1;
    const WITHDRAWAL: u8 = 2;
    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;

    msg!("instruction_data[0] {}", instruction_data[0]);

    match instruction_data[0] {
        DEPOSIT => {
            let tmp_storage_pda = next_account_info(account)?;
            let system_program_account = next_account_info(account)?;
            let rent_sysvar_info = next_account_info(account)?;
            let rent = &Rent::from_account_info(rent_sysvar_info)?;
            let merkle_tree_pda_token = next_account_info(account)?;
            let user_ecrow_acc = next_account_info(account)?;

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            msg!("Depositing {}", amount);
            create_and_check_pda(
                program_id,
                signer_account,
                user_ecrow_acc,
                system_program_account,
                rent,
                &tmp_storage_pda.key.to_bytes(),
                &b"escrow"[..],
                0,      //bytes
                amount, // amount
                true,   //rent_exempt
            )?;
            // Close escrow account to make deposit to shielded pool.
            close_account(user_ecrow_acc, merkle_tree_pda_token)
        }
        WITHDRAWAL => {
            let merkle_tree_pda_token = next_account_info(account)?;
            // withdraws amounts to accounts
            msg!("Entered withdrawal. {:?}", instruction_data[1..].chunks(8));
            for amount_u8 in instruction_data[1..].chunks(8) {
                let amount = u64::from_le_bytes(amount_u8.try_into().unwrap());
                let to = next_account_info(account)?;
                msg!("Withdrawing {}", amount);
                sol_transfer(merkle_tree_pda_token, to, amount)?;
            }
            Ok(())
        }
        _ => err!(ErrorCode::WithdrawalFailed),
    }
}
