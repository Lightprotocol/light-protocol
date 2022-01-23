use crate::instructions::{check_and_insert_nullifier, create_and_check_account, transfer};
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::poseidon_merkle_tree::state_roots::{check_root_hash_exists, MERKLE_TREE_ACC_BYTES};
use crate::state::ChecksAndTransferState;
use std::convert::{TryFrom, TryInto};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use ark_ed_on_bn254::FqParameters;
use ark_ff::{biginteger::BigInteger256, bytes::FromBytes, fields::FpParameters, BigInteger};

// Processor for deposit and withdraw logic.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    current_instruction_index: usize,
) -> Result<(), ProgramError> {
    msg!("Entered process_instruction");

    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;
    let tmp_storage_pda = next_account_info(account)?;
    let mut tmp_storage_pda_data = ChecksAndTransferState::unpack(&tmp_storage_pda.data.borrow())?;

    // Checks whether passed-in root exists in Merkle tree history array.
    // We do this check as soon as possible to avoid proof transaction invalidation for missing
    // root. Currently 500 roots are stored at once. After 500 transactions roots are overwritten.
    if current_instruction_index == 1 {
        let merkle_tree_pda = next_account_info(account)?;
        msg!(
            "Passed-in merkle_tree_pda pubkey: {:?}",
            *merkle_tree_pda.key
        );
        msg!(
            "Checks against hardcoded merkle_tree_pda pubkey: {:?}",
            solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..])
        );
        tmp_storage_pda_data.found_root = check_root_hash_exists(
            merkle_tree_pda,
            &tmp_storage_pda_data.root_hash,
            &program_id,
        )?;
    }
    // Checks and inserts nullifier pdas, two Merkle tree leaves (output utxo hashes),
    // and executes transaction, deposit or withdrawal.
    else if current_instruction_index == 1502 {
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0_pda = next_account_info(account)?;
        let nullifier1_pda = next_account_info(account)?;
        let merkle_tree_pda = next_account_info(account)?;
        let system_program_account = next_account_info(account)?;

        msg!("Starting nullifier check.");
        tmp_storage_pda_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            signer_account,
            nullifier0_pda,
            system_program_account,
            &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers[320..352],
        )?;
        msg!(
            "nullifier0_pda inserted {}",
            tmp_storage_pda_data.found_nullifier
        );

        tmp_storage_pda_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            signer_account,
            nullifier1_pda,
            system_program_account,
            &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers[352..384],
        )?;
        msg!(
            "nullifier1_pda inserted {}",
            tmp_storage_pda_data.found_nullifier
        );

        msg!("Inserting new merkle root.");
        let mut merkle_tree_processor = MerkleTreeProcessor::new(Some(tmp_storage_pda), None)?;

        // ext_amount includes the substracted fees
        //TODO implement fees
        let ext_amount =
            i64::from_le_bytes(tmp_storage_pda_data.ext_amount.clone().try_into().unwrap());
        // pub_amount is the public amount included in public inputs for proof verification
        let pub_amount =
            <BigInteger256 as FromBytes>::read(&tmp_storage_pda_data.amount[..]).unwrap();

        if ext_amount > 0 {
            if *merkle_tree_pda.key != solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES) {
                msg!("Recipient has to be merkle tree account for deposit.");
                return Err(ProgramError::InvalidInstructionData);
            }

            if pub_amount.0[1] != 0 || pub_amount.0[2] != 0 || pub_amount.0[3] != 0 {
                msg!("Public amount is larger than u64.");
                return Err(ProgramError::InvalidInstructionData);
            }

            let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);
            if pub_amount_fits_i64.is_err() == true {
                msg!("Public amount is larger than i64.");
                return Err(ProgramError::InvalidInstructionData);
            }

            if u64::try_from(ext_amount).unwrap() != pub_amount.0[0] {
                msg!("ext_amount != pub_amount");
                return Err(ProgramError::InvalidInstructionData);
            }

            msg!("Creating two_leaves_pda.");
            create_and_check_account(
                program_id,
                signer_account,
                two_leaves_pda,
                system_program_account,
                &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers[320..352],
                &b"leaves"[..],
                106u64,                             //bytes
                u64::try_from(ext_amount).unwrap(), //lamports
                true,                               //rent_exempt
            )?;
            msg!("Created two_leaves_pda successfully.");

            msg!("Deposited {}", ext_amount);
            transfer(
                two_leaves_pda,
                merkle_tree_pda,
                u64::try_from(ext_amount).unwrap(),
            )?;
        } else if ext_amount <= 0 {
            let recipient_account = next_account_info(account)?;

            if *recipient_account.key
                != solana_program::pubkey::Pubkey::new(&tmp_storage_pda_data.to_address)
            {
                msg!("Recipient has to be address specified in tx integrity hash.");
                return Err(ProgramError::InvalidInstructionData);
            }

            msg!("Creating two_leaves_pda.");
            create_and_check_account(
                program_id,
                signer_account,
                two_leaves_pda,
                system_program_account,
                &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers[320..352],
                &b"leaves"[..],
                106u64, //bytes
                0u64,   //lamports
                true,   //rent_exempt
            )?;
            msg!("Created two_leaves_pda successfully.");

            // calculate ext_amount from pubAmount:
            let mut field = FqParameters::MODULUS;
            field.sub_noborrow(&pub_amount);

            if field.0[1] != 0 || field.0[2] != 0 || field.0[3] != 0 {
                msg!("Public amount is larger than u64.");
                return Err(ProgramError::InvalidInstructionData);
            }
            let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);
            if pub_amount_fits_i64.is_err() {
                msg!("Public amount is larger than i64.");
                return Err(ProgramError::InvalidInstructionData);
            }
            // field is the positive value
            let ext_amount_from_pub = field.0[0];
            if u64::try_from(-ext_amount).unwrap() != ext_amount_from_pub {
                msg!("ext_amount != pub_amount");
                return Err(ProgramError::InvalidInstructionData);
            }
            transfer(merkle_tree_pda, recipient_account, ext_amount_from_pub)?;
        }

        //insert Merkle root
        merkle_tree_processor.process_instruction(accounts)?;
    }

    tmp_storage_pda_data.current_instruction_index += 1;
    ChecksAndTransferState::pack_into_slice(
        &tmp_storage_pda_data,
        &mut tmp_storage_pda.data.borrow_mut(),
    );
    Ok(())
}
