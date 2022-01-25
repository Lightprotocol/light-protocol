use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

use solana_program::program::invoke_signed;
use solana_program::system_instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
};

use crate::nullifier_state::NullifierState;
use crate::state::ChecksAndTransferState;
use crate::Groth16Processor;
use borsh::BorshSerialize;
use std::convert::TryInto;

pub fn transfer(_from: &AccountInfo, _to: &AccountInfo, amount: u64) -> Result<(), ProgramError> {
    if _from
        .try_borrow_mut_lamports()
        .unwrap()
        .checked_sub(amount)
        .is_none()
    {
        msg!("Invalid amount.");
        return Err(ProgramError::InvalidArgument);
    }
    **_from.try_borrow_mut_lamports().unwrap() -= amount;

    if _to
        .try_borrow_mut_lamports()
        .unwrap()
        .checked_add(amount)
        .is_none()
    {
        msg!("Invalid amount.");
        return Err(ProgramError::InvalidArgument);
    }
    **_to.try_borrow_mut_lamports().unwrap() += amount;
    msg!(
        "Transferred of {} Lamp from {:?} to {:?}",
        amount,
        _from.key,
        _to.key
    );
    Ok(())
}

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
    try_initialize_tmp_storage_pda(account_main, _instruction_data, signer_account.key)
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

    let nullifier_account_data = NullifierState::unpack(&nullifier_account.data.borrow())?;
    NullifierState::pack_into_slice(
        &nullifier_account_data,
        &mut nullifier_account.data.borrow_mut(),
    );
    Ok(1u8)
}

pub fn create_and_check_account<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    passed_in_pda: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    _instruction_data: &[u8],
    domain_separation_seed: &[u8],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
) -> Result<(), ProgramError> {
    msg!(
        "domain_separation_seed: {:?}",
        &[&_instruction_data, &domain_separation_seed]
    );
    let derived_pubkey =
        Pubkey::find_program_address(&[_instruction_data, domain_separation_seed], program_id); // TODO: clippy. check if [..] rm has sideeffects

    if derived_pubkey.0 != *passed_in_pda.key {
        msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
        msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
        msg!("Passed-in pda pubkey {:?}", *passed_in_pda.key);
        msg!("Instruction data seed  {:?}", _instruction_data);
        return Err(ProgramError::InvalidInstructionData);
    }
    let rent = Rent::default();
    let mut account_lamports = lamports;
    if rent_exempt {
        account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap());
    }
    // TODO: if not rent_exempt apply min rent, currently every account is rent_exempt on devnet
    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,
            passed_in_pda.key,
            account_lamports,
            number_storage_bytes,
            program_id,
        ),
        &[
            signer_account.clone(),
            passed_in_pda.clone(),
            system_program.clone(),
        ],
        &[&[
            _instruction_data,
            domain_separation_seed,
            &[derived_pubkey.1],
        ]],
    )?;

    // Check for rent exemption
    if rent_exempt {
        if !rent.is_exempt(**passed_in_pda.lamports.borrow(), 2) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::InvalidInstructionData);
        }
    }
    Ok(())
}

pub fn try_initialize_tmp_storage_pda(
    tmp_storage_pda: &AccountInfo,
    _instruction_data: &[u8],
    signing_address: &Pubkey,
) -> Result<(), ProgramError> {
    msg!(
        "Initing tmp_storage_pda {}",
        tmp_storage_pda.data.borrow().len()
    );
    // Initializing temporary storage pda with instruction data.

    let mut tmp_storage_pda_data = ChecksAndTransferState::unpack(&tmp_storage_pda.data.borrow())?;

    let mut groth16_processor = Groth16Processor::new(
        tmp_storage_pda,
        tmp_storage_pda_data.current_instruction_index,
    )?;
    groth16_processor.try_initialize(&_instruction_data[0..224])?;

    tmp_storage_pda_data.signing_address = signing_address.to_bytes().to_vec();
    tmp_storage_pda_data.root_hash = _instruction_data[0..32].to_vec();
    tmp_storage_pda_data.amount = _instruction_data[32..64].to_vec();
    tmp_storage_pda_data.tx_integrity_hash = _instruction_data[64..96].to_vec();

    let input_nullifier_0 = _instruction_data[96..128].to_vec();
    let input_nullifier_1 = &_instruction_data[128..160];

    let commitment_right = &_instruction_data[160..192];
    let commitment_left = &_instruction_data[192..224];

    tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[224..480].to_vec(), // proof
        commitment_right.to_vec(),            // TODO left right
        commitment_left.to_vec(),
        input_nullifier_0.to_vec(),
        input_nullifier_1.to_vec(),
    ]
    .concat();
    tmp_storage_pda_data.to_address = _instruction_data[480..512].to_vec();
    tmp_storage_pda_data.ext_amount = _instruction_data[512..520].to_vec();
    let relayer = _instruction_data[520..552].to_vec();
    let fee = _instruction_data[552..560].to_vec();

    check_tx_integrity_hash(
        tmp_storage_pda_data.to_address.to_vec(),
        tmp_storage_pda_data.ext_amount.to_vec(),
        relayer.to_vec(),
        fee.to_vec(),
        &tmp_storage_pda_data.tx_integrity_hash,
    )?;
    for i in 0..12 {
        tmp_storage_pda_data.changed_constants[i] = true;
    }
    tmp_storage_pda_data.current_instruction_index += 1;
    ChecksAndTransferState::pack_into_slice(
        &tmp_storage_pda_data,
        &mut tmp_storage_pda.data.borrow_mut(),
    );
    msg!("packed successfully");
    Ok(())
}

//performs the following security checks:
//signer is consistent over all tx of a pool tx
//the correct merkle tree is called
//instruction data is empty
//there are no more and no less than the required accounts
//attached to the tx, the accounts have the appropiate length
/*
pub fn security_checks(
        signer_pubkey: &Pubkey,
        signer_pubkey_passed_in: &Pubkey,
        instruction_data_len: usize
    ) -> Result<(), ProgramError> {
    if *signer_pubkey != *signer_pubkey_passed_in {
        msg!("*signer_pubkey {:?} != *signer_pubkey_passed_in {:?}", *signer_pubkey, *signer_pubkey_passed_in);
        return Err(ProgramError::IllegalOwner);
    }
    if instruction_data_len >= 9 {
        msg!("instruction_data_len: {}", instruction_data_len);
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}
*/
