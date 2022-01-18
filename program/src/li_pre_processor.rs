use crate::li_instructions::{
    check_and_insert_nullifier,
    check_tx_integrity_hash,
    create_and_check_account
};
use crate::li_state::LiBytes;
use crate::poseidon_merkle_tree::mt_processor::MerkleTreeProcessor;
use crate::poseidon_merkle_tree::mt_state_roots::{check_root_hash_exists, MERKLE_TREE_ACC_BYTES};
use crate::Groth16_verifier::groth16_processor::Groth16Processor;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
// use spl_math::uint::U256;
use ark_ff::biginteger::{BigInteger256, BigInteger384};
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::BigInteger;
use ark_ff::Fp256;
use std::convert::{TryInto, TryFrom};

//use crate::process_instruction_merkle_tree;
//pre processor for light protocol logic
//merkle root checks
//nullifier checks
//_args.publicAmount == calculatePublicAmount(_extData.extAmount, _extData.fee)
//check tx data hash
//deposit and withdraw logic
pub fn li_pre_process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    current_instruction_index: usize,
) -> Result<(), ProgramError> {
    msg!("entered li_pre_process_instruction");

    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;
    let main_account = next_account_info(account)?;
    let mut account_data = LiBytes::unpack(&main_account.data.borrow())?;

    if current_instruction_index == 1 {

        let merkle_tree_account = next_account_info(account)?;
        msg!("merkletree acc key: {:?}", *merkle_tree_account.key);
        msg!(
            "merkletree key to check: {:?}",
            solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..])
        );
        account_data.found_root =
            check_root_hash_exists(merkle_tree_account, &account_data.root_hash)?;
    }
    //nullifier checks
    //deposit and withdraw logic
    else if current_instruction_index == 1502 {
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0 = next_account_info(account)?;
        let nullifier1 = next_account_info(account)?;
        let merkle_tree_account = next_account_info(account)?;
        let system_program_info = next_account_info(account)?;
        msg!("starting nullifier check");
        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            signer_account,
            nullifier0,
            system_program_info,
            &account_data.proof_a_b_c_leaves_and_nullifiers[320..352],
        )?;
        msg!("nullifier0 inserted {}", account_data.found_nullifier);

        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            signer_account,
            nullifier1,
            system_program_info,
            &account_data.proof_a_b_c_leaves_and_nullifiers[352..384],
        )?;
        msg!("nullifier1 inserted {}", account_data.found_nullifier);

        msg!("inserting new merkle root");
        let mut merkle_tree_processor = MerkleTreeProcessor::new(Some(main_account), None)?;
        msg!("creating pda account onchain");

        let ext_amount = i64::from_le_bytes(account_data.ext_amount.clone().try_into().unwrap());
        let pub_amount = <BigInteger256 as FromBytes>::read(&account_data.amount[..]).unwrap();

        //check not necessary since withdrawing pub_amount later
        // if ext_amount as u64 != pub_amount.0 {
        //     msg!("external and public amount need to match");
        //     return Err(ProgramError::InvalidInstructionData);
        // }
        msg!("withdrawal amount: {:?}", pub_amount);

        if ext_amount > 0 {
            if *merkle_tree_account.key
                != solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES)
            {
                msg!("recipient has to be merkle tree account for deposit");
                return Err(ProgramError::InvalidInstructionData);
            }
            create_and_check_account(
                program_id,
                signer_account,
                two_leaves_pda,
                system_program_info,
                &account_data.proof_a_b_c_leaves_and_nullifiers[320..352],
                &b"leaves"[..],
                106u64,     //bytes
                ext_amount as u64,  //lamports
                true,       //rent_exempt
            )?;
            msg!("created pda account onchain successfully");
            merkle_tree_processor.process_instruction_merkle_tree(accounts)?;
            // calculate extAmount from pubAmount:
            let ext_amount_from_pub = i64::from_str_radix(&pub_amount.to_string(), 16).unwrap();

            assert_eq!(ext_amount, ext_amount_from_pub, "ext_amount != pub_amount");
            msg!("deposited {}", ext_amount);
            transfer(
                two_leaves_pda,
                merkle_tree_account,
                u64::try_from(ext_amount).unwrap(),
            )?;
        } else if ext_amount <= 0 {
            let recipient_account = next_account_info(account)?;

            if *recipient_account.key
                != solana_program::pubkey::Pubkey::new(&account_data.to_address)
            {
                msg!("recipient has to be address specified in tx integrity hash");
                return Err(ProgramError::InvalidInstructionData);
            }

            create_and_check_account(
                program_id,
                signer_account,
                two_leaves_pda,
                system_program_info,
                &(*signer_account.key).to_bytes(),
                &b"leaves"[..],
                106u64,     //bytes
                0u64,  //lamports
                true,       //rent_exempt
            )?;
            msg!("created pda account onchain successfully");
            merkle_tree_processor.process_instruction_merkle_tree(accounts)?;
            // calculate extAmount from pubAmount:
            let field_size: Vec<u8> = vec![
                1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129,
                129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48,
            ];
            let mut field = <BigInteger256 as FromBytes>::read(&field_size[..]).unwrap();
            field.sub_noborrow(&pub_amount);
            // field is the positive value
            let ext_amount_from_pub = field.0[0];

            transfer(
                merkle_tree_account,
                recipient_account,
                u64::try_from(ext_amount * -1).unwrap(), // *-1?
            )?;
        }
    } else if current_instruction_index == 4 {
        //state_check_nullifier::check_and_insert_nullifier();
    }

    account_data.current_instruction_index += 1;
    LiBytes::pack_into_slice(&account_data, &mut main_account.data.borrow_mut());
    msg!("finished successfully");
    Ok(())
}

//performs the following security checks:
//signer is consistent over all tx of a pool tx
//the correct merkle tree is called
//instruction data is empty
//there are no more and no less than the required accounts
//attached to the tx, the accounts have the appropiate length
pub fn li_security_checks(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;

    let main_account = next_account_info(account)?;
    // assert_eq!(
    //     *signing_account.key,
    //     solana_program::pubkey::Pubkey::new(&account_data.signing_address)
    // );
    Ok(())
}

pub fn transfer(_from: &AccountInfo, _to: &AccountInfo, amount: u64) -> Result<(), ProgramError>{
    if _from.try_borrow_mut_lamports().unwrap().checked_sub(amount).is_some() != true {
        msg!("invalid withdrawal amount");
        return Err(ProgramError::InvalidArgument);
    }
    **_from.try_borrow_mut_lamports().unwrap() -= amount; //1000000000; // 1 SOL

    if _to.try_borrow_mut_lamports().unwrap().checked_add(amount).is_some() != true  {
        msg!("invalid withdrawal amount");
        return Err(ProgramError::InvalidArgument);
    }
    **_to.try_borrow_mut_lamports().unwrap() +=  amount;
    msg!(
        "transferred of {} Lamp from {:?} to {:?}",
        amount,
        _from.key,
        _to.key
    );
    Ok(())
}
