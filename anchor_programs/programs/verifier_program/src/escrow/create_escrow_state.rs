use anchor_lang::prelude::*;
use crate::groth16_verifier::VerifierState;
use crate::escrow::escrow_state::FeeEscrowState;
use anchor_lang::solana_program::{
    clock::Clock,
    msg,
    sysvar::Sysvar,
};

#[derive(Accounts)]
#[instruction(
    tx_integrity_hash: [u8;32]
)]
pub struct CreateEscrowState<'info> {
    #[account(init,seeds = [tx_integrity_hash.as_ref(), b"fee_escrow"], bump,  payer=signing_address, space= 128 as usize)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(init, seeds = [tx_integrity_hash.as_ref(), b"storage"], bump,  payer=signing_address, space= 5 * 1024 as usize)]
    /// CHECK: is ininitialized at this point the
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// User account which partially signed the tx to create the escrow such that the relayer
    /// can executed all transactions.
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>
}

pub fn process_create_escrow_state(
        ctx: Context<CreateEscrowState>,
        _tx_integrity_hash: [u8; 32],
        tx_fee: u64,
        relayer_fee: [u8;8],
        amount: u64
    ) -> Result<()> {
    msg!("starting initializing escrow account");

    // init escrow account
    let fee_escrow_state = &mut ctx.accounts.fee_escrow_state;

    fee_escrow_state.verifier_state_pubkey = ctx.accounts.verifier_state.key();
    fee_escrow_state.relayer_pubkey = ctx.accounts.signing_address.key();
    fee_escrow_state.user_pubkey = ctx.accounts.user.key();
    fee_escrow_state.tx_fee = tx_fee;//u64::from_le_bytes(tx_fee.try_into().unwrap()).clone();// fees for tx (tx_fee = number_of_tx * 0.000005)
    fee_escrow_state.relayer_fee = u64::from_le_bytes(relayer_fee.try_into().unwrap()).clone();// for relayer
    fee_escrow_state.creation_slot = <Clock as Sysvar>::get()?.slot;

    let cpi_ctx1 = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer{
         from: ctx.accounts.user.to_account_info(),
         to: ctx.accounts.fee_escrow_state.to_account_info()
     });
    anchor_lang::system_program::transfer(cpi_ctx1, amount)?;
    msg!(" initialized escrow account");
    Ok(())
}
