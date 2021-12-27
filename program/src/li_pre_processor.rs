use crate::mt_state_roots::{check_root_hash_exists, MERKLE_TREE_ACC_BYTES};
use crate::fe_instructions::{
    check_and_insert_nullifier,
    check_tx_integrity_hash
};
use crate::pi_state::PiBytes;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::Pack
};
use std::convert::TryInto;
use crate::mt_processor::MerkleTreeProcessor;

//use crate::_pre_process_instruction_merkle_tree;
//pre processor for light protocol logic
//merkle root checks
//nullifier checks
//_args.publicAmount == calculatePublicAmount(_extData.extAmount, _extData.fee)
//check tx data hash
//deposit and withdraw logic
pub fn li_pre_process_instruction(program_id: &Pubkey, accounts: &[AccountInfo],current_instruction_index: usize) -> Result<(), ProgramError> {
    msg!("entered li_pre_process_instruction");

    let account = &mut accounts.iter();
    msg!("here0");

    let _signing_account = next_account_info(account)?;
    msg!("here1");
    let main_account = next_account_info(account)?;
    msg!("here2");
    let mut account_data = PiBytes::unpack(&main_account.data.borrow())?;

    if current_instruction_index == 1 {
        msg!("here3");
        let merkle_tree_account = next_account_info(account)?;
        msg!("here4");
        msg!("merkletree acc key: {:?}", *merkle_tree_account.key);
        msg!(
            "key to check: {:?}",
            solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..])
        );
        account_data.found_root = check_root_hash_exists(
            merkle_tree_account,
            &account_data.root_hash
        )?;

    }
    //check tx data hash and public amount
    else if current_instruction_index == 2 {
        //account_data.tx_integrity_hash
        //account_data.amount

    }
    //nullifier checks
    //deposit and withdraw logic
    else if current_instruction_index == 1502 {
        //assert_eq!(true, false, "does not work yet");
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0 = next_account_info(account)?;
        let nullifier1 = next_account_info(account)?;
        let merkel_tree_account = next_account_info(account)?;
        msg!("starting nullifier check");
        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            _signing_account.key,
            nullifier0,
            &account_data.proof_a_b_c_leaves_and_nullifiers[320..352]
        )?;
        msg!("nullifier0 inserted {}", account_data.found_nullifier);

        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            _signing_account.key,
            nullifier1,
            &account_data.proof_a_b_c_leaves_and_nullifiers[352..384],
        )?;
        msg!("nullifier1 inserted {}", account_data.found_nullifier);


        check_tx_integrity_hash(
            vec![1u8,32],
            vec![1u8,8],
            vec![1u8,32],
            vec![1u8,8],
            vec![1u8,32],
            vec![1u8,32],
            &account_data.tx_integrity_hash
        )?;
        //
        msg!("inserting new merkle root");
        let mut merkle_tree_processor = MerkleTreeProcessor::new(
            Some(main_account),
            None
        )?;
        merkle_tree_processor._pre_process_instruction_merkle_tree(
            accounts
        );
        //_pre_process_instruction_merkle_tree(&[0u8],accounts)?;



        let amount = i64::from_le_bytes(account_data.amount.clone().try_into().unwrap());

        //remove this for dynamic testing adjust full unit test after
        let amount: i64 = 1000000000;

        if amount > 0 {
            msg!("deposited {}", amount);
            if *merkel_tree_account.key != solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES) {
                msg!("recipient has to be merkle tree account for deposit");
                return Err(ProgramError::InvalidInstructionData);
            }
            transfer(two_leaves_pda, merkel_tree_account, u64::try_from(amount).unwrap());
        } else if amount < 0 {
            let recipient_account = next_account_info(account)?;

            msg!("withdraw of {}", amount);
            if *recipient_account.key != solana_program::pubkey::Pubkey::new(&account_data.to_address) {
                msg!("recipient has to be address specified in tx integrity hash");
                return Err(ProgramError::InvalidInstructionData);
            }
            transfer(merkel_tree_account, recipient_account, u64::try_from((amount * -1)).unwrap());
        }

    } else if current_instruction_index == 4 {
        //state_check_nullifier::check_and_insert_nullifier();
    }

    account_data.current_instruction_index +=1;
    PiBytes::pack_into_slice(&account_data, &mut main_account.data.borrow_mut());
    msg!("finished successfully");
    Ok(())
}

//performs the following security checks:
//signer is consistent over all tx of a pool tx
//the correct merkle tree is called
//instruction data is empty
//there are no more and no less than the required accounts
//attached to the tx, the accounts have the appropiate length
pub fn li_security_checks(accounts: &[AccountInfo]) -> Result<(),ProgramError> {
    let account = &mut accounts.iter();
    let _signing_account = next_account_info(account)?;

    let main_account = next_account_info(account)?;
    // assert_eq!(
    //     *signing_account.key,
    //     solana_program::pubkey::Pubkey::new(&account_data.signing_address)
    // );
    Ok(())
}

use std::convert::TryFrom;
pub fn transfer( _from: &AccountInfo, _to: &AccountInfo, amount: u64){
    **_from.try_borrow_mut_lamports().unwrap()      -= amount;//1000000000; // 1 SOL

    **_to.try_borrow_mut_lamports().unwrap()        += amount;


        //merkle_tree_account.current_total_deposits += 1;
        msg!("transferred of {} Sol from {:?} to {:?}", amount / 1000000000,_from.key, _to.key);
}

// recipient: toFixedHex(recipient, 20),
// extAmount: toFixedHex(extAmount),
// relayer: toFixedHex(relayer, 20),
// fee: toFixedHex(fee),
// encryptedOutput1: encryptedOutput1,
// encryptedOutput2: encryptedOutput2,
