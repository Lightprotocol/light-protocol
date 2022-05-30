use crate::instructions_last_transaction::{LastTransactionDeposit, LastTransactionWithdrawal};
use crate::ErrorCode;
use crate::*;
use anchor_lang::prelude::*;
use ark_ed_on_bn254::FqParameters;
use ark_ff::BigInteger;
use ark_ff::FpParameters;
use ark_ff::{bytes::FromBytes, BigInteger256};
use merkle_tree_program;
use merkle_tree_program::utils::create_pda::create_and_check_pda;
use solana_program::{msg, pubkey::Pubkey};
use std::cell::RefMut;

pub fn process_last_transaction_deposit(ctx: Context<LastTransactionDeposit>) -> Result<()> {
    let verifier_state = &mut ctx.accounts.verifier_state.load_mut()?;

    if !verifier_state.last_transaction {
        msg!("Wrong state");
        return err!(ErrorCode::NotLastTransactionState);
    }

    let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;

    let (pub_amount_checked, _relayer_fee) = check_external_amount(verifier_state)?;
    let ext_amount = i64::from_le_bytes(verifier_state.ext_amount);

    if ext_amount < 0 {
        msg!(
            "Deposit function called with negative external amount {}",
            ext_amount
        );
        return err!(ErrorCode::NotLastTransactionState);
    }
    // Deposit
    create_and_check_pda(
        &ctx.program_id,
        &ctx.accounts.user_account.to_account_info(),
        &ctx.accounts.escrow_pda.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &rent,
        &ctx.accounts.verifier_state.to_account_info().key.to_bytes()[..],
        &b"escrow"[..],
        0,                  //bytes
        pub_amount_checked, // amount
        true,               //rent_exempt
    )?;
    // Close escrow account to make deposit to shielded pool.
    close_account(
        &ctx.accounts.escrow_pda.to_account_info(),
        &ctx.accounts.merkle_tree_pda_token.to_account_info(),
    )?;

    // Inserting leaves and root
    let derived_pubkey = Pubkey::find_program_address(
        &[verifier_state.tx_integrity_hash.as_ref(), b"storage"],
        ctx.program_id,
    );
    msg!("derived_pubkey {:?}", derived_pubkey);
    let bump_seed = &[derived_pubkey.1][..];

    let data = [vec![0u8; 32], verifier_state.encrypted_utxos.to_vec()].concat();

    let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
    let accounts = merkle_tree_program::cpi::accounts::UpdateMerkleTree {
        authority: ctx.accounts.signing_address.to_account_info(),
        merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_storage.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
    };
    let seeds = [&[
        verifier_state.tx_integrity_hash.as_ref(),
        &b"storage"[..],
        &bump_seed,
    ][..]];
    msg!("starting cpi");
    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![
        ctx.accounts.leaves_pda.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.rent.to_account_info(),
    ]);
    merkle_tree_program::cpi::update_merkle_tree(cpi_ctx, data)?;

    // Close tmp account.
    close_account(
        &ctx.accounts.verifier_state.to_account_info(),
        &ctx.accounts.signing_address.to_account_info(),
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

    if verifier_state.merkle_tree_index == 0 {
        let derived_pubkey = Pubkey::find_program_address(
            &[verifier_state.tx_integrity_hash.as_ref(), b"storage"],
            ctx.program_id,
        );
        let bump_seed = &[derived_pubkey.1][..];

        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
        let accounts = merkle_tree_program::cpi::accounts::WithdrawSOL {
            authority: ctx.accounts.signing_address.to_account_info(),
            merkle_tree_token: ctx.accounts.merkle_tree_pda_token.to_account_info(),
        };
        let seeds = [&[
            verifier_state.tx_integrity_hash.as_ref(),
            &b"storage"[..],
            &bump_seed,
        ][..]];
        msg!(
            "Starting cpi transfer of {} to recipient: {:?}",
            pub_amount_checked,
            ctx.accounts.recipient.to_account_info().key
        );
        let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
        cpi_ctx = cpi_ctx.with_remaining_accounts(vec![ctx.accounts.recipient.to_account_info()]);
        let amount = pub_amount_checked.to_le_bytes().to_vec();
        merkle_tree_program::cpi::withdraw_sol(cpi_ctx, amount)?;
    } else {
        panic!("Spl-Token transfers not implemented yet");
    }

    if relayer_fee > 0 {
        if verifier_state.merkle_tree_index == 0 {
            let derived_pubkey = Pubkey::find_program_address(
                &[verifier_state.tx_integrity_hash.as_ref(), b"storage"],
                ctx.program_id,
            );
            let bump_seed = &[derived_pubkey.1][..];

            let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
            let accounts = merkle_tree_program::cpi::accounts::WithdrawSOL {
                authority: ctx.accounts.signing_address.to_account_info(),
                merkle_tree_token: ctx.accounts.merkle_tree_pda_token.to_account_info(),
            };
            let seeds = [&[
                verifier_state.tx_integrity_hash.as_ref(),
                &b"storage"[..],
                &bump_seed,
            ][..]];
            msg!(
                "Starting cpi transfer of {} to relayer {:?}",
                relayer_fee,
                ctx.accounts.relayer_recipient.to_account_info()
            );
            let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
            cpi_ctx = cpi_ctx
                .with_remaining_accounts(vec![ctx.accounts.relayer_recipient.to_account_info()]);
            let amount = relayer_fee.to_le_bytes().to_vec();
            merkle_tree_program::cpi::withdraw_sol(cpi_ctx, amount)?;
        }
    }
    // Inserting leaves and root
    let derived_pubkey = Pubkey::find_program_address(
        &[verifier_state.tx_integrity_hash.as_ref(), b"storage"],
        ctx.program_id,
    );
    msg!("derived_pubkey {:?}", derived_pubkey);
    let bump_seed = &[derived_pubkey.1][..];

    let data = [vec![0u8; 32], verifier_state.encrypted_utxos.to_vec()].concat();

    let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();
    let accounts = merkle_tree_program::cpi::accounts::UpdateMerkleTree {
        authority: ctx.accounts.signing_address.to_account_info(),
        merkle_tree_tmp_storage: ctx.accounts.merkle_tree_tmp_storage.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
    };
    let seeds = [&[
        verifier_state.tx_integrity_hash.as_ref(),
        &b"storage"[..],
        &bump_seed,
    ][..]];
    msg!("starting cpi");
    let mut cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id, accounts, &seeds);
    cpi_ctx = cpi_ctx.with_remaining_accounts(vec![
        ctx.accounts.leaves_pda.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.rent.to_account_info(),
    ]);
    merkle_tree_program::cpi::update_merkle_tree(cpi_ctx, data)?;

    // Close tmp account.
    close_account(
        &ctx.accounts.verifier_state.to_account_info(),
        &ctx.accounts.signing_address.to_account_info(),
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

pub fn close_account(account: &AccountInfo, dest_account: &AccountInfo) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .unwrap();
    // .ok_or(Err(ErrorCode::WrongPubAmount.into()))?;
    **account.lamports.borrow_mut() = 0;
    Ok(())
}
