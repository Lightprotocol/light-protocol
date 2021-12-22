use crate::instructions_merkle_tree::*;
use crate::state_merkle_tree::{MerkleTree, HashBytes, TwoLeavesBytesPda};
use crate::state_merkle_tree;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
    clock::Clock,
    sysvar::Sysvar,
    pubkey::Pubkey,
};
use crate::instructions_poseidon::{permute_instruction_first,permute_instruction_6,permute_instruction_3, permute_instruction_last};
use crate::init_bytes18;
use crate::IX_ORDER;


pub fn _pre_process_instruction_merkle_tree(_instruction_data: &[u8], accounts: &[AccountInfo] ) -> Result<(),ProgramError> {
    let account = &mut accounts.iter();
    let signer = next_account_info(account)?;
            //init instruction
            if _instruction_data.len() >= 9 && _instruction_data[8] == 240 {
                let merkle_tree_storage_acc = next_account_info(account)?;
                let mut merkle_tree_tmp_account_data = state_merkle_tree::InitMerkleTreeBytes::unpack(&merkle_tree_storage_acc.data.borrow())?;

                for i in 0..init_bytes18::INIT_BYTES_MERKLE_TREE_18.len() {
                    merkle_tree_tmp_account_data.bytes[i] = init_bytes18::INIT_BYTES_MERKLE_TREE_18[i];
                }

                if merkle_tree_tmp_account_data.bytes[0..init_bytes18::INIT_BYTES_MERKLE_TREE_18.len()] != init_bytes18::INIT_BYTES_MERKLE_TREE_18[..] {
                    msg!("merkle tree init failed");
                    return Err(ProgramError::InvalidAccountData);
                }
                state_merkle_tree::InitMerkleTreeBytes::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());

            } else {
                let hash_storage_acc = next_account_info(account)?;
                let mut hash_tmp_account_data = HashBytes::unpack(&hash_storage_acc.data.borrow())?;
                msg!("hash_tmp_account_data.current_instruction_index {}", hash_tmp_account_data.current_instruction_index);

                if hash_tmp_account_data.current_instruction_index < IX_ORDER.len() && (IX_ORDER[hash_tmp_account_data.current_instruction_index] ==  14
                    || IX_ORDER[hash_tmp_account_data.current_instruction_index] ==  25) {

                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;

                    merkle_tree_pubkey_check(*merkle_tree_storage_acc.key)?;
                    pubkey_check(
                        *signer.key,
                        solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked),
                        String::from("merkle tree locked by other account")
                    )?;

                    _process_instruction_merkle_tree(
                        IX_ORDER[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut merkle_tree_tmp_account_data,
                    );

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;
                } else if hash_tmp_account_data.current_instruction_index == 1502
                    {
                    //the pda account should be created in the same tx, the pda account also functions as escrow account

                    msg!("instruction: {}", IX_ORDER[hash_tmp_account_data.current_instruction_index]);
                    let leaf_pda = next_account_info(account)?;
                    msg!("pm here0");
                    let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&leaf_pda.data.borrow())?;
                    msg!("pm here1");
                    let nullifer0 = next_account_info(account)?;
                    msg!("pm here2");
                    let nullifer1 = next_account_info(account)?;
                    msg!("pm here3");
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    msg!("pm here4");
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    msg!("pm here5");


                    pubkey_check(
                        *signer.key,
                        solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked),
                        String::from("merkle tree locked by other account")
                    )?;

                    merkle_tree_pubkey_check(*merkle_tree_storage_acc.key)?;


                    insert_last_double ( &mut merkle_tree_tmp_account_data, &mut hash_tmp_account_data);
                    leaf_pda_account_data.leaf_left = hash_tmp_account_data.leaf_left.clone();
                    leaf_pda_account_data.leaf_right = hash_tmp_account_data.leaf_right.clone();
                    leaf_pda_account_data.merkle_tree_pubkey = state_merkle_tree::MERKLE_TREE_ACC_BYTES.to_vec().clone();

                    msg!("Lock set at slot {}", merkle_tree_tmp_account_data.time_locked );
                    msg!("lock released at slot: {}",  <Clock as Sysvar>::get()?.slot);
                    merkle_tree_tmp_account_data.time_locked = 0;
                    merkle_tree_tmp_account_data.pubkey_locked = vec![0;32];

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    TwoLeavesBytesPda::pack_into_slice(&leaf_pda_account_data, &mut leaf_pda.data.borrow_mut());

                } else if (hash_tmp_account_data.current_instruction_index < IX_ORDER.len() &&  IX_ORDER[hash_tmp_account_data.current_instruction_index] == 34 ){
                    //locks and transfers deposit money
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    let current_slot = <Clock as Sysvar>::get()?.slot.clone();
                    msg!("Current slot: {:?}",  current_slot);

                    msg!("locked at slot: {}",  merkle_tree_tmp_account_data.time_locked);
                    msg!("lock ends at slot: {}",  merkle_tree_tmp_account_data.time_locked + 1000);

                    //lock
                    if merkle_tree_tmp_account_data.time_locked == 0 ||  merkle_tree_tmp_account_data.time_locked + 1000 < current_slot{
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = signer.key.to_bytes().to_vec();
                        msg!("locked at {}", merkle_tree_tmp_account_data.time_locked);
                        msg!("locked by: {:?}", merkle_tree_tmp_account_data.pubkey_locked );
                        msg!("locked by: {:?}", solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked) );

                    } else if merkle_tree_tmp_account_data.time_locked + 1000 > current_slot /*&& solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked[..]) != *signer.key*/ {
                        msg!("contract is still locked");
                        return Err(ProgramError::InvalidInstructionData);
                    } else {
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = signer.key.to_bytes().to_vec();
                    }

                    merkle_tree_pubkey_check(*merkle_tree_storage_acc.key)?;
                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;

                } else {
                    //hash instructions do not need the merkle tree
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    merkle_tree_pubkey_check(*merkle_tree_storage_acc.key)?;

                    let mut dummy_smt = MerkleTree {is_initialized: true,
                        levels: 1,
                        filledSubtrees:vec![vec![0 as u8; 1];1],
                        //zeros: vec![vec![0 as u8; 1];1],
                        currentRootIndex: 0,
                        nextIndex: 0,
                        ROOT_HISTORY_SIZE: 10,
                        roots: vec![0 as u8; 1],
                        //leaves: vec![0],
                        current_total_deposits: 0,
                        inserted_leaf: false,
                        inserted_root: false,
                        pubkey_locked: vec![0],
                        time_locked: 0,

                    };
                    _process_instruction_merkle_tree(
                        IX_ORDER[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut dummy_smt,
                    );

                    hash_tmp_account_data.current_instruction_index +=1;
                }
                HashBytes::pack_into_slice(&hash_tmp_account_data, &mut hash_storage_acc.data.borrow_mut());
            }
            Ok(())

}


