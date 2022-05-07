use crate::instructions::{
    check_external_amount, close_account, create_and_check_pda,
    sol_transfer, token_transfer,
};
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::poseidon_merkle_tree::state_roots::check_root_hash_exists;
use crate::state::MerkleTreeTmpPda;
use crate::utils::config::MERKLE_TREE_ACC_BYTES_ARRAY;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use std::convert::{TryFrom, TryInto};

use crate::{
    NULLIFIER_0_END, NULLIFIER_0_START, NULLIFIER_1_END, NULLIFIER_1_START, TWO_LEAVES_PDA_SIZE,
};
// Processor for deposit and withdraw logic.
#[allow(clippy::comparison_chain)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    current_instruction_index: usize,
) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;
    let tmp_storage_pda = next_account_info(account)?;
    let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;

    // Checks whether passed-in root exists in Merkle tree history array.
    // We do this check as soon as possible to avoid proof transaction invalidation for missing
    // root. Currently 500 roots are stored at once. After 500 transactions roots are overwritten.
    if current_instruction_index == 1 {
        let merkle_tree_pda = next_account_info(account)?;
        tmp_storage_pda_data.found_root = check_root_hash_exists(
            merkle_tree_pda,
            &tmp_storage_pda_data.root_hash,
            program_id,
            tmp_storage_pda_data.merkle_tree_index,
        )?;
        tmp_storage_pda_data.changed_state = 1;
        tmp_storage_pda_data.current_instruction_index += 1;
        MerkleTreeTmpPda::pack_into_slice(
            &tmp_storage_pda_data,
            &mut tmp_storage_pda.data.borrow_mut(),
        );
    }
    // Checks and inserts nullifier pdas, two Merkle tree leaves (output utxo hashes),
    // executes transaction, deposit or withdrawal, and closes the tmp account.
    else if current_instruction_index == 1501 {
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0_pda = next_account_info(account)?;
        let nullifier1_pda = next_account_info(account)?;
        let merkle_tree_pda = next_account_info(account)?;
        let merkle_tree_pda_token = next_account_info(account)?;
        let system_program_account = next_account_info(account)?;
        let token_program_account = next_account_info(account)?;
        let rent_sysvar_info = next_account_info(account)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;

        let authority = next_account_info(account)?;
        let authority_seed = program_id.to_bytes();
        let (expected_authority_pubkey, authority_bump_seed) =
            Pubkey::find_program_address(&[&authority_seed], program_id);

        if expected_authority_pubkey != *authority.key {
            msg!("Invalid passed-in authority.");
            return Err(ProgramError::InvalidArgument);
        }

        if tmp_storage_pda_data.found_root != 1u8 {
            msg!("Root was not found. {}", tmp_storage_pda_data.found_root);
            return Err(ProgramError::InvalidArgument);
        }

        if *merkle_tree_pda.key
            != solana_program::pubkey::Pubkey::new(
                &MERKLE_TREE_ACC_BYTES_ARRAY[
                    tmp_storage_pda_data.merkle_tree_index
                ]
                .0,
            )
        {
            msg!(
                "Passed-in Merkle tree account is invalid. {:?} != {:?}",
                *merkle_tree_pda.key,
                solana_program::pubkey::Pubkey::new(
                    &MERKLE_TREE_ACC_BYTES_ARRAY[
                        tmp_storage_pda_data.merkle_tree_index
                    ]
                    .0
                )
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        if *merkle_tree_pda.owner != *program_id {
            msg!("Invalid merkle tree owner.");
            return Err(ProgramError::IllegalOwner);
        }

        if *merkle_tree_pda_token.key
            != solana_program::pubkey::Pubkey::new(
                &MERKLE_TREE_ACC_BYTES_ARRAY[
                    tmp_storage_pda_data.merkle_tree_index
                ]
                .1,
            )
        {
            msg!(
                "Passed-in Merkle tree token account is invalid. {:?} != {:?}",
                merkle_tree_pda_token.key.to_bytes(),
                &MERKLE_TREE_ACC_BYTES_ARRAY[
                    tmp_storage_pda_data.merkle_tree_index
                ]
                .1
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        msg!("Starting nullifier check.");
        // tmp_storage_pda_data.account_type = check_and_insert_nullifier(
        //     program_id,
        //     signer_account,
        //     nullifier0_pda,
        //     system_program_account,
        //     rent,
        //     &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers
        //         [NULLIFIER_0_START..NULLIFIER_0_END],
        // )?;
        msg!(
            "nullifier0_pda inserted: {}",
            tmp_storage_pda_data.account_type
        );

        // tmp_storage_pda_data.account_type = check_and_insert_nullifier(
        //     program_id,
        //     signer_account,
        //     nullifier1_pda,
        //     system_program_account,
        //     rent,
        //     &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers
        //         [NULLIFIER_1_START..NULLIFIER_1_END],
        // )?;
        msg!(
            "nullifier1_pda inserted: {}",
            tmp_storage_pda_data.account_type
        );
        let (pub_amount_checked, relayer_fee) = check_external_amount(&tmp_storage_pda_data)?;
        let ext_amount =
            i64::from_le_bytes(tmp_storage_pda_data.ext_amount.clone().try_into().unwrap());
        msg!("0 != pub_amount_checked: 0 != {}", pub_amount_checked);

        if 0 != pub_amount_checked {
            if ext_amount > 0 {
                let user_pda_token = next_account_info(account)?;

                if tmp_storage_pda_data.merkle_tree_index == 0 {
                    // Create escrow account which is program owned.
                    // The ext_amount is transferred since we might want to charge relayer fees.
                    create_and_check_pda(
                        program_id,
                        signer_account,
                        user_pda_token,
                        system_program_account,
                        rent,
                        &tmp_storage_pda.key.to_bytes(),
                        &b"escrow"[..],
                        0,                                                    //bytes
                        <u64 as TryFrom<i64>>::try_from(ext_amount).unwrap(), // amount
                        true,                                                 //rent_exempt
                    )?;
                    // Close escrow account to make deposit to shielded pool.
                    close_account(user_pda_token, merkle_tree_pda_token)?;
                } else {
                    token_transfer(
                        token_program_account,
                        user_pda_token,
                        merkle_tree_pda_token,
                        authority,
                        &authority_seed[..],
                        &[authority_bump_seed],
                        <u64 as TryFrom<i64>>::try_from(ext_amount).unwrap(),
                    )?;
                    msg!("Deposited {}", pub_amount_checked);
                }
            } else if ext_amount < 0 {
                let recipient_account = next_account_info(account)?;
                if *recipient_account.key
                    != solana_program::pubkey::Pubkey::new(&tmp_storage_pda_data.recipient)
                {
                    msg!("Recipient has to be address specified in tx integrity hash.");
                    return Err(ProgramError::InvalidInstructionData);
                }

                // Checking for wrapped sol and Merkle tree index can only be 0. This does
                // not allow multiple Merkle trees for wSol.
                if tmp_storage_pda_data.merkle_tree_index == 0 {
                    sol_transfer(merkle_tree_pda_token, recipient_account, pub_amount_checked)?;
                } else {
                    msg!("withdrawing tokens");

                    token_transfer(
                        token_program_account,
                        merkle_tree_pda_token,
                        recipient_account,
                        authority,
                        &authority_seed[..],
                        &[authority_bump_seed],
                        pub_amount_checked,
                    )?;
                }
            }
        }

        if relayer_fee > 0 {
            if Pubkey::new(&tmp_storage_pda_data.relayer) != *signer_account.key {
                msg!("Wrong relayer.");
                return Err(ProgramError::InvalidArgument);
            }
            let relayer_pda_token = next_account_info(account)?;

            if tmp_storage_pda_data.merkle_tree_index == 0 {
                sol_transfer(merkle_tree_pda_token, relayer_pda_token, relayer_fee)?;
            } else {
                msg!("withdrawing tokens");

                token_transfer(
                    token_program_account,
                    merkle_tree_pda_token,
                    relayer_pda_token,
                    authority,
                    &authority_seed[..],
                    &[authority_bump_seed],
                    relayer_fee,
                )?;
            }
        }
        panic!("commented create two_leaves_pda");
        /*
        msg!("Creating two_leaves_pda.");
        create_and_check_pda(
            program_id,
            signer_account,
            two_leaves_pda,
            system_program_account,
            rent,
            &tmp_storage_pda_data.proof_a_b_c_leaves_and_nullifiers
                [NULLIFIER_0_START..NULLIFIER_0_END],
            &b"leaves"[..],
            TWO_LEAVES_PDA_SIZE, //bytes
            0,                   //lamports
            true,                //rent_exempt
        )?;*/

        msg!("Inserting new merkle root.");
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(Some(tmp_storage_pda), None, *program_id)?;
        merkle_tree_processor.process_instruction(accounts)?;
        // Close tmp account.
        close_account(tmp_storage_pda, signer_account)?;
    }

    Ok(())
}
