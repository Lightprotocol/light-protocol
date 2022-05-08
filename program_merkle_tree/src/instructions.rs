use ark_ed_on_bn254::Fq;
use ark_ff::PrimeField;

use crate::state::MerkleTreeTmpPda;
use crate::utils::config::{
    ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_ACC_BYTES_ARRAY, TMP_STORAGE_ACCOUNT_TYPE, MERKLE_TREE_TMP_PDA_SIZE
};
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
    tmp_storage_pda_data: &MerkleTreeTmpPda,
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
    pub root_hash: Vec<u8>,
    pub amount:  Vec<u8>,
    pub tx_integrity_hash:  Vec<u8>,
    pub nullifiers:  Vec<u8>,
    pub node_left:  Vec<u8>,
    pub node_right:  Vec<u8>,
    pub recipient:  Vec<u8>,
    pub ext_amount:  Vec<u8>,
    pub relayer_fee:  Vec<u8>,
    pub ext_sol_amount:  Vec<u8>,
    pub verifier_index: Vec<u8>,
    pub encrypted_utxos:  Vec<u8>,
    pub verifier_tmp_pda:  Vec<u8>,
    pub relayer:  Vec<u8>,
    pub merkle_tree_index: Vec<u8>,
    pub found_root: u8,
    pub is_initialized: u8,
}

impl MerkleTreeTmpStorageAccInputData {
    pub fn new(data: &[u8],
            merkle_tree_address: Vec<u8>,
            signing_address: Vec<u8>,
            verifier_tmp_pda: Vec<u8>) -> Result<MerkleTreeTmpStorageAccInputData, ProgramError> {
        Ok(MerkleTreeTmpStorageAccInputData {
            root_hash:          data[0..32].to_vec(),
            amount:             data[32..64].to_vec(),
            tx_integrity_hash:  data[64..96].to_vec(),
            nullifiers:         data[96..160].to_vec(),
            node_left:          data[160..192].to_vec(),
            node_right:         data[192..224].to_vec(),
            recipient:          data[224..256].to_vec(),
            ext_amount:         data[256..264].to_vec(),
            relayer_fee:        data[264..272].to_vec(),
            ext_sol_amount:     data[272..304].to_vec(),
            verifier_index:     data[304..312].to_vec(),
            merkle_tree_index:  data[312..320].to_vec(),
            encrypted_utxos:    data[320..320 + ENCRYPTED_UTXOS_LENGTH].to_vec(),
            verifier_tmp_pda:   verifier_tmp_pda,
            relayer:    signing_address,
            is_initialized: 1u8,
            found_root: 0u8,
        })
    }

    pub fn return_ix_data(&self) ->  Result<Vec<u8>, ProgramError>{
        assert_eq!(self.node_left.to_vec(), vec![2u8;32]);

        let res = [
        self.root_hash.clone(),
        self.amount.clone(),
        self.tx_integrity_hash.clone(),
        self.nullifiers.clone(),
        self.node_left.clone(),
        self.node_right.clone(),
        self.recipient.clone(),
        self.ext_amount.clone(),
        self.relayer_fee.clone(),
        self.ext_sol_amount.clone(),
        self.verifier_index.clone(),
        self.merkle_tree_index.clone(),
        self.encrypted_utxos.clone(),
        self.verifier_tmp_pda.clone(),
        self.relayer.clone()
        ].concat();
        Ok(res)
    }

    fn check_tx_integrity_hash(&mut self) -> Result<(), ProgramError> {
        let input = [
            self.recipient.clone(),
            self.ext_amount.clone(),
            self.relayer.clone(),
            self.relayer_fee.clone(),
            self.merkle_tree_index.clone(),
            self.verifier_index.clone(),
            self.encrypted_utxos.clone(),
        ]
        .concat();
        msg!("integrity_hash inputs: {:?}", input);
        let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;
        msg!("hash computed {:?}", hash);

        if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&self.tx_integrity_hash) {
            msg!(
                "tx_integrity_hash verification failed.{:?} != {:?}",
                &hash[..],
                &self.tx_integrity_hash
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(())
    }

    pub fn try_initialize(&mut self, account: &AccountInfo) -> Result<(), ProgramError> {
        let mut tmp = MerkleTreeTmpPda::new();
        tmp.root_hash = self.root_hash.clone();
        tmp.amount = self.amount.clone();
        tmp.tx_integrity_hash = self.tx_integrity_hash.clone();
        tmp.nullifiers = self.nullifiers.clone();
        tmp.node_left = self.node_left.clone();
        tmp.node_right = self.node_right.clone();
        tmp.leaf_left = self.node_left.clone();
        tmp.leaf_right = self.node_right.clone();
        tmp.ext_amount = self.ext_amount.clone();
        tmp.relayer_fee = self.relayer_fee.clone();
        tmp.ext_sol_amount = self.ext_sol_amount.clone();
        tmp.verifier_index = usize::from_le_bytes(self.verifier_index.clone().try_into().unwrap());
        tmp.encrypted_utxos = self.encrypted_utxos.clone();
        tmp.verifier_tmp_pda = self.verifier_tmp_pda.clone();
        tmp.relayer = self.relayer.clone();
        tmp.merkle_tree_index = usize::from_le_bytes(self.merkle_tree_index.clone().try_into().unwrap());
        tmp.found_root = self.found_root.clone();
        tmp.recipient = self.recipient.clone();
        tmp.changed_state = 1;
        assert_eq!(tmp.node_left , [2u8;32]);
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
        _instruction_data,
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
        &merkle_tree_pda.tx_integrity_hash,
        &b"storage"[..],
        MERKLE_TREE_TMP_PDA_SIZE.try_into().unwrap(),   //bytes
        0,                          //lamports
        true,                       //rent_exempt
    )?;
    msg!("created_pda");
    merkle_tree_pda.check_tx_integrity_hash()?;
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