pub fn _process_instruction_merkle_tree(
        id: u8,
        hash_tmp_account: &mut HashBytes,
        merkle_tree_account: &mut MerkleTree,
    ){
        msg!("executing instruction {}", id);
    if id == 0 {
        permute_instruction_first(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index, &hash_tmp_account.left, &hash_tmp_account.right);

    } else if id == 1{
        permute_instruction_6(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 2 {
        permute_instruction_3(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 3 {
        permute_instruction_last(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 25 {
        insert_1_inner_loop(merkle_tree_account, hash_tmp_account);

    } else if id == 14 {
        insert_0_double (&vec![0], &vec![0], merkle_tree_account, hash_tmp_account);

    } else if id == 16 {
        insert_last_double ( merkle_tree_account, hash_tmp_account);
    }

}

pub fn merkle_tree_pubkey_check(account_pubkey: Pubkey) -> Result<(), ProgramError> {
    if account_pubkey != solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]) {
        msg!("invalid merkle tree");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub fn pubkey_check(account_pubkey0: Pubkey, account_pubkey1: Pubkey, msg: String) -> Result<(), ProgramError> {
    if account_pubkey0 != account_pubkey1{
        msg!(&msg);
        return Err(ProgramError::InvalidInstructionData);
    }

    Ok(())
}
