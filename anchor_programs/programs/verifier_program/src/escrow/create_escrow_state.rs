use crate::escrow::escrow_state::FeeEscrowState;
use crate::groth16_verifier::VerifierState;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock::Clock, msg, sysvar::Sysvar};
use anchor_spl::token::{Token, Transfer};
use crate::errors::ErrorCode;
use solana_program::program_pack::Pack;
use merkle_tree_program::ID;

#[derive(Accounts)]
#[instruction(
    tx_integrity_hash: [u8;32]
)]
pub struct CreateEscrowState<'info> {
    #[account(init,seeds = [tx_integrity_hash.as_ref(), b"escrow"], bump,  payer=signing_address, space=256)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(init, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024)]
    /// CHECK: is ininitialized at this point the
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// User account which partially signed the tx to create the escrow such that the relayer
    /// can executed all transactions.
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// The corresponding token escrow account is passed-in as remaining account and checked before transfer.
    pub token_program: Program<'info, Token>,
    /// CHECK: is ininitialized at this point the
    #[account(mut)]
    pub token_authority: AccountInfo<'info>
}

pub fn process_create_escrow<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateEscrowState<'info>>,
    tx_integrity_hash: [u8; 32],
    tx_fee: u64,
    relayer_fee: [u8; 8],
    amount: u64,
    merkle_tree_index: u64
) -> Result<()> {
    msg!("starting initializing escrow account");
    // init verifier state with signer
    let verifier_state_data = &mut ctx.accounts.verifier_state.load_init()?;
    verifier_state_data.signing_address = ctx.accounts.signing_address.key().clone();
    verifier_state_data.tx_integrity_hash = tx_integrity_hash.clone();
    let ext_amount: i64 = amount.try_into().unwrap();
    verifier_state_data.ext_amount = ext_amount.to_le_bytes();
    verifier_state_data.merkle_tree_index = <u8 as TryFrom<u64>>::try_from(merkle_tree_index).unwrap();
    verifier_state_data.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();

    // init escrow account
    let fee_escrow_state = &mut ctx.accounts.fee_escrow_state;

    fee_escrow_state.verifier_state_pubkey = ctx.accounts.verifier_state.key();
    fee_escrow_state.relayer_pubkey = ctx.accounts.signing_address.key();
    fee_escrow_state.user_pubkey = ctx.accounts.user.key();
    fee_escrow_state.tx_fee = tx_fee;
    fee_escrow_state.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();
    fee_escrow_state.creation_slot = <Clock as Sysvar>::get()?.slot;

    // Always transfer fees in sol to escrow account.
    let mut escrow_amount = fee_escrow_state.tx_fee.checked_add(fee_escrow_state.relayer_fee).unwrap();

    if verifier_state_data.merkle_tree_index == 0 {
        // If amount is in sol as well add it to escrow amount.
        escrow_amount = escrow_amount.checked_add(amount).unwrap();
    } else if ctx.remaining_accounts.len() == 2 {
        let accounts = &mut ctx.remaining_accounts.iter();
        let from = next_account_info(accounts)?;
        let to = next_account_info(accounts)?;

        // Check that remaining_accounts are token accounts.
        spl_token::state::Account::unpack(&from.data.borrow())?;
        spl_token::state::Account::unpack(&to.data.borrow())?;

        // Save user_token_pda for check in close escrow.
        fee_escrow_state.user_token_pda = from.key().clone();

        let address= solana_program::pubkey::Pubkey::create_with_seed(
            &ctx.accounts.signing_address.key(),
            "escrow",
            &ctx.accounts.token_program.key()).unwrap();
        // Check that the recipient is the correct token account and owned by the program.
        if to.key() != address {
            return err!(ErrorCode::IncorrectTokenEscrowAcc);
        }

        let seed = ID.to_bytes();
        let (_, bump) = solana_program::pubkey::Pubkey::find_program_address(
            &[seed.as_ref()],
            ctx.program_id,
        );
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = Transfer {
            from:       from.to_account_info().clone(),
            to:         to.to_account_info().clone(),
            authority:  ctx.accounts.token_authority.to_account_info().clone()
        };
        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info().clone(), accounts, seeds);
        anchor_spl::token::transfer(cpi_ctx, amount)?;
    }

    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.fee_escrow_state.to_account_info(),
        },
    );
    anchor_lang::system_program::transfer(cpi_ctx, escrow_amount)?;
    Ok(())
}
