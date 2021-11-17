use crate::instructions_merkle_tree::*;
use crate::parsers_merkle_tree::*;
use crate::state_merkle_tree::{MerkleTree, HashBytes, InitMerkleTreeBytes};
use crate::state_merkle_tree;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
    clock::Clock,
    sysvar::Sysvar,
};


pub fn _pre_process_instruction_merkle_tree(_instruction_data: &[u8], accounts: &[AccountInfo] ) -> Result<(),ProgramError> {
    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?;
            //init instruction
            if _instruction_data[0] == 240{
                msg!("here1 _pre_process_instruction_merkle_tree");
                let merkle_tree_storage_acc = next_account_info(account)?;
                msg!("here2 _pre_process_instruction_merkle_tree {:?}", merkle_tree_storage_acc.key);

                let mut merkle_tree_tmp_account_data = state_merkle_tree::InitMerkleTreeBytes::unpack(&merkle_tree_storage_acc.data.borrow())?;
                msg!("here3 _pre_process_instruction_merkle_tree");

                for i in 0..state_merkle_tree::INIT_DATA_MERKLE_TREE_HEIGHT_11.len() {
                    merkle_tree_tmp_account_data.bytes[i] = state_merkle_tree::INIT_DATA_MERKLE_TREE_HEIGHT_11[i];
                }
                msg!("{:?}", merkle_tree_tmp_account_data.bytes[0..32].to_vec());
                assert_eq!(merkle_tree_tmp_account_data.bytes[0..state_merkle_tree::INIT_DATA_MERKLE_TREE_HEIGHT_11.len()], state_merkle_tree::INIT_DATA_MERKLE_TREE_HEIGHT_11[..]);
                state_merkle_tree::InitMerkleTreeBytes::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());

            } else {
                let hash_storage_acc = next_account_info(account)?;
                let mut hash_tmp_account_data = HashBytes::unpack(&hash_storage_acc.data.borrow())?;

                if (state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index] == 24){
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    _process_instruction_merkle_tree( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index],&mut hash_tmp_account_data,&mut merkle_tree_tmp_account_data, _instruction_data[2..34].to_vec(), &merkle_tree_storage_acc);

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;

                } else if ( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index] == 25 || state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index] == 26 ){
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    _process_instruction_merkle_tree( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index],&mut hash_tmp_account_data,&mut merkle_tree_tmp_account_data, vec![0], &merkle_tree_storage_acc);

                    if state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index]  == 26 {
                        msg!("Lock set at slot {}", merkle_tree_tmp_account_data.time_locked );
                        msg!("lock released at slot: {}",  <Clock as Sysvar>::get()?.slot);
                        merkle_tree_tmp_account_data.time_locked = 0;
                        merkle_tree_tmp_account_data.pubkey_locked = vec![0;32];

                    }
                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());


                    hash_tmp_account_data.current_instruction_index +=1;

                } else if ( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index] == 34 ){
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let tmp_escrow_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    let current_slot = <Clock as Sysvar>::get()?.slot.clone();
                    msg!("Current slot: {:?}",  current_slot);
                    //assert_eq!(true, false, "contract is still locked");

                    msg!("locked at slot: {}",  merkle_tree_tmp_account_data.time_locked);
                    msg!("lock ends at slot: {}",  merkle_tree_tmp_account_data.time_locked + 500);

                    msg!("locked by: {:?}", merkle_tree_tmp_account_data.pubkey_locked );


                    //lock
                    if merkle_tree_tmp_account_data.time_locked == 0 ||  merkle_tree_tmp_account_data.time_locked + 500 < current_slot{
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = account1.key.to_bytes().to_vec();
                        msg!("locked at {}", merkle_tree_tmp_account_data.time_locked);
                    } else if merkle_tree_tmp_account_data.time_locked + 500 > current_slot /*&& solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked[..]) != *account1.key*/ {
                        assert_eq!(true, false, "contract is still locked");
                        return Err(ProgramError::InvalidInstructionData);
                    } else {
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = account1.key.to_bytes().to_vec();
                        //assert_eq!(true, false, "something went wrong");
                    }

                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

                    deposit(&mut merkle_tree_tmp_account_data, &merkle_tree_storage_acc, &tmp_escrow_acc);
                    //deposit(merkle_tree_account, pure_merkle_tree_account);

                    //_process_instruction_merkle_tree( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index],);

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;

                } else {
                    //hash instructions do not need the merkle tree
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    //assert_eq!(*account1.key, solana_program::pubkey::Pubkey::new(&merkle_tree_storage_acc.pubkey_locked));

                    let mut dummy_smt = MerkleTree {is_initialized: true,
                        levels: 1,
                        filledSubtrees:vec![vec![0 as u8; 1];1],
                        zeros: vec![vec![0 as u8; 1];1],
                        currentRootIndex: 0,
                        nextIndex: 0,
                        ROOT_HISTORY_SIZE: 10,
                        roots: vec![0 as u8; 1],
                        leaves: vec![0 as u8; 1],
                        current_total_deposits: 0,
                        inserted_leaf: false,
                        inserted_root: false,
                        pubkey_locked: vec![0],
                        time_locked: 0,

                    };
                    _process_instruction_merkle_tree( state_merkle_tree::INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11[hash_tmp_account_data.current_instruction_index], &mut hash_tmp_account_data, &mut dummy_smt, vec![0], &merkle_tree_storage_acc);
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
        leaf: Vec<u8>,
        pure_merkle_tree_account: &AccountInfo
    ){

    if id == 1 {
        absorb_instruction_squeeze_field_elem_22(&mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3, &hash_tmp_account.left, 0);
    } else if id == 2 {
        absorb_instruction_squeeze_field_elem_22(&mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3, &hash_tmp_account.right, 1);
    } else if id == 3 {
        permute_instruction_1_and_3(0, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
    } else if id == 4 {
        permute_instruction_1_and_3(2, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
    } else if id == 5 {
        permute_instruction_2_x_4(4, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 6 {
        permute_instruction_2_x_4(8, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 7 {
        permute_instruction_2_x_4(12, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 8 {
        permute_instruction_2_x_4(16, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 9 {
        permute_instruction_2_x_4(20, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 10 {
        permute_instruction_2_x_4(24, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 11 {
        permute_instruction_2_x_4(28, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 12 {
        permute_instruction_2_x_4(32, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 13 {
        permute_instruction_2_x_4(36, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 14 {
        permute_instruction_2_x_4(40, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 15 {
        permute_instruction_2_x_4(44, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 16 {
        permute_instruction_2_x_4(48, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 17 {
        permute_instruction_2_x_4(52, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 18 {
        permute_instruction_2_x_4(56, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 30 {
        permute_instruction_2_x_4(60, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 31 {
        permute_instruction_2_x_4(64, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 32 {
        permute_instruction_2_x_4(68, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 33 {
        permute_instruction_2_x_2(72, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
        // hash_tmp_account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 19 { // *3
        permute_instruction_2_x_3(74, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
    } else if id == 20 {
        permute_instruction_1_and_3(77, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
    } else if id == 21 {
        permute_instruction_1_and_3(79, &mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3);
    } else if id == 23 {
        squeeze_internal_custom(&mut hash_tmp_account.state_range_1 , &mut hash_tmp_account.state_range_2, &mut hash_tmp_account.state_range_3, &mut hash_tmp_account.currentLevelHash, 0);
    } else if id == 24 {
        insert_0(&leaf,merkle_tree_account, hash_tmp_account);
    } else if id == 25 {
        insert_1_inner_loop(merkle_tree_account, hash_tmp_account);
    } else if id == 26 {
        insert_last( merkle_tree_account, hash_tmp_account);
    } else if id == 27 {
        init_sponge(hash_tmp_account);
    } else if id == 34 {
        //deposit(merkle_tree_account, pure_merkle_tree_account);
        assert_eq!(false, true, "should not enter here");
    }



}
