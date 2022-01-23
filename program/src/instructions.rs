use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

use solana_program::program::invoke_signed;
use solana_program::system_instruction;
use solana_program::{
    account_info::{AccountInfo, next_account_info}, msg, program_error::ProgramError, program_pack::Pack,
    pubkey::Pubkey, sysvar::rent::Rent,
};

use crate::state::LiBytes;
use crate::state_check_nullifier::NullifierBytesPda;
use crate::Groth16Processor;
use borsh::BorshSerialize;
use std::convert::TryInto;

pub fn create_and_try_initialize_tmp_storage_pda(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    let system_program_info = next_account_info(account)?;
    create_and_check_account(
        program_id,
        signer_account,
        account_main,
        system_program_info,
        &_instruction_data[96..128],
        &b"storage"[..],
        number_storage_bytes, //bytes
        lamports,             //lamports
        rent_exempt,          //rent_exempt
    )?;
    try_initialize_tmp_storage_account(account_main, _instruction_data, signer_account.key)
}

pub fn check_tx_integrity_hash(
    recipient: Vec<u8>,
    ext_amount: Vec<u8>,
    relayer: Vec<u8>,
    fee: Vec<u8>,
    tx_integrity_hash: &[u8], // Vec<u8> TODO: CLIPPY
) -> Result<(), ProgramError> {
    let input = [recipient, ext_amount, relayer, fee].concat();

    let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;

    if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&tx_integrity_hash) {
        msg!("tx_integrity_hash verification failed");
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
pub fn check_and_insert_nullifier<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    nullifier_account: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    _instruction_data: &[u8],
) -> Result<u8, ProgramError> {
    create_and_check_account(
        program_id,
        signer_account,
        nullifier_account,
        system_program,
        _instruction_data,
        &b"nf"[..],
        2u64, //bytes
        0u64, //904800u64,  //lamports
        true, //rent_exempt
    )?;

    let nullifier_account_data = NullifierBytesPda::unpack(&nullifier_account.data.borrow())?;
    NullifierBytesPda::pack_into_slice(
        &nullifier_account_data,
        &mut nullifier_account.data.borrow_mut(),
    );
    Ok(1u8)
}

pub fn create_and_check_account<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    derived_account: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    _instruction_data: &[u8],
    domain_sep_seed: &[u8],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
) -> Result<(), ProgramError> {
    msg!(
        "domain_sep_seed: {:?}",
        &[&_instruction_data, &domain_sep_seed]
    );
    let derived_pubkey =
        Pubkey::find_program_address(&[_instruction_data, domain_sep_seed], program_id); // TODO: clippy. check if [..] rm has sideeffects

    if derived_pubkey.0 != *derived_account.key {
        msg!("passed inaccount is wrong");
        msg!(" pubkey.0 {:?}", derived_pubkey.0);
        msg!("passed in account {:?}", *derived_account.key);
        msg!("ixdata SEED  {:?}", _instruction_data);
        // panic!();
        return Err(ProgramError::InvalidInstructionData);
    }
    let rent = Rent::default();
    let mut account_lamports = lamports;
    if rent_exempt {
        account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap());
    }
    //msg!("number_storage_bytes {}", lamports + rent.minimum_balance(number_storage_bytes.try_into().unwrap()));
    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,
            derived_account.key,
            // 1e7 as u64,
            //lamports, // TODO: adapt
            account_lamports,     //.max(1),
            number_storage_bytes, //.try_into().unwrap(),
            program_id,
        ),
        &[
            signer_account.clone(),
            derived_account.clone(),
            system_program.clone(),
        ],
        // A slice of seed slices, each seed slice being the set
        // of seeds used to generate one of the PDAs required by the
        // callee program, the final seed being a single-element slice
        // containing the `u8` bump seed.
        &[&[
            _instruction_data,
            domain_sep_seed, // Almighty clippy. check if rm [..] has sideffects
            &[derived_pubkey.1],
        ]],
    )?;

    //check for rent exemption
    if rent_exempt {
        if !rent.is_exempt(**derived_account.lamports.borrow(), 2) {
            msg!("account is not rent exempt");
            return Err(ProgramError::InvalidInstructionData);
        }
    }
    Ok(())
}

pub fn try_initialize_tmp_storage_account(
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

    main_account_data.signing_address = signing_address.to_bytes().to_vec();
    main_account_data.root_hash = _instruction_data[0..32].to_vec();
    main_account_data.amount = _instruction_data[32..64].to_vec();
    main_account_data.tx_integrity_hash = _instruction_data[64..96].to_vec();

    let input_nullifier_0 = _instruction_data[96..128].to_vec();
    let input_nullifier_1 = &_instruction_data[128..160];

    let commitment_right = &_instruction_data[160..192];
    let commitment_left = &_instruction_data[192..224];

    main_account_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[224..480].to_vec(), // proof
        commitment_right.to_vec(), //TODO left right
        commitment_left.to_vec(),
        input_nullifier_0.to_vec(),
        input_nullifier_1.to_vec(),
    ]
    .concat();
    main_account_data.to_address = _instruction_data[480..512].to_vec(); // ..688
    main_account_data.ext_amount = _instruction_data[512..520].to_vec();
    let relayer = _instruction_data[520..552].to_vec();
    let fee = _instruction_data[552..560].to_vec();
    // let encrypted_output_0 = _instruction_data[560..796].to_vec().clone(); // 16
    // let encrypted_output_1 = _instruction_data[796..1032].to_vec().clone();

    // msg!(
    //     "main_account_data.signing_address {:?}",
    //     main_account_data.signing_address
    // );
    // msg!(
    //     "main_account_data.root_hash {:?}",
    //     main_account_data.root_hash
    // );
    // msg!("main_account_data.amount {:?}", main_account_data.amount);
    // msg!(
    //     "main_account_data.tx_integrity_hash {:?}",
    //     main_account_data.tx_integrity_hash
    // );
    // // msg!("input_nullifier_0 ); {:?}", input_nullifier_0);
    // // msg!("input_nullifier_1 ); {:?}", input_nullifier_1);
    // // msg!("commitment_right ); {:?}", commitment_right);
    // // msg!("commitment_left ); {:?}", commitment_left);
    // msg!(
    //     "main_account_data.to_address {:?}",
    //     main_account_data.to_address
    // );
    // msg!(
    //     "main_account_data.ext_amount {:?}",
    //     main_account_data.ext_amount
    // );
    // msg!("relayer ); {:?}", relayer);
    // msg!("fee ); {:?}", fee);
    // msg!("encrypted_output_0 ); {:?}", encrypted_output_0);
    // msg!("encrypted_output_1 ); {:?}", encrypted_output_1);
    // // panic!();

    //main_account_data.changed_constants[11] = true;

    check_tx_integrity_hash(
        // vec![1u8, 32],   // recipient
        main_account_data.to_address.to_vec(),
        // vec![1u8, 8],    // ext_amount
        main_account_data.ext_amount.to_vec(),
        // vec![1u8, 32],   // relayer
        relayer.to_vec(),
        //vec![1u8, 8],    // fee
        fee.to_vec(),
        // vec![1u8, 32],   // o0
        // encrypted_output_0.to_vec(),
        // // vec![1u8, 32],   // o1
        // encrypted_output_1.to_vec(),
        &main_account_data.tx_integrity_hash,
    )?;
    // panic!();
    for i in 0..12 {
        main_account_data.changed_constants[i] = true;
    }
    main_account_data.current_instruction_index += 1;
    LiBytes::pack_into_slice(&main_account_data, &mut main_account.data.borrow_mut());
    msg!("packed successfully");
    Ok(())
}
