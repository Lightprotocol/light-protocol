use solana_program::{
    msg,
     pubkey::Pubkey,
};
use anchor_lang::prelude::*;
use merkle_tree_program::utils::create_pda::create_and_check_pda;
use crate::*;
use std::cell::RefMut;
use ark_ff::{
    bytes::FromBytes,
    BigInteger256
};
use crate::ErrorCode;
use anchor_lang::error::Error;
use ark_ff::BigInteger;
use ark_ed_on_bn254::FqParameters;
use ark_ff::FpParameters;

use merkle_tree_program::{self, program::MerkleTreeProgram};



pub fn process_last_transaction(
    ctx: Context<LastTransaction>

) -> Result<()> {

    // let merkle_tree_pda = next_account_info(account)?;
    // let merkle_tree_pda_token = next_account_info(account)?;
    //
    //
    // let authority = next_account_info(account)?;
    // let authority_seed = program_id.to_bytes();
    // let (expected_authority_pubkey, authority_bump_seed) =
    //     Pubkey::find_program_address(&[&authority_seed], program_id);
    let verifier_state= &mut ctx.accounts.verifier_state.load_mut()?;

    if !verifier_state.last_transaction {
        msg!("Wrong state");
        // return Err(NotLastTransactionState);
    }

    let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;

    let (pub_amount_checked, relayer_fee) = check_external_amount(verifier_state)?;
    let ext_amount =
        i64::from_le_bytes(verifier_state.ext_amount);
    msg!("0 != pub_amount_checked: 0 != {}", pub_amount_checked);

    if 0 != pub_amount_checked {
        if ext_amount > 0 {
            // let user_pda_token = next_account_info(account)?;
            create_and_check_pda(
                &ctx.program_id,
                &ctx.accounts.user_account.to_account_info(),
                &ctx.accounts.escrow_pda.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &rent,
                &ctx.accounts.verifier_state.to_account_info().key.to_bytes()[..],
                &b"escrow"[..],
                0,                                                    //bytes
                <u64 as TryFrom<i64>>::try_from(ext_amount).unwrap(), // amount
                true,                                                 //rent_exempt
            )?;
            // Close escrow account to make deposit to shielded pool.
            close_account(&ctx.accounts.escrow_pda.to_account_info(), &ctx.accounts.merkle_tree_pda_token.to_account_info())?;

        } else if ext_amount < 0 {
            // let recipient_account = next_account_info(account)?;
            // if *recipient_account.key
            //     != verifier_state.recipient
            // {
            //     msg!("Recipient has to be address specified in tx integrity hash.");
            //     return err!(ErrorCode::WrongPubAmount.into());
            // }

            // Checking for wrapped sol and Merkle tree index can only be 0. This does
            // not allow multiple Merkle trees for wSol.
            if verifier_state.merkle_tree_index == 0 {
                // TODO: replace with cpi

                // sol_transfer(merkle_tree_pda_token, recipient_account, pub_amount_checked)?;
            }
        }
    }

    if verifier_state.relayer_fee > 0 {
        // TODO: replace with cpi
        // if Pubkey::new(&verifier_state.signing_address) != *ctx.accounts.signing_address.key {
        //     msg!("Wrong relayer.");
        //     return Err(ProgramError::InvalidArgument);
        // }
        // let relayer_pda_token = next_account_info(account)?;
        //
        // if verifier_state.merkle_tree_index == 0 {
        //     // TODO: replace with cpi
        //
        //     sol_transfer(merkle_tree_pda_token, relayer_pda_token, relayer_fee)?;
        // }
    }
    // TODO: replace with cpi
    msg!("Creating two_leaves_pda.");
    // create_and_check_pda(
    //     program_id,
    //     ctx.accounts.signing_address,
    //     two_leaves_pda,
    //     system_program_account,
    //     rent,
    //     &verifier_state.proof_a_b_c_leaves_and_nullifiers
    //         [NULLIFIER_0_START..NULLIFIER_0_END],
    //     &b"leaves"[..],
    //     TWO_LEAVES_PDA_SIZE, //bytes
    //     0,                   //lamports
    //     true,                //rent_exempt
    // )?;
    //arbitrary bump
    let derived_pubkey =
         Pubkey::find_program_address(&[verifier_state.tx_integrity_hash.as_ref(), b"storage"], ctx.program_id);
    msg!("derived_pubkey {:?}", derived_pubkey);
    let bump_seed = &[derived_pubkey.1][..];

    let data = [vec![0u8;32],verifier_state.encrypted_utxos.to_vec()].concat();

    let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
    let accounts = merkle_tree_program::cpi::accounts::UpdateMerkleTree {
        authority: ctx.accounts.signing_address.to_account_info(),
        merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_storage.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),

    };
    let seeds = [&[verifier_state.tx_integrity_hash.as_ref(), &b"storage"[..],  &bump_seed][..]];
    msg!("starting cpi");
    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![
            ctx.accounts.leaves_pda.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info()
        ]);
    // let cpi_ctx = CpiContext::new(merkle_tree_program_id, accounts);
    let x = merkle_tree_program::cpi::update_merkle_tree(cpi_ctx, data);

    // Close tmp account.
    close_account(&ctx.accounts.verifier_state.to_account_info(), &ctx.accounts.signing_address.to_account_info());
    Ok(())
}


