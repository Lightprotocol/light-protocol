use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

use crate::state::MerkleTreeTmpPda;
use crate::utils::config::{
    ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_ACC_BYTES_ARRAY, TMP_STORAGE_ACCOUNT_TYPE, MERKLE_TREE_TMP_PDA_SIZE
};
use crate::utils::create_pda::create_and_check_pda;
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
    // msg!(
    //     "Transferring {} from {:?} to {:?}",
    //     amount,
    //     source.key,
    //     destination.key
    // );

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

pub struct MerkleTreeTmpStorageAccInputData {
    pub node_left:  Vec<u8>,
    pub node_right:  Vec<u8>,
    pub verifier_tmp_pda:  Vec<u8>,
    pub relayer:  Vec<u8>,
    pub merkle_tree_pda_pubkey: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub found_root: u8,
    pub is_initialized: u8,
}

impl MerkleTreeTmpStorageAccInputData {
    pub fn new(
            node_left: Vec<u8>,
            node_right:Vec<u8>,
            root_hash: Vec<u8>,
            merkle_tree_address: Vec<u8>,
            signing_address: Vec<u8>,
            verifier_tmp_pda: Vec<u8>
        ) -> Result<MerkleTreeTmpStorageAccInputData, ProgramError> {
        Ok(MerkleTreeTmpStorageAccInputData {
            node_left:          node_left,
            node_right:         node_right,
            relayer:            signing_address,
            merkle_tree_pda_pubkey:  merkle_tree_address,
            verifier_tmp_pda:  verifier_tmp_pda,
            root_hash: root_hash,
            is_initialized: 1u8,
            found_root: 0u8,
        })
    }

    pub fn return_ix_data(&self) ->  Result<Vec<u8>, ProgramError>{
        let res = [
        self.node_left.clone(),
        self.node_right.clone(),
        self.root_hash.clone(),
        self.relayer.clone(),
        self.merkle_tree_pda_pubkey.clone(),
        self.verifier_tmp_pda.clone(),
        ].concat();
        Ok(res)
    }


    pub fn try_initialize(&mut self, account: &AccountInfo) -> Result<(), ProgramError> {
        let mut tmp = MerkleTreeTmpPda::new();
        tmp.node_left = self.node_left.clone();
        tmp.node_right = self.node_right.clone();
        tmp.leaf_left = self.node_left.clone();
        tmp.leaf_right = self.node_right.clone();
        tmp.verifier_tmp_pda = self.verifier_tmp_pda.clone();
        tmp.relayer = self.relayer.clone();
        tmp.root_hash = self.root_hash.clone();
        tmp.merkle_tree_pda_pubkey = self.merkle_tree_pda_pubkey.clone();
        tmp.found_root = self.found_root.clone();
        tmp.changed_state = 1;
        MerkleTreeTmpPda::pack_into_slice(
            &tmp,
            &mut account.data.borrow_mut(),
        );
        Ok(())
    }
}


#[allow(clippy::clone_double_ref)]
pub fn create_and_try_initialize_tmp_storage_pda(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    let verifier_tmp_pda = next_account_info(account)?;
    // TODO: check owner
    let merkle_tree_tmp_pda = next_account_info(account)?;
    let system_program_info = next_account_info(account)?;
    let rent_sysvar_info = next_account_info(account)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    msg!("MerkleTreeTmpStorageAccInputData started");

    let mut merkle_tree_pda = MerkleTreeTmpStorageAccInputData::new(
        _instruction_data[0..32].to_vec(),
        _instruction_data[32..64].to_vec(),
        _instruction_data[64..96].to_vec(),
        merkle_tree_tmp_pda.key.to_bytes().to_vec(),
        signer_account.key.to_bytes().to_vec(),
        verifier_tmp_pda.key.to_bytes().to_vec()
    )?;
    msg!("MerkleTreeTmpStorageAccInputData done");

    create_and_check_pda(
        program_id,
        signer_account,
        merkle_tree_tmp_pda,
        system_program_info,
        rent,
        &merkle_tree_pda.node_left,
        &b"storage"[..],
        MERKLE_TREE_TMP_PDA_SIZE.try_into().unwrap(),   //bytes
        0,                          //lamports
        true,                       //rent_exempt
    )?;
    msg!("created_pda");
    merkle_tree_pda.try_initialize(
        &merkle_tree_tmp_pda
    )
}


pub fn close_account(
    account: &AccountInfo,
    dest_account: &AccountInfo,
) -> Result<(), ProgramError> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(ProgramError::InvalidAccountData)?;
    **account.lamports.borrow_mut() = 0;
    Ok(())
}

pub fn sol_transfer(
    from_account: &AccountInfo,
    dest_account: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    let from_starting_lamports = from_account.lamports();
    **from_account.lamports.borrow_mut() = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;

    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    Ok(())
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
    msg!("account_lamports: {}", account_lamports);
    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,   // from_pubkey
            passed_in_pda.key,    // to_pubkey
            account_lamports,     // lamports
            number_storage_bytes, // space
            program_id,           // owner
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