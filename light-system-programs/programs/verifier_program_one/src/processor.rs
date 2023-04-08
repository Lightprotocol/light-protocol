use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Config, Transaction},
};

use crate::{LightInstructionFirst, LightInstructionSecond};
#[derive(Clone)]
pub struct TransactionConfig;
impl Config for TransactionConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 10;
    /// Number of output utxos.
    const NR_LEAVES: usize = 2;
    /// Number of checked public inputs.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
    /// ProgramId.
    const ID: Pubkey = pubkey!("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");
}

pub fn process_transfer_10_ins_2_outs_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
    proof_a: &'a [u8; 64],
    proof_b: &'a [u8; 128],
    proof_c: &'a [u8; 64],
    public_amount_spl: &'a [u8; 32],
    nullifiers: &'a [[u8; 32]; 10],
    leaves: &'a [[[u8; 32]; 2]; 1],
    public_amount_sol: &'a [u8; 32],
    encrypted_utxos: &'a Vec<u8>,
    root_index: &'a u64,
    relayer_fee: &'a u64,
) -> Result<()> {
    let checked_public_inputs = Vec::<Vec<u8>>::new();
    let pool_type = [0u8; 32];
    let tx = Transaction::<1, 10, TransactionConfig>::new(
        proof_a,
        proof_b,
        proof_c,
        public_amount_spl,
        public_amount_sol,
        &checked_public_inputs, // checked_public_inputs
        nullifiers,
        leaves,
        &encrypted_utxos,
        *relayer_fee,
        (*root_index).try_into().unwrap(),
        &pool_type, //pool_type
        None,
        &VERIFYINGKEY,
    );
    ctx.accounts.verifier_state.set_inner(tx.into());
    ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;
    Ok(())
}

pub fn process_transfer_10_ins_2_outs_second<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
    proof_a: &'a [u8; 64],
    proof_b: &'a [u8; 128],
    proof_c: &'a [u8; 64],
    pool_type: [u8; 32],
) -> Result<()> {
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.transaction_merkle_tree,
        ctx.accounts.authority.to_account_info(),
        Some(&ctx.accounts.token_program),
        Some(ctx.accounts.sender_spl.to_account_info()),
        Some(ctx.accounts.recipient_spl.to_account_info()),
        Some(ctx.accounts.sender_sol.to_account_info()),
        Some(ctx.accounts.recipient_sol.to_account_info()),
        Some(ctx.accounts.relayer_recipient_sol.to_account_info()),
        Some(ctx.accounts.token_authority.to_account_info()),
        &ctx.accounts.registered_verifier_pda,
        ctx.remaining_accounts,
    )?;
    let checked_public_inputs = Vec::<Vec<u8>>::new();

    let leaves = [[
        ctx.accounts.verifier_state.leaves[0],
        ctx.accounts.verifier_state.leaves[1],
    ]; 1];
    let nullifier: [[u8; 32]; 10] = ctx
        .accounts
        .verifier_state
        .nullifiers
        .to_vec()
        .try_into()
        .unwrap();

    let mut tx = Transaction::<1, 10, TransactionConfig>::new(
        proof_a,
        proof_b,
        proof_c,
        &ctx.accounts.verifier_state.public_amount_spl,
        &ctx.accounts.verifier_state.public_amount_sol,
        &checked_public_inputs, // checked_public_inputs
        &nullifier,
        &leaves,
        &ctx.accounts.verifier_state.encrypted_utxos,
        ctx.accounts.verifier_state.relayer_fee,
        ctx.accounts
            .verifier_state
            .merkle_root_index
            .try_into()
            .unwrap(),
        &pool_type,
        Some(&accounts),
        &VERIFYINGKEY,
    );
    tx.transact()
}
