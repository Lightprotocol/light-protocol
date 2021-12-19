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
};
use crate::instructions_poseidon::{permute_instruction_first,permute_instruction_6,permute_instruction_3, permute_instruction_last};
use crate::init_bytes18;
use crate::IX_ORDER;


pub fn _pre_process_instruction_merkle_tree(_instruction_data: &[u8], accounts: &[AccountInfo] ) -> Result<(),ProgramError> {
    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?;

            //init instruction
            if _instruction_data[8] == 240 {
                msg!("here1 _pre_process_instruction_merkle_tree");
                let merkle_tree_storage_acc = next_account_info(account)?;
                msg!("here2 _pre_process_instruction_merkle_tree {:?}", merkle_tree_storage_acc.key);

                let mut merkle_tree_tmp_account_data = state_merkle_tree::InitMerkleTreeBytes::unpack(&merkle_tree_storage_acc.data.borrow())?;
                msg!("here3 _pre_process_instruction_merkle_tree");

                for i in 0..init_bytes18::INIT_BYTES_MERKLE_TREE_18.len() {
                    //msg!("{}", i);
                    merkle_tree_tmp_account_data.bytes[i] = init_bytes18::INIT_BYTES_MERKLE_TREE_18[i];

                }
                msg!("{:?}", merkle_tree_tmp_account_data.bytes[0..32].to_vec());
                assert_eq!(merkle_tree_tmp_account_data.bytes[0..init_bytes18::INIT_BYTES_MERKLE_TREE_18.len()], init_bytes18::INIT_BYTES_MERKLE_TREE_18[..]);
                state_merkle_tree::InitMerkleTreeBytes::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());

            } else {
                let hash_storage_acc = next_account_info(account)?;
                let mut hash_tmp_account_data = HashBytes::unpack(&hash_storage_acc.data.borrow())?;
                // if hash_tmp_account_data.current_instruction_index == 0 {
                //     hash_tmp_account_data.current_instruction_index += 801;
                // }
                assert_eq!(IX_ORDER[hash_tmp_account_data.current_instruction_index], init_bytes18::INSERT_INSTRUCTION_ORDER_18[hash_tmp_account_data.current_instruction_index - (801+ 466)]);
                msg!("instruction: {}", IX_ORDER[hash_tmp_account_data.current_instruction_index]);

                if IX_ORDER[hash_tmp_account_data.current_instruction_index] ==  14
                    {
                    msg!("here");
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    msg!("here1");
                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    // msg!("data equal: {}", _instruction_data[10..42].to_vec() == vec![143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2]);
                    // msg!("data equal: {:?}", _instruction_data[10..42].to_vec());
                    msg!("here2");
                    _process_instruction_merkle_tree(
                        IX_ORDER[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut merkle_tree_tmp_account_data,
                        //_instruction_data[10..42].to_vec(),
                        //_instruction_data[42..74].to_vec()
                    );
                    msg!("here3");
                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    hash_tmp_account_data.current_instruction_index +=1;
                    msg!("here4");
                } else if ( IX_ORDER[hash_tmp_account_data.current_instruction_index] == 25
                    ){

                    msg!("instruction: {}", IX_ORDER[hash_tmp_account_data.current_instruction_index]);
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    msg!("here -2");
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    msg!("here -1 {:?} != {:?}", *account1.key , merkle_tree_tmp_account_data.pubkey_locked);
                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        msg!("here err");
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    msg!("here0");
                    _process_instruction_merkle_tree(
                        IX_ORDER[hash_tmp_account_data.current_instruction_index],
                        &mut hash_tmp_account_data,
                        &mut merkle_tree_tmp_account_data,
                        // vec![0],
                        // vec![0]
                    );
                    msg!("here1");



                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());


                    hash_tmp_account_data.current_instruction_index +=1;

                } else if IX_ORDER[hash_tmp_account_data.current_instruction_index] == 16
                    {
                    //the pda account should be created in the same tx, the pda account also functions as escrow account

                    msg!("instruction: {}", IX_ORDER[hash_tmp_account_data.current_instruction_index]);
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    let leaf_pda = next_account_info(account)?;
                    let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&leaf_pda.data.borrow())?;

                    msg!("here -2");
                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
                    msg!("here -1 {:?} != {:?}", *account1.key , merkle_tree_tmp_account_data.pubkey_locked);
                    if *account1.key != solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked){
                        msg!("here err");
                        return Err(ProgramError::InvalidInstructionData);
                    }
                    msg!("here0");

                    insert_last_double ( &mut merkle_tree_tmp_account_data, &mut hash_tmp_account_data);
                    leaf_pda_account_data.leaf_left = hash_tmp_account_data.leaf_left.clone();
                    msg!("Leaf left: {:?}", leaf_pda_account_data.leaf_left);
                    leaf_pda_account_data.leaf_right = hash_tmp_account_data.leaf_right.clone();
                    msg!("Leaf right: {:?}", leaf_pda_account_data.leaf_right);
                    leaf_pda_account_data.merkle_tree_pubkey = state_merkle_tree::MERKLE_TREE_ACC_BYTES.to_vec().clone();


                    msg!("here1");

                    msg!("Lock set at slot {}", merkle_tree_tmp_account_data.time_locked );
                    msg!("lock released at slot: {}",  <Clock as Sysvar>::get()?.slot);
                    merkle_tree_tmp_account_data.time_locked = 0;
                    merkle_tree_tmp_account_data.pubkey_locked = vec![0;32];

                    //deposit(&mut merkle_tree_tmp_account_data, &merkle_tree_storage_acc, &leaf_pda);

                    msg!("here2");

                    MerkleTree::pack_into_slice(&merkle_tree_tmp_account_data, &mut merkle_tree_storage_acc.data.borrow_mut());
                    TwoLeavesBytesPda::pack_into_slice(&leaf_pda_account_data, &mut leaf_pda.data.borrow_mut());

                    hash_tmp_account_data.current_instruction_index +=1;

                } else if ( IX_ORDER[hash_tmp_account_data.current_instruction_index] == 34 ){
                    //locks and transfers deposit money
                    let merkle_tree_storage_acc = next_account_info(account)?;
                    let mut merkle_tree_tmp_account_data = MerkleTree::unpack(&merkle_tree_storage_acc.data.borrow())?;
                    let current_slot = <Clock as Sysvar>::get()?.slot.clone();
                    msg!("Current slot: {:?}",  current_slot);
                    //assert_eq!(true, false, "contract is still locked");

                    msg!("locked at slot: {}",  merkle_tree_tmp_account_data.time_locked);
                    msg!("lock ends at slot: {}",  merkle_tree_tmp_account_data.time_locked + 1000);

                    //lock
                    if merkle_tree_tmp_account_data.time_locked == 0 ||  merkle_tree_tmp_account_data.time_locked + 1000 < current_slot{
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = account1.key.to_bytes().to_vec();
                        msg!("locked at {}", merkle_tree_tmp_account_data.time_locked);
                        msg!("locked by: {:?}", merkle_tree_tmp_account_data.pubkey_locked );
                        msg!("locked by: {:?}", solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked) );


                    } else if merkle_tree_tmp_account_data.time_locked + 1000 > current_slot /*&& solana_program::pubkey::Pubkey::new(&merkle_tree_tmp_account_data.pubkey_locked[..]) != *account1.key*/ {
                        assert_eq!(true, false, "contract is still locked");
                        return Err(ProgramError::InvalidInstructionData);
                    } else {
                        merkle_tree_tmp_account_data.time_locked = <Clock as Sysvar>::get()?.slot;
                        merkle_tree_tmp_account_data.pubkey_locked = account1.key.to_bytes().to_vec();
                        //assert_eq!(true, false, "something went wrong");
                    }

                    assert_eq!(*merkle_tree_storage_acc.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

                    //deposit(&mut merkle_tree_tmp_account_data, &merkle_tree_storage_acc, &tmp_escrow_acc);
                    //deposit(merkle_tree_account, pure_merkle_tree_account);

                    //_process_instruction_merkle_tree( IX_ORDER[hash_tmp_account_data.current_instruction_index],);

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
                        // vec![0],
                        // vec![0]
                    );

                    hash_tmp_account_data.current_instruction_index +=1;
                }
                msg!("here5");
                HashBytes::pack_into_slice(&hash_tmp_account_data, &mut hash_storage_acc.data.borrow_mut());
                msg!("here6");
            }
            Ok(())

}


pub fn _process_instruction_merkle_tree(
        id: u8,
        hash_tmp_account: &mut HashBytes,
        merkle_tree_account: &mut MerkleTree,
        // leaf_r: Vec<u8>,
        // leaf_l: Vec<u8>
        //pure_merkle_tree_account: &AccountInfo
    ){
        msg!("executing instruction {}", id);
    //pub fn processor_poseidon(id: u8, hash_tmp_account: &mut PoseidonHashMemory, data: &[u8]) {
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
        //assert_eq!(true, false, "should not enter here");
    }

}
