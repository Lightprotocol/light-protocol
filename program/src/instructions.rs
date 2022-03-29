use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

use crate::nullifier_state::NullifierState;
use crate::state::ChecksAndTransferState;
use crate::utils::config::{
    ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_ACC_BYTES_ARRAY, TMP_STORAGE_ACCOUNT_TYPE,
};
use crate::Groth16Processor;
use ark_ed_on_bn254::FqParameters;
use ark_ff::{biginteger::BigInteger256, bytes::FromBytes, fields::FpParameters, BigInteger};
use borsh::BorshSerialize;
use solana_program::program::invoke_signed;
use solana_program::system_instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use std::convert::{TryFrom, TryInto};

#[allow(clippy::comparison_chain)]
pub fn check_external_amount(
    tmp_storage_pda_data: &ChecksAndTransferState,
) -> Result<(u64, u64), ProgramError> {
    let ext_amount =
        i64::from_le_bytes(tmp_storage_pda_data.ext_amount.clone().try_into().unwrap());
    // ext_amount includes relayer_fee
    let relayer_fee =
        u64::from_le_bytes(tmp_storage_pda_data.relayer_fee.clone().try_into().unwrap());
    // pub_amount is the public amount included in public inputs for proof verification
    let pub_amount = <BigInteger256 as FromBytes>::read(&tmp_storage_pda_data.amount[..]).unwrap();

    if ext_amount > 0 {
        if pub_amount.0[1] != 0 || pub_amount.0[2] != 0 || pub_amount.0[3] != 0 {
            msg!("Public amount is larger than u64.");
            return Err(ProgramError::InvalidInstructionData);
        }

        let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);

        if pub_amount_fits_i64.is_err() {
            msg!("Public amount is larger than i64.");
            return Err(ProgramError::InvalidInstructionData);
        }

        //check amount
        if pub_amount.0[0].checked_add(relayer_fee).unwrap() != ext_amount.try_into().unwrap() {
            msg!(
                "Deposit invalid external amount (relayer_fee) {} != {}",
                pub_amount.0[0] + relayer_fee,
                ext_amount
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok((ext_amount.try_into().unwrap(), relayer_fee))
    } else if ext_amount < 0 {
        // calculate ext_amount from pubAmount:
        let mut field = FqParameters::MODULUS;
        field.sub_noborrow(&pub_amount);

        // field.0[0] is the positive value
        if field.0[1] != 0 || field.0[2] != 0 || field.0[3] != 0 {
            msg!("Public amount is larger than u64.");
            return Err(ProgramError::InvalidInstructionData);
        }
        let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);
        if pub_amount_fits_i64.is_err() {
            msg!("Public amount is larger than i64.");
            return Err(ProgramError::InvalidInstructionData);
        }

        if field.0[0]
            != u64::try_from(-ext_amount)
                .unwrap()
                .checked_add(relayer_fee)
                .unwrap()
        {
            msg!(
                "Withdrawal invalid external amount: {} != {}",
                pub_amount.0[0],
                relayer_fee + u64::try_from(-ext_amount).unwrap()
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(((-ext_amount).try_into().unwrap(), relayer_fee))
    } else if ext_amount == 0 {
        Ok((ext_amount.try_into().unwrap(), relayer_fee))
    } else {
        msg!("Invalid state checking external amount.");
        Err(ProgramError::InvalidInstructionData)
    }
}

pub fn token_transfer<'a, 'b>(
    token_program: &'b AccountInfo<'a>,
    source: &'b AccountInfo<'a>,
    destination: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    seed: &[u8],
    bump_seed: &[u8],
    amount: u64,
) -> Result<(), ProgramError> {
    let authority_signature_seeds = [seed, bump_seed];

    let signers = &[&authority_signature_seeds[..]];
    msg!(
        "Transferring {} from {:?} to {:?}",
        amount,
        source.key,
        destination.key
    );

    let ix = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            source.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        signers,
    )?;
    Ok(())
}

#[allow(clippy::clone_double_ref)]
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
    let rent_sysvar_info = next_account_info(account)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    create_and_check_pda(
        program_id,
        signer_account,
        account_main,
        system_program_info,
        rent,
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
    tx_integrity_hash: Vec<u8>,
    merkle_tree_index: u8,
    encrypted_utxos: Vec<u8>,
    merkle_tree_pda_pubkey: Vec<u8>,
) -> Result<(), ProgramError> {
    let input = [
        recipient,
        ext_amount,
        relayer,
        fee,
        merkle_tree_pda_pubkey,
        vec![merkle_tree_index],
        encrypted_utxos,
    ]
    .concat();

    let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;
    msg!("hash computed");

    if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&tx_integrity_hash) {
        msg!("tx_integrity_hash verification failed.");
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

pub fn check_and_insert_nullifier<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    nullifier_account: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent: &Rent,
    _instruction_data: &[u8],
) -> Result<u8, ProgramError> {
    create_and_check_pda(
        program_id,
        signer_account,
        nullifier_account,
        system_program,
        rent,
        _instruction_data,
        &b"nf"[..],
        2u64, //nullifier pda length
        0u64, //lamports
        true, //rent_exempt
    )?;
    // Initializing nullifier pda.
    let nullifier_account_data = NullifierState::unpack(&nullifier_account.data.borrow())?;
    NullifierState::pack_into_slice(
        &nullifier_account_data,
        &mut nullifier_account.data.borrow_mut(),
    );
    Ok(1u8)
}

pub fn create_and_check_pda<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    passed_in_pda: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent: &Rent,
    _instruction_data: &[u8],
    domain_separation_seed: &[u8],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
) -> Result<(), ProgramError> {
    let derived_pubkey =
        Pubkey::find_program_address(&[_instruction_data, domain_separation_seed], program_id);

    if derived_pubkey.0 != *passed_in_pda.key {
        msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
        msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
        msg!("Passed-in pda pubkey {:?}", *passed_in_pda.key);
        msg!("Instruction data seed  {:?}", _instruction_data);
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut account_lamports = lamports;
    if rent_exempt {
        account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap());
    } else {
        account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap()) / 365;
    }
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
    if rent_exempt
        && !rent.is_exempt(
            **passed_in_pda.lamports.borrow(),
            number_storage_bytes.try_into().unwrap(),
        )
    {
        msg!("Account is not rent exempt.");
        return Err(ProgramError::AccountNotRentExempt);
    }
    Ok(())
}

