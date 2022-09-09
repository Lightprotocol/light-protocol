use crate::errors::ErrorCode;
use crate::groth16_verifier::VerifierState;
use crate::last_transaction::instructions_last_transaction::{
    LastTransactionDeposit, LastTransactionWithdrawal,
};
use anchor_lang::prelude::*;
use ark_ed_on_bn254::FqParameters;
use ark_ff::BigInteger;
use ark_ff::FpParameters;
use ark_ff::{bytes::FromBytes, BigInteger256};
use merkle_tree_program;
use merkle_tree_program::instructions::sol_transfer;
use solana_program::msg;
use std::cell::RefMut;

use crate::last_transaction::cpi_instructions::{
    check_merkle_root_exists_cpi, initialize_nullifier_cpi, insert_two_leaves_cpi, withdraw_sol_cpi,
    withdraw_spl_cpi, deposit_spl_cpi
};

pub fn process_last_transaction_deposit<'info>(ctx: Context<'_, '_, '_, 'info, LastTransactionDeposit<'info>>) -> Result<()> {
    let verifier_state = &mut ctx.accounts.verifier_state.load_mut()?;

    if !verifier_state.last_transaction {
        msg!("Wrong state");
        return err!(ErrorCode::NotLastTransactionState);
    }

    let (pub_amount_checked, _relayer_fee) = check_external_amount(verifier_state)?;
    let ext_amount = i64::from_le_bytes(verifier_state.ext_amount);

    if ext_amount <= 0 {
        msg!(
            "Deposit function called with negative external amount {}",
            ext_amount
        );
        return err!(ErrorCode::NotLastTransactionState);
    }

    // Deposit
    if verifier_state.merkle_tree_index == 0 {
        msg!("starting sol transfer");

        sol_transfer(
            &ctx.accounts.fee_escrow_state.to_account_info(),
            &ctx.accounts.merkle_tree_pda_token.to_account_info(),
            pub_amount_checked,
        )?;
    } else {
        msg!("starting spl transfer");
        let address= anchor_lang::prelude::Pubkey::create_with_seed(
            &ctx.accounts.signing_address.key(),
            "escrow",
            &ctx.accounts.token_program.key()).unwrap();

        if ctx.remaining_accounts[0].key() != address {
            return err!(ErrorCode::IncorrectTokenEscrowAcc);
        }
        deposit_spl_cpi(
            &ctx.program_id,
            &ctx.accounts.program_merkle_tree.to_account_info(),
            &ctx.accounts.signing_address.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.remaining_accounts[0].to_account_info(),
            &ctx.accounts.merkle_tree_pda_token.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            pub_amount_checked
        )?;
    }

    let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();


    initialize_nullifier_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.nullifier0_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info().clone(),
        &ctx.accounts.rent.to_account_info().clone(),
        verifier_state.nullifier0,
    )?;

    initialize_nullifier_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.nullifier1_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info().clone(),
        &ctx.accounts.rent.to_account_info().clone(),
        verifier_state.nullifier1,
    )?;

    check_merkle_root_exists_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.merkle_tree.to_account_info(),
        verifier_state.merkle_tree_index.into(),
        verifier_state.merkle_root.clone(),
    )?;

    insert_two_leaves_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.two_leaves_pda.to_account_info(),
        &ctx.accounts.pre_inserted_leaves_index.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.rent.to_account_info(),
        verifier_state.nullifier0,
        verifier_state.leaf_left,
        verifier_state.leaf_right,
        ctx.accounts.merkle_tree.key().to_bytes(),
        verifier_state.encrypted_utxos,
    )?;


    Ok(())
}

pub fn process_last_transaction_withdrawal(ctx: Context<LastTransactionWithdrawal>) -> Result<()> {
    let verifier_state = &mut ctx.accounts.verifier_state.load_mut()?;

    if !verifier_state.last_transaction {
        msg!("Wrong state");
        return err!(ErrorCode::NotLastTransactionState);
    }

    let (pub_amount_checked, relayer_fee) = check_external_amount(verifier_state)?;
    let ext_amount = i64::from_le_bytes(verifier_state.ext_amount);
    msg!("0 != pub_amount_checked: 0 != {}", pub_amount_checked);
    if ext_amount > 0 {
        return err!(ErrorCode::NotLastTransactionState);
    }
    let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();

    if verifier_state.merkle_tree_index == 0 {
        withdraw_sol_cpi(
            &ctx.program_id,
            &merkle_tree_program_id,
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_tree_pda_token.to_account_info(),
            &ctx.accounts.recipient.to_account_info(),
            pub_amount_checked,
        )?;
    } else {
        withdraw_spl_cpi(
            &ctx.program_id,
            &merkle_tree_program_id,
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_tree_pda_token.to_account_info(),
            &ctx.accounts.recipient.to_account_info(),
            &ctx.accounts.token_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            pub_amount_checked,
            verifier_state.merkle_tree_index.into()
        )?;
    }

    if relayer_fee > 0 {
        if verifier_state.merkle_tree_index == 0 {
            withdraw_sol_cpi(
                &ctx.program_id,
                &merkle_tree_program_id,
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.merkle_tree_pda_token.to_account_info(),
                &ctx.accounts.relayer_recipient.to_account_info(),
                relayer_fee,
            )?;
        } else {
            withdraw_spl_cpi(
                &ctx.program_id,
                &merkle_tree_program_id,
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.merkle_tree_pda_token.to_account_info(),
                &ctx.accounts.relayer_recipient.to_account_info(),
                &ctx.accounts.token_authority.to_account_info(),
                &ctx.accounts.token_program.to_account_info(),
                relayer_fee,
                verifier_state.merkle_tree_index.into()
            )?;
        }
    }

    initialize_nullifier_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.nullifier0_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info().clone(),
        &ctx.accounts.rent.to_account_info().clone(),
        verifier_state.nullifier0,
    )?;
    initialize_nullifier_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.nullifier1_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info().clone(),
        &ctx.accounts.rent.to_account_info().clone(),
        verifier_state.nullifier1,
    )?;

    check_merkle_root_exists_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.merkle_tree.to_account_info(),
        verifier_state.merkle_tree_index.into(),
        verifier_state.merkle_root.clone(),
    )?;
    // Inserting leaves
    insert_two_leaves_cpi(
        &ctx.program_id,
        &merkle_tree_program_id,
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.two_leaves_pda.to_account_info(),
        &ctx.accounts.pre_inserted_leaves_index.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.rent.to_account_info(),
        verifier_state.nullifier0,
        verifier_state.leaf_left,
        verifier_state.leaf_right,
        ctx.accounts.merkle_tree.key().to_bytes(),
        verifier_state.encrypted_utxos,
    )?;
    Ok(())
}

#[allow(clippy::comparison_chain)]
pub fn check_external_amount(verifier_state: &mut RefMut<'_, VerifierState>) -> Result<(u64, u64)> {
    let ext_amount = i64::from_le_bytes(verifier_state.ext_amount);
    // ext_amount includes relayer_fee
    let relayer_fee = verifier_state.relayer_fee;
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
        if pub_amount.0[0].checked_add(relayer_fee).unwrap() != ext_amount as u64 {
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
