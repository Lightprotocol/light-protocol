use anchor_lang::prelude::*;
use solana_program::log::sol_log_compute_units;
use crate::verification_key::VERIFYINGKEY;
use light_verifier_sdk::{
    light_transaction::{
        TxConfig,
        LightTransaction
    },
    accounts::Accounts,
    state::VerifierStateTenNF
};

use crate::{LightInstructionFirst, LightInstructionSecond};
#[derive(Clone)]
pub struct LightTx;
impl TxConfig for LightTx {
    /// Number of nullifiers to be inserted with the transaction.
	const NR_NULLIFIERS: usize = 2;
	/// Number of output utxos.
	const NR_LEAVES: usize = 2;
	/// Number of checked public inputs.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
    /// ProgramId in bytes.
    const ID: [u8;32]  = [
       34, 112,  33,  68, 178, 147, 230, 193,
      113,  82, 213, 107, 154, 193, 174, 159,
      246, 190,  23, 138, 211,  16, 120, 183,
        7,  91,  10, 173,  20, 245,  75, 167
    ];
}

pub fn process_shielded_transfer_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info,LightInstructionFirst<'info>>,
    proof: [u8; 256],
    merkle_root: [u8; 32],
    public_amount: [u8; 32],
    ext_data_hash: [u8; 32],
    nullifiers: [[u8; 32]; 10],
    leaves: [[u8; 32]; 2],
    fee_amount: [u8; 32],
    mint_pubkey: [u8;32],
    encrypted_utxos0: Vec<u8>,
    encrypted_utxos1: Vec<u8>,
    root_index: u64,
    relayer_fee: u64,
) -> Result<()> {

    let mut tx = LightTransaction::<LightTx>::new(
        &proof,
        &merkle_root,
        &public_amount,
        &ext_data_hash,
        &fee_amount,
        &mint_pubkey,
        Vec::<[u8; 32]>::new(), // checked_public_inputs
        nullifiers.to_vec(),
        vec![leaves],
        [encrypted_utxos0, encrypted_utxos1].concat(),
        relayer_fee,
        root_index.try_into().unwrap(),
        None,
        &VERIFYINGKEY
    );

    tx.verify()?;
    sol_log_compute_units();
    msg!("packing verifier state");
    ctx.accounts.verifier_state.set_inner(tx.into());
    sol_log_compute_units();
    msg!("packed verifier state");
    Ok(())
}

pub fn process_shielded_transfer_second<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info,LightInstructionSecond<'info>>,
) -> Result<()> {

    // trait with the nunber of inputs and commitments
    // Put nullifier accounts in remaining accounts
    // Put commitment accounts in the remaining accounts
    // make the instruction flexible enough such that I can easily call it in a second tx
    // actually with that I can easily implement it in 2 tx in the first place
    // Shielded state update should be atomic thus this account struct should only be used completely
    // or not at all.
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.rent,
        &ctx.accounts.merkle_tree,
        &ctx.accounts.pre_inserted_leaves_index,
        ctx.accounts.authority.to_account_info(),
        &ctx.accounts.token_program,
        ctx.accounts.sender.to_account_info(),
        ctx.accounts.recipient.to_account_info(),
        ctx.accounts.sender_fee.to_account_info(),
        ctx.accounts.recipient_fee.to_account_info(),
        ctx.accounts.relayer_recipient.to_account_info(),
        ctx.accounts.escrow.to_account_info(),
        ctx.accounts.token_authority.to_account_info(),
        &ctx.accounts.registered_verifier_pda,
        ctx.remaining_accounts
    )?;

    let mut tx: LightTransaction::<LightTx> = ctx.accounts.verifier_state.into_light_transaction(Some(&accounts), &VERIFYINGKEY);

    tx.check_root()?;
    sol_log_compute_units();
    msg!("leaves");
    tx.insert_leaves()?;
    sol_log_compute_units();
    msg!("nullifiers");
    tx.insert_nullifiers()?;
    sol_log_compute_units();
    tx.transfer_user_funds()?;
    tx.transfer_fee()?;
    tx.check_completion()

}