pub const PREPARED_INPUTS_RANGE_START: usize = 0;
pub const PREPARED_INPUTS_RANGE_END: usize = 224;
pub const PROOF_A_B_C_RANGE_START: usize = 224;
pub const PROOF_A_B_C_RANGE_END: usize = 480;

pub fn try_initialize_tmp_storage_pda(
    tmp_storage_pda: &AccountInfo,
    _instruction_data: &[u8],
    signing_address: &Pubkey,
) -> Result<(), ProgramError> {
    msg!(
        "Initializing tmp_storage_pda: {}",
        tmp_storage_pda.data.borrow().len()
    );
    // Initializing temporary storage pda with instruction data.
    let mut tmp_storage_pda_data = ChecksAndTransferState::unpack(&tmp_storage_pda.data.borrow())?;
    tmp_storage_pda_data.account_type = TMP_STORAGE_ACCOUNT_TYPE;

    let mut groth16_processor = Groth16Processor::new(
        tmp_storage_pda,
        tmp_storage_pda_data.current_instruction_index,
    )?;
    // store zero knowledge prepared inputs bytes
    groth16_processor.try_initialize(
        &_instruction_data[PREPARED_INPUTS_RANGE_START..PREPARED_INPUTS_RANGE_END],
    )?;

    tmp_storage_pda_data.signing_address = signing_address.to_bytes().to_vec();
    tmp_storage_pda_data.root_hash = _instruction_data[0..32].to_vec();
    tmp_storage_pda_data.amount = _instruction_data[32..64].to_vec();
    tmp_storage_pda_data.tx_integrity_hash = _instruction_data[64..96].to_vec();

    let input_nullifier_0 = _instruction_data[96..128].to_vec();
    let input_nullifier_1 = &_instruction_data[128..160];

    let leaf_right = &_instruction_data[160..192];
    let leaf_left = &_instruction_data[192..224];

    let encrypted_utxos = &_instruction_data[593..593 + ENCRYPTED_UTXOS_LENGTH];
    tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers = [
        _instruction_data[PROOF_A_B_C_RANGE_START..PROOF_A_B_C_RANGE_END].to_vec(),
        leaf_right.to_vec(),
        leaf_left.to_vec(),
        input_nullifier_0.to_vec(),
        input_nullifier_1.to_vec(),
        encrypted_utxos.to_vec(),
    ]
    .concat();
    tmp_storage_pda_data.recipient = _instruction_data[480..512].to_vec();
    tmp_storage_pda_data.ext_amount = _instruction_data[512..520].to_vec();
    let relayer = _instruction_data[520..552].to_vec();

    // Check that relayer in integrity hash == signer.
    // In case of deposit the depositor is their own relayer
    if *signing_address != Pubkey::new(&relayer) {
        msg!("Specified relayer is not signer.");
        return Err(ProgramError::InvalidAccountData);
    }

    let fee = _instruction_data[552..560].to_vec();
    tmp_storage_pda_data.relayer_fee = fee;

    let merkle_tree_pda_pubkey = _instruction_data[560..592].to_vec();
    tmp_storage_pda_data.merkle_tree_index = _instruction_data[592];

    if merkle_tree_pda_pubkey
        != MERKLE_TREE_ACC_BYTES_ARRAY
            [<usize as TryFrom<u8>>::try_from(tmp_storage_pda_data.merkle_tree_index).unwrap()]
        .0
        .to_vec()
    {
        msg!(
            "Merkle tree in tx integrity hash not whitelisted or wrong ID. is: {:?}",
            merkle_tree_pda_pubkey,
        );
        return Err(ProgramError::InvalidAccountData);
    }

    check_tx_integrity_hash(
        tmp_storage_pda_data.recipient.to_vec(),
        tmp_storage_pda_data.ext_amount.to_vec(),
        relayer.to_vec(),
        tmp_storage_pda_data.relayer_fee.to_vec(),
        tmp_storage_pda_data.tx_integrity_hash.to_vec(),
        tmp_storage_pda_data.merkle_tree_index,
        encrypted_utxos.to_vec(),
        merkle_tree_pda_pubkey,
    )?;
    for i in 0..11 {
        tmp_storage_pda_data.changed_constants[i] = true;
    }
    tmp_storage_pda_data.current_instruction_index += 1;
    ChecksAndTransferState::pack_into_slice(
        &tmp_storage_pda_data,
        &mut tmp_storage_pda.data.borrow_mut(),
    );
    msg!("packed init.");
    Ok(())
}