#[allow(clippy::comparison_chain)]
pub fn check_external_amount(
    verifier_state: &mut RefMut<'_, VerifierState>,
) -> Result<(u64, u64)> {
    let ext_amount =
        i64::from_le_bytes(verifier_state.ext_amount);
    // ext_amount includes relayer_fee
    let relayer_fee =
        verifier_state.relayer_fee;
    // pub_amount is the public amount included in public inputs for proof verification
    let pub_amount = <BigInteger256 as FromBytes>::read(&verifier_state.amount[..]).unwrap();

    if ext_amount > 0 {
        if pub_amount.0[1] != 0 || pub_amount.0[2] != 0 || pub_amount.0[3] != 0 {
            msg!("Public amount is larger than u64.");
            return Err(ErrorCode::WrongPubAmount.into());
        }

        let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);

        if pub_amount_fits_i64.is_err() {
            msg!("Public amount is larger than i64.");
            return Err(ErrorCode::WrongPubAmount.into());
        }

        //check amount
        if pub_amount.0[0].checked_add(relayer_fee).unwrap() != ext_amount.try_into().unwrap() {
            msg!(
                "Deposit invalid external amount (relayer_fee) {} != {}",
                pub_amount.0[0] + relayer_fee,
                ext_amount
            );
            return Err(ErrorCode::WrongPubAmount.into());
        }
        Ok((ext_amount.try_into().unwrap(), relayer_fee))
    } else if ext_amount < 0 {
        // calculate ext_amount from pubAmount:
        let mut field = FqParameters::MODULUS;
        field.sub_noborrow(&pub_amount);

        // field.0[0] is the positive value
        if field.0[1] != 0 || field.0[2] != 0 || field.0[3] != 0 {
            msg!("Public amount is larger than u64.");
            return Err(ErrorCode::WrongPubAmount.into());
        }
        let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);
        if pub_amount_fits_i64.is_err() {
            msg!("Public amount is larger than i64.");
            return Err(ErrorCode::WrongPubAmount.into());
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
            return Err(ErrorCode::WrongPubAmount.into());
        }
        Ok(((-ext_amount).try_into().unwrap(), relayer_fee))
    } else if ext_amount == 0 {
        Ok((ext_amount.try_into().unwrap(), relayer_fee))
    } else {
        msg!("Invalid state checking external amount.");
        Err(ErrorCode::WrongPubAmount.into())
    }
}


pub fn close_account(
    account: &AccountInfo,
    dest_account: &AccountInfo,
) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports()).unwrap();
        // .ok_or(Err(ErrorCode::WrongPubAmount.into()))?;
    **account.lamports.borrow_mut() = 0;
    Ok(())
}
