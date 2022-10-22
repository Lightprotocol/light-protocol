use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_verifier_sdk::{
    accounts::Accounts,
    errors::VerifierSdkError,
    light_transaction::{LightTransaction, TxConfig},
    state::VerifierStateTenNF,
};
use solana_program::log::sol_log_compute_units;

use crate::{LightInstructionFirst, LightInstructionSecond};
#[derive(Clone)]
pub struct LightTx;
impl TxConfig for LightTx {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 10;
    /// Number of output utxos.
    const NR_LEAVES: usize = 2;
    /// Number of checked public inputs.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
    /// ProgramId in bytes.
    const ID: [u8; 32] = [
        34, 112, 33, 68, 178, 147, 230, 193, 113, 82, 213, 107, 154, 193, 174, 159, 246, 190, 23,
        138, 211, 16, 120, 183, 7, 91, 10, 173, 20, 245, 75, 167,
    ];
    const UTXO_SIZE: usize = 256;
}

pub fn process_shielded_transfer_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
    proof: Vec<u8>,
    merkle_root: Vec<u8>,
    public_amount: Vec<u8>,
    ext_data_hash: Vec<u8>,
    nullifiers: Vec<Vec<u8>>,
    leaves: Vec<Vec<Vec<u8>>>,
    fee_amount: Vec<u8>,
    mint_pubkey: Vec<u8>,
    encrypted_utxos: Vec<u8>,
    root_index: &u64,
    relayer_fee: &u64,
) -> Result<()> {
    let mut tx = LightTransaction::<LightTx>::new(
        proof,
        merkle_root,
        public_amount,
        ext_data_hash,
        fee_amount,
        mint_pubkey,
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        nullifiers,
        leaves,
        encrypted_utxos,
        *relayer_fee,
        (*root_index).try_into().unwrap(),
        None,
        &VERIFYINGKEY,
    );

    sol_log_compute_units();
    msg!("packing verifier state");
    ctx.accounts.verifier_state.set_inner(tx.into());
    msg!(
        "ctx.accounts.verifier_state {:?}",
        ctx.accounts.verifier_state.leaves
    );
    sol_log_compute_units();
    msg!("packed verifier state");
    Ok(())
}

pub fn process_shielded_transfer_second<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
    proof: Vec<u8>,
) -> Result<()> {
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.rent,
        &ctx.accounts.merkle_tree,
        &ctx.accounts.pre_inserted_leaves_index,
        ctx.accounts.authority.to_account_info(),
        Some(&ctx.accounts.token_program),
        Some(ctx.accounts.sender.to_account_info()),
        Some(ctx.accounts.recipient.to_account_info()),
        Some(ctx.accounts.sender_fee.to_account_info()),
        Some(ctx.accounts.recipient_fee.to_account_info()),
        Some(ctx.accounts.relayer_recipient.to_account_info()),
        Some(ctx.accounts.escrow.to_account_info()),
        Some(ctx.accounts.token_authority.to_account_info()),
        &ctx.accounts.registered_verifier_pda,
        ctx.remaining_accounts,
    )?;

    let mut tx = LightTransaction::<LightTx>::new(
        proof,
        ctx.accounts.verifier_state.merkle_root.to_vec(),
        ctx.accounts.verifier_state.public_amount.to_vec(),
        ctx.accounts.verifier_state.tx_integrity_hash.to_vec(),
        ctx.accounts.verifier_state.fee_amount.to_vec(),
        ctx.accounts.verifier_state.mint_pubkey.to_vec(),
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        ctx.accounts.verifier_state.nullifiers.to_vec(), //vec![nullifier0.to_vec(), nullifier1.to_vec()],
        vec![ctx.accounts.verifier_state.leaves.to_vec()], //vec![vec![leaf_left.to_vec(), leaf_right.to_vec()]],
        ctx.accounts.verifier_state.encrypted_utxos.to_vec(),
        ctx.accounts.verifier_state.relayer_fee,
        ctx.accounts
            .verifier_state
            .merkle_root_index
            .try_into()
            .unwrap(),
        Some(&accounts),
        &VERIFYINGKEY,
    );
    tx.verify()?;
    tx.check_tx_integrity_hash()?;
    tx.check_root()?;
    tx.insert_leaves()?;
    tx.insert_nullifiers()?;
    tx.transfer_user_funds()?;
    tx.transfer_fee()?;
    tx.check_completion()?;
    Ok(())
}
