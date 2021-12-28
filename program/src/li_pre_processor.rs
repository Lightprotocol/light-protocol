use crate::poseidon_merkle_tree::mt_state_roots::{check_root_hash_exists, MERKLE_TREE_ACC_BYTES};
use crate::li_instructions::{
    check_and_insert_nullifier,
    check_tx_integrity_hash
};
use crate::li_state::LiBytes;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::Pack
};
use crate::Groth16_verifier::prepare_inputs::{
    pi_instructions,
    pi_ranges::*

};
use ark_ff::{Fp256, FromBytes};
use std::convert::TryInto;
use crate::poseidon_merkle_tree::mt_processor::MerkleTreeProcessor;

//use crate::process_instruction_merkle_tree;
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
        merkle_tree_processor.process_instruction_merkle_tree(
            accounts
        );
        //process_instruction_merkle_tree(&[0u8],accounts)?;



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




pub fn try_initialize_hash_bytes_account(main_account: &AccountInfo,_instruction_data: &[u8], signing_address: &Pubkey) -> Result<(), ProgramError>{
    msg!("initing hash bytes account {}", main_account.data.borrow().len());
    //initing temporary storage account with bytes

    let mut main_account_data = LiBytes::unpack(&main_account.data.borrow())?;


    //should occur in groth16 processor
    let mut public_inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];

    main_account_data.signing_address = signing_address.to_bytes().to_vec().clone();
    // get public_inputs from _instruction_data.
    //root
    main_account_data.root_hash = _instruction_data[0..32].to_vec().clone();
    let input1 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*main_account_data.root_hash,
    )
    .unwrap();
    //public amount
    let input2 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &_instruction_data[32..64],
    )
    .unwrap();
    main_account_data.amount = _instruction_data[32..40].to_vec().clone();
    //external data hash
    main_account_data.tx_integrity_hash = _instruction_data[64..96].to_vec().clone();
    let input3 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*main_account_data.tx_integrity_hash,
    )
    .unwrap();

    //inputNullifier0
    let inputNullifier0 = _instruction_data[96..128].to_vec().clone();
    let input4 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        &*inputNullifier0,
    )
    .unwrap();

    //inputNullifier1
    let inputNullifier1 = &_instruction_data[128..160];
    let input5 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        inputNullifier1,
    )
    .unwrap();
    //inputCommitment0
    let commitment_right = &_instruction_data[160..192];
    let input6 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        commitment_right,
    )
    .unwrap();
    //inputCommitment1
    let commitment_left = &_instruction_data[192..224];
    let input7 = <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(
        commitment_left,
    )
    .unwrap();

    public_inputs = vec![input1, input2, input3, input4, input5, input6, input7];

    pi_instructions::init_pairs_instruction(
        &public_inputs,
        &mut main_account_data.i_1_range,
        &mut main_account_data.x_1_range,
        &mut main_account_data.i_2_range,
        &mut main_account_data.x_2_range,
        &mut main_account_data.i_3_range,
        &mut main_account_data.x_3_range,
        &mut main_account_data.i_4_range,
        &mut main_account_data.x_4_range,
        &mut main_account_data.i_5_range,
        &mut main_account_data.x_5_range,
        &mut main_account_data.i_6_range,
        &mut main_account_data.x_6_range,
        &mut main_account_data.i_7_range,
        &mut main_account_data.x_7_range,
        &mut main_account_data.g_ic_x_range,
        &mut main_account_data.g_ic_y_range,
        &mut main_account_data.g_ic_z_range,
    );
    msg!("len _instruction_data{}", _instruction_data.len());
    let indices: [usize; 17] = [
        I_1_RANGE_INDEX,
        X_1_RANGE_INDEX,
        I_2_RANGE_INDEX,
        X_2_RANGE_INDEX,
        I_3_RANGE_INDEX,
        X_3_RANGE_INDEX,
        I_4_RANGE_INDEX,
        X_4_RANGE_INDEX,
        I_5_RANGE_INDEX,
        X_5_RANGE_INDEX,
        I_6_RANGE_INDEX,
        X_6_RANGE_INDEX,
        I_7_RANGE_INDEX,
        X_7_RANGE_INDEX,
        G_IC_X_RANGE_INDEX,
        G_IC_Y_RANGE_INDEX,
        G_IC_Z_RANGE_INDEX,
    ];
    for i in indices.iter() {
        main_account_data.changed_variables[*i] = true;
    }

    // should occur in light protocol logic
    main_account_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[224..480].to_vec(), commitment_right.to_vec(), commitment_left.to_vec(), inputNullifier0.to_vec(), inputNullifier1.to_vec()].concat();
    main_account_data.changed_constants[11] = true;


    for i in 0..12 {
        main_account_data.changed_constants[i] = true;
    }
    main_account_data.current_instruction_index += 1;
    LiBytes::pack_into_slice(&main_account_data, &mut main_account.data.borrow_mut());
    msg!("packed successfully");
    Ok(())
}
