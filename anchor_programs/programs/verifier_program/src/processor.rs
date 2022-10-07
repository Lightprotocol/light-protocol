use anchor_lang::prelude::*;
use solana_program::log::sol_log_compute_units;
use crate::verification_key::VERIFYINGKEY;
use light_verifier_sdk::{
    light_transaction::{
        TxConfig,
        LightTransaction
    },
    accounts::Accounts
};

use crate::LightInstruction;
struct LightTx;
impl TxConfig for LightTx {
    /// Number of nullifiers to be inserted with the transaction.
	const NR_NULLIFIERS: usize = 2;
	/// Number of output utxos.
	const NR_LEAVES: usize = 2;
	/// Number of checked public inputs.
    const N_CHECKED_PUBLIC_INPUTS: usize = 0;
}

// split into two tx
// tx checks which data it has and computes accordingly
// tx checks if other compute was already completed
// if yes insert leaves etc

pub fn process_shielded_transfer_2_inputs<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info,LightInstruction<'info>>,
    proof: [u8; 256],
    merkle_root: [u8; 32],
    public_amount: [u8; 32],
    ext_data_hash: [u8; 32],
    nullifier0: [u8; 32],
    nullifier1: [u8; 32],
    leaf_right: [u8; 32],
    leaf_left: [u8; 32],
    ext_amount: i64,
    fee_amount: [u8; 32],
    mint_pubkey: [u8;32],
    encrypted_utxos: Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
) -> Result<()> {

    // trait with the nunber of inputs and commitments
    // Put nullifier accounts in remaining accounts
    // Put commitment accounts in the remaining accounts
    // make the instruction flexible enough such that I can easily call it in a second tx
    // actually with that I can easily implement it in 2 tx in the first place
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

    let mut tx = LightTransaction::<LightTx>::new(
        &proof,
        &merkle_root,
        &public_amount,
        &ext_data_hash,
        &fee_amount,
        &mint_pubkey,
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        vec![nullifier0.to_vec(), nullifier1.to_vec()],
        vec![(leaf_left.to_vec(), leaf_right.to_vec())],
        encrypted_utxos,
        ext_amount,
        relayer_fee,
        &accounts,
        &VERIFYINGKEY
    );
    tx.verify()?;
    tx.check_tx_integrity_hash()?;
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
    tx.check_completion()?;
    Ok(())
}
