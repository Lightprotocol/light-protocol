use crate::errors::ErrorCode;
use crate::escrow::escrow_state::FeeEscrowState;
use crate::groth16_verifier::VerifierState;
use crate::utils::config::{FEE_PER_INSTRUCTION, TIMEOUT_ESCROW};
use anchor_lang::prelude::*;
use merkle_tree_program::instructions::sol_transfer;

use anchor_lang::solana_program::{clock::Clock, sysvar::Sysvar};
use anchor_spl::token::{Token, Transfer};
use solana_program::program_pack::Pack;
use merkle_tree_program::ID;

#[derive(Accounts)]
pub struct CloseFeeEscrowPda<'info> {
    #[account(mut, seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), b"escrow"], bump, close = relayer)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(mut, seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), b"storage"], bump, close = relayer)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    /// Signer is either the user or relayer.
    #[account(mut)]
    pub signing_address: Signer<'info>,
    #[account(mut, constraint= user.key() == fee_escrow_state.user_pubkey)]
    /// CHECK:` that the user account is the same which deposited.
    pub user: AccountInfo<'info>,
    #[account(mut, constraint=relayer.key() == fee_escrow_state.relayer_pubkey )]
    /// CHECK:` that the relayer account is consistent.
    pub relayer: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK:`
    #[account(mut)]
    pub token_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_close_escrow<'info>(ctx: Context<'_, '_, '_, 'info, CloseFeeEscrowPda<'info>>) -> Result<()> {
    let fee_escrow_state = &ctx.accounts.fee_escrow_state;
    let verifier_state = &mut ctx.accounts.verifier_state.load()?;

    let external_amount: i64 = i64::from_le_bytes(verifier_state.ext_amount);
    // escrow is only applied for deposits
    if external_amount < 0 {
        return err!(ErrorCode::NotDeposit);
    }

    // if yes check that signer such that user can only close after timeout
    if verifier_state.current_instruction_index != 0
        && fee_escrow_state.creation_slot + TIMEOUT_ESCROW > <Clock as Sysvar>::get()?.slot
        && ctx.accounts.signing_address.key() != Pubkey::new(&[0u8; 32])
    {
        if ctx.accounts.signing_address.key() != verifier_state.signing_address {
            return err!(ErrorCode::NotTimedOut);
        }
    }

    // transfer remaining funds after subtracting the fee
    // for the number of executed transactions to the user
    let transfer_amount_relayer = verifier_state.current_instruction_index * FEE_PER_INSTRUCTION;
    msg!("transfer_amount_relayer: {}", transfer_amount_relayer);
    sol_transfer(
        &fee_escrow_state.to_account_info(),
        &ctx.accounts.relayer.to_account_info(),
        transfer_amount_relayer.try_into().unwrap(),
    )?;

    // Transfer remaining funds after subtracting the fee
    // for the number of executed transactions to the user
    if verifier_state.merkle_tree_index == 0 {
        let transfer_amount_user: u64 = fee_escrow_state.relayer_fee + fee_escrow_state.tx_fee
            - transfer_amount_relayer as u64
            + external_amount as u64;

        msg!("transfer_amount_user sol: {}", transfer_amount_user);
        sol_transfer(
            &fee_escrow_state.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            transfer_amount_user.try_into().unwrap(),
        )?;

    } else if ctx.remaining_accounts.len() == 2 {
        let transfer_amount_user: u64 = fee_escrow_state.relayer_fee + fee_escrow_state.tx_fee
            - transfer_amount_relayer as u64;
        msg!("transfer_amount_user sol: {}", transfer_amount_user);
        sol_transfer(
            &fee_escrow_state.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            transfer_amount_user.try_into().unwrap(),
        )?;

        msg!("transfer_amount_user spl: {}", external_amount);
        let accounts = &mut ctx.remaining_accounts.iter();
        let from = next_account_info(accounts)?;
        let to = next_account_info(accounts)?;
        
        if fee_escrow_state.user_token_pda != to.key() {
            return err!(ErrorCode::WrongUserTokenPda);
        }

        let address= solana_program::pubkey::Pubkey::create_with_seed(
            &ctx.accounts.relayer.key(),
            "escrow",
            &ctx.accounts.token_program.key()).unwrap();

        // Check that the sender is the correct token account and owned by the program.
        if from.key() != address {
            return err!(ErrorCode::IncorrectTokenEscrowAcc);
        }

        spl_token::state::Account::unpack(&from.data.borrow())?;
        spl_token::state::Account::unpack(&to.data.borrow())?;

        let seed = ID.to_bytes();
        let (_, bump) = solana_program::pubkey::Pubkey::find_program_address(
            &[seed.as_ref()],
            ctx.program_id,
        );


        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];
        let accounts = Transfer {
            from:       from.clone(),
            to:         to.clone(),
            authority:  ctx.accounts.token_authority.to_account_info()
        };
        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), accounts, seeds);
        anchor_spl::token::transfer(cpi_ctx, external_amount as u64)?;
    }


    Ok(())
}
