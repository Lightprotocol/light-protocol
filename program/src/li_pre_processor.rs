use crate::li_instructions::{check_and_insert_nullifier, check_tx_integrity_hash};
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
use std::convert::TryInto;

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
    msg!("here0");

    let _signing_account = next_account_info(account)?;
    msg!("here1");
    let main_account = next_account_info(account)?;
    msg!("here2");
    let mut account_data = LiBytes::unpack(&main_account.data.borrow())?;

    if current_instruction_index == 1 {
        msg!("here3");
        let merkle_tree_account = next_account_info(account)?;
        msg!("here4");
        msg!("merkletree acc key: {:?}", *merkle_tree_account.key);
        msg!(
            "key to check: {:?}",
            solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..])
        );
        account_data.found_root =
            check_root_hash_exists(merkle_tree_account, &account_data.root_hash)?;
    }
    //nullifier checks
    //deposit and withdraw logic
    else if current_instruction_index == 1502 {
        //assert_eq!(true, false, "does not work yet");
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0 = next_account_info(account)?;
        let nullifier1 = next_account_info(account)?;
        let merkle_tree_account = next_account_info(account)?;
        msg!("starting nullifier check");
        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            _signing_account.key,
            nullifier0,
            &account_data.proof_a_b_c_leaves_and_nullifiers[320..352],
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
            vec![1u8, 32],
            vec![1u8, 8],
            vec![1u8, 32],
            vec![1u8, 8],
            vec![1u8, 32],
            vec![1u8, 32],
            &account_data.tx_integrity_hash,
        )?;
        msg!("inserting new merkle root");
        let mut merkle_tree_processor = MerkleTreeProcessor::new(Some(main_account), None)?;
        merkle_tree_processor.process_instruction_merkle_tree(accounts);

        // TODO: this is a hotfix. Checks first byte only.
        let ext_amount_bool =
            i64::from_le_bytes(account_data.ext_amount.clone().try_into().unwrap());
        let amount = <BigInteger256 as FromBytes>::read(&account_data.amount[..]).unwrap();

        msg!("amount 0 or 1? {:?}", ext_amount_bool);
        msg!("amount: {:?}", amount);

        if ext_amount_bool > 0 {
            if *merkle_tree_account.key
                != solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES)
            {
                msg!("recipient has to be merkle tree account for deposit");
                return Err(ProgramError::InvalidInstructionData);
            }

            // calculate extAmount from pubAmount:
            let ext_amount = i64::from_str_radix(&amount.to_string(), 16).unwrap();
            msg!("deposited {}", ext_amount);
            transfer(
                two_leaves_pda,
                merkle_tree_account,
                u64::try_from(ext_amount).unwrap(),
            );
        } else if ext_amount_bool <= 0 {
            // bool is 0 hotfix.
            // TODO: replace "bool" with negative bytes of extAmount.
            let recipient_account = next_account_info(account)?;

            // if *recipient_account.key
            //     != solana_program::pubkey::Pubkey::new(&account_data.to_address)
            // {
            //     msg!("recipient has to be address specified in tx integrity hash");
            //     return Err(ProgramError::InvalidInstructionData);
            // }
            // calculate extAmount:
            let field_size: Vec<u8> = vec![
                1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129,
                129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48,
            ];
            let mut field = <BigInteger256 as FromBytes>::read(&field_size[..]).unwrap();
            field.sub_noborrow(&amount);
            let string = field.to_string();
            let ext_amount = i64::from_str_radix(&string, 16).unwrap();

            msg!("withdraw of {}", ext_amount);

            transfer(
                merkle_tree_account,
                recipient_account,
                u64::try_from(ext_amount).unwrap(), // *-1?
            );
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
    let _signing_account = next_account_info(account)?;

    let main_account = next_account_info(account)?;
    // assert_eq!(
    //     *signing_account.key,
    //     solana_program::pubkey::Pubkey::new(&account_data.signing_address)
    // );
    Ok(())
}

use std::convert::TryFrom;
pub fn transfer(_from: &AccountInfo, _to: &AccountInfo, amount: u64) {
    **_from.try_borrow_mut_lamports().unwrap() -= amount; //1000000000; // 1 SOL

    **_to.try_borrow_mut_lamports().unwrap() += amount;

    //merkle_tree_account.current_total_deposits += 1;
    msg!(
        "transferred of {} Lamp from {:?} to {:?}",
        amount,
        _from.key,
        _to.key
    );
}

// recipient: toFixedHex(recipient, 20),
// extAmount: toFixedHex(extAmount),
// relayer: toFixedHex(relayer, 20),
// fee: toFixedHex(fee),
// encryptedOutput1: encryptedOutput1,
// encryptedOutput2: encryptedOutput2,

pub fn try_initialize_hash_bytes_account(
    main_account: &AccountInfo,
    _instruction_data: &[u8],
    signing_address: &Pubkey,
) -> Result<(), ProgramError> {
    msg!(
        "initing hash bytes account {}",
        main_account.data.borrow().len()
    );
    //initing temporary storage account with bytes

    let mut main_account_data = LiBytes::unpack(&main_account.data.borrow())?;

    let mut groth16_processor =
        Groth16Processor::new(main_account, main_account_data.current_instruction_index)?;
    groth16_processor.try_initialize(&_instruction_data[0..224])?;

    main_account_data.signing_address = signing_address.to_bytes().to_vec().clone();
    main_account_data.root_hash = _instruction_data[0..32].to_vec().clone();
    main_account_data.amount = _instruction_data[32..64].to_vec().clone(); // note: changed from 8 to 32 bytes to support negative amounts
    main_account_data.tx_integrity_hash = _instruction_data[64..96].to_vec().clone();

    let input_nullifier_0 = _instruction_data[96..128].to_vec().clone();
    let input_nullifier_1 = &_instruction_data[128..160];

    let commitment_right = &_instruction_data[160..192];
    let commitment_left = &_instruction_data[192..224];

    main_account_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[224..480].to_vec(),
        commitment_right.to_vec(),
        commitment_left.to_vec(),
        input_nullifier_0.to_vec(),
        input_nullifier_1.to_vec(),
    ]
    .concat();
    // adding ext_amount to indicate + - transfer.
    main_account_data.ext_amount = _instruction_data[480..488].to_vec().clone();
    //main_account_data.changed_constants[11] = true;

    for i in 0..12 {
        main_account_data.changed_constants[i] = true;
    }
    main_account_data.current_instruction_index += 1;
    LiBytes::pack_into_slice(&main_account_data, &mut main_account.data.borrow_mut());
    msg!("packed successfully");
    Ok(())
}
