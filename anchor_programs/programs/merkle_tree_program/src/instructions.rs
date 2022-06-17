use crate::state::MerkleTreeTmpPda;

use anchor_lang::solana_program::{
    account_info::{next_account_info, AccountInfo},
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    msg
};
use std::cell::RefMut;
use anchor_lang::prelude::Error;
use crate::ErrorCode;

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
/*
pub struct MerkleTreeTmpStorageAccInputData {
    pub node_left: Vec<u8>,
    pub node_right: Vec<u8>,
    pub relayer: Vec<u8>,
    pub merkle_tree_pda_pubkey: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub found_root: u8,
    pub is_initialized: u8,
}

impl MerkleTreeTmpStorageAccInputData {
    pub fn new(
        node_left: Vec<u8>,
        node_right: Vec<u8>,
        root_hash: Vec<u8>,
        merkle_tree_address: Vec<u8>,
        signing_address: Vec<u8>,
    ) -> Result<MerkleTreeTmpStorageAccInputData, ProgramError> {
        Ok(MerkleTreeTmpStorageAccInputData {
            node_left: node_left,
            node_right: node_right,
            relayer: signing_address,
            merkle_tree_pda_pubkey: merkle_tree_address,
            root_hash: root_hash,
            is_initialized: 1u8,
            found_root: 0u8,
        })
    }

    pub fn return_ix_data(&self) -> Result<Vec<u8>, ProgramError> {
        let res = [
            self.node_left.clone(),
            self.node_right.clone(),
            self.root_hash.clone(),
            self.relayer.clone(),
            self.merkle_tree_pda_pubkey.clone(),
        ]
        .concat();
        Ok(res)
    }

    pub fn try_initialize(&mut self, account: &mut RefMut<'_, MerkleTreeTmpPda>) -> Result<(), ProgramError> {
        account.node_left = self.node_left.clone().try_into().unwrap();
        account.node_right = self.node_right.clone().try_into().unwrap();
        account.leaf_left = self.node_left.clone().try_into().unwrap();
        account.leaf_right = self.node_right.clone().try_into().unwrap();
        account.relayer = self.relayer.clone().try_into().unwrap();
        account.root_hash = self.root_hash.clone().try_into().unwrap();
        account.merkle_tree_pda_pubkey = self.merkle_tree_pda_pubkey.clone().try_into().unwrap();
        Ok(())
    }
}

#[allow(clippy::clone_double_ref)]
pub fn create_and_try_initialize_tmp_storage_pda(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
    // tx_integrity_hash: &[u8]
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    // let verifier_tmp_pda = next_account_info(account)?;
    // TODO: check owner
    let merkle_tree_tmp_pda = next_account_info(account)?;
    // let system_program_info = next_account_info(account)?;
    // let rent_sysvar_info = next_account_info(account)?;
    // let rent = &Rent::from_account_info(rent_sysvar_info)?;
    // msg!("MerkleTreeTmpStorageAccInputData started");
    //
    let mut merkle_tree_pda = MerkleTreeTmpStorageAccInputData::new(
        _instruction_data[0..32].to_vec(),
        _instruction_data[32..64].to_vec(),
        _instruction_data[64..96].to_vec(),
        merkle_tree_tmp_pda.key.to_bytes().to_vec(),
        signer_account.key.to_bytes().to_vec(),
    )?;
    // msg!("MerkleTreeTmpStorageAccInputData done");
    //
    // create_and_check_pda(
    //     program_id,
    //     signer_account,
    //     merkle_tree_tmp_pda,
    //     system_program_info,
    //     rent,
    //     &tx_integrity_hash[..],
    //     &b"storage"[..],
    //     MERKLE_TREE_TMP_PDA_SIZE.try_into().unwrap(),   //bytes
    //     0,                          //lamports
    //     true,                       //rent_exempt
    // )?;
    // msg!("created_pda");
    merkle_tree_pda.try_initialize(merkle_tree_tmp_pda)
}
*/
pub fn close_account(
    account: &AccountInfo,
    dest_account: &AccountInfo,
) -> Result<(), Error> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(ErrorCode::CloseAccountFailed)?;
    **account.lamports.borrow_mut() = 0;
    Ok(())
}

pub fn sol_transfer(
    from_account: &AccountInfo,
    dest_account: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    let from_starting_lamports = from_account.lamports();
    msg!("from_starting_lamports: {}", from_starting_lamports);
    let res = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    **from_account.lamports.borrow_mut() = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    msg!("from_starting_lamports: {}", res);

    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    let res = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    msg!("from_starting_lamports: {}", res);

    Ok(())
}
