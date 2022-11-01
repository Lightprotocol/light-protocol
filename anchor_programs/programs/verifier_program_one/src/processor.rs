use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
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
    /// ProgramId in bytes.
    const ID: [u8; 32] = [
        34, 112, 33, 68, 178, 147, 230, 193, 113, 82, 213, 107, 154, 193, 174, 159, 246, 190, 23,
        138, 211, 16, 120, 183, 7, 91, 10, 173, 20, 245, 75, 167,
    ];
    const UTXO_SIZE: usize = 256;
}

pub fn process_transfer_10_ins_2_outs_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
    proof: Vec<u8>,
    public_amount: Vec<u8>,
    nullifiers: Vec<Vec<u8>>,
    leaves: Vec<Vec<Vec<u8>>>,
    fee_amount: Vec<u8>,
    encrypted_utxos: Vec<u8>,
    root_index: &u64,
    relayer_fee: &u64,
) -> Result<()> {
    let tx = Transaction::<TransactionConfig>::new(
        proof,
        public_amount,
        fee_amount,
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        nullifiers,
        leaves,
        encrypted_utxos,
        *relayer_fee,
        (*root_index).try_into().unwrap(),
        vec![0u8; 32], //pool_type
        None,
        &VERIFYINGKEY,
    );
    ctx.accounts.verifier_state.set_inner(tx.into());
    ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;
    Ok(())
}

pub fn process_transfer_10_ins_2_outs_second<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
    proof: Vec<u8>,
    pool_type: Vec<u8>,
) -> Result<()> {
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
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

    let mut tx = Transaction::<TransactionConfig>::new(
        proof,
        ctx.accounts.verifier_state.public_amount.to_vec(),
        ctx.accounts.verifier_state.fee_amount.to_vec(),
        Vec::<Vec<u8>>::new(), // checked_public_inputs
        ctx.accounts.verifier_state.nullifiers.to_vec(),
        vec![ctx.accounts.verifier_state.leaves.to_vec()],
        ctx.accounts.verifier_state.encrypted_utxos.to_vec(),
        ctx.accounts.verifier_state.relayer_fee,
        ctx.accounts
            .verifier_state
            .merkle_root_index
            .try_into()
            .unwrap(),
        pool_type,
        Some(&accounts),
        &VERIFYINGKEY,
    );
    tx.transact()
}
