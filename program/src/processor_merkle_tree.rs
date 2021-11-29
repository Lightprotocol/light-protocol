use crate::instructions_merkle_tree::*;
use crate::parsers_merkle_tree::*;
use crate::state_merkle_tree::{MerkleTree, HashBytes};
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
use crate::instructions_poseidon::{permute_instruction_first,permute_instruction_6,permute_instruction_3, permute_instruction_last};
use crate::init_bytes11;

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

                for i in 0..init_bytes11::INIT_BYTES_MERKLE_TREE_11.len() {
                    merkle_tree_tmp_account_data.bytes[i] = init_bytes11::INIT_BYTES_MERKLE_TREE_11[i];
                }
                msg!("{:?}", merkle_tree_tmp_account_data.bytes[0..32].to_vec());
                assert_eq!(merkle_tree_tmp_account_data.bytes[0..init_bytes11::INIT_BYTES_MERKLE_TREE_11.len()], init_bytes11::INIT_BYTES_MERKLE_TREE_11[..]);
                state_merkle_tree::InitMerkleTreeBytes::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());

            } else {
                let hash_storage_acc = next_account_info(account)?;
                let mut hash_tmp_account_data = HashBytes::unpack(&hash_storage_acc.data.borrow())?;

                if (init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index] == 24){
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    // msg!("data equal: {}", _instruction_data[10..42].to_vec() == vec![143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2]);
                    // msg!("data equal: {:?}", _instruction_data[10..42].to_vec());

                    _process_instruction_merkle_tree(
                        init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut merkle_tree_tmp_account_data,
                        _instruction_data[10..42].to_vec()
                    );

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;

                } else if ( init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index] == 25 || init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index] == 26 ){
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    _process_instruction_merkle_tree(
                        init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut merkle_tree_tmp_account_data,
                        vec![0]
                    );

                    if init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index]  == 26 {
                        msg!("Lock set at slot {}", merkle_tree_tmp_account_data.time_locked );
                        msg!("lock released at slot: {}",  <Clock as Sysvar>::get()?.slot);
                        merkle_tree_tmp_account_data.time_locked = 0;
                        merkle_tree_tmp_account_data.pubkey_locked = vec![0;32];

                    }
                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());


                    hash_tmp_account_data.current_instruction_index +=1;

                } else if ( init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index] == 34 ){
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

                    //_process_instruction_merkle_tree( init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index],);

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
                    _process_instruction_merkle_tree(
                        init_bytes11::INSERT_INSTRUCTION_ORDER_11[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut dummy_smt,
                        vec![0]
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
        leaf: Vec<u8>
        //pure_merkle_tree_account: &AccountInfo
    ){

    //pub fn processor_poseidon(id: u8, hash_tmp_account: &mut PoseidonHashMemory, data: &[u8]) {
    if id == 0 {
        permute_instruction_first(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index, &hash_tmp_account.left, &hash_tmp_account.right);

    } else if id == 1{
        permute_instruction_6(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 2 {
        permute_instruction_3(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 3 {
        permute_instruction_last(&mut hash_tmp_account.state,&mut hash_tmp_account.current_round, &mut hash_tmp_account.current_round_index);

    } else if id == 24 {
        insert_0(&leaf,merkle_tree_account, hash_tmp_account);

    } else if id == 25 {
        insert_1_inner_loop(merkle_tree_account, hash_tmp_account);

    } else if id == 26 {
        insert_last( merkle_tree_account, hash_tmp_account);
    }

}
