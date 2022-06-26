use crate::instructions::{close_account, sol_transfer};
use crate::poseidon_merkle_tree::processor::{
    compute_updated_merkle_tree,
};
use crate::constant::{
    MERKLE_TREE_UPDATE_START,
    MERKLE_TREE_UPDATE_LEVEL,
    LOCK_START,
    HASH_0,
    HASH_1,
    HASH_2,
    ROOT_INSERT,
    IX_ORDER
};
use crate::utils::create_pda::create_and_check_pda;

use anchor_lang::solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack
};
use crate::UpdateMerkleTree;
use anchor_lang::prelude::*;
use crate::ErrorCode;
use crate::MerkleTree;
use crate::poseidon_merkle_tree::processor::pubkey_check;

#[allow(clippy::comparison_chain)]
pub fn process_instruction(
    ctx: &mut Context<UpdateMerkleTree>,
) -> Result<()>{
    let tmp_storage_pda_data = ctx.accounts.merkle_tree_tmp_storage.load()?.clone();
    msg!("\n prior process_instruction {}\n",tmp_storage_pda_data.current_instruction_index );

    if tmp_storage_pda_data.current_instruction_index > 0
        && tmp_storage_pda_data.current_instruction_index < 56
    {
        let tmp_storage_pda_data = &mut ctx.accounts.merkle_tree_tmp_storage.load_mut()?;
        let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        pubkey_check(
            ctx.accounts.merkle_tree_tmp_storage.key(),
            Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
            String::from("Merkle tree locked by another account."),
        )?;

        msg!(
            "tmp_storage_pda_data.current_instruction_index0 {}",
            tmp_storage_pda_data.current_instruction_index
        );

        if tmp_storage_pda_data.current_instruction_index == 1 {
            compute_updated_merkle_tree(
                IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
                tmp_storage_pda_data,
                &mut merkle_tree_pda_data,
            )?;
            tmp_storage_pda_data.current_instruction_index +=1;
        }

        msg!(
            "tmp_storage_pda_data.current_instruction_index1 {}",
            tmp_storage_pda_data.current_instruction_index
        );

        compute_updated_merkle_tree(
            IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
            tmp_storage_pda_data,
            &mut merkle_tree_pda_data,
        )?;
        tmp_storage_pda_data.current_instruction_index +=1;
        // renews lock
        merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
        MerkleTree::pack_into_slice(
            &merkle_tree_pda_data,
            &mut ctx.accounts.merkle_tree.data.borrow_mut(),
        );
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
