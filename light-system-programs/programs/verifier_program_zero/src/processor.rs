use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Config, Transaction},
};

use crate::LightInstruction;
struct TransactionConfig;
impl Config for TransactionConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 2;
    /// Number of output utxos.
    const NR_LEAVES: usize = 2;
    /// ProgramId in bytes.
    const ID: Pubkey = pubkey!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");
}

pub fn process_shielded_transfer_2_in_2_out<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
    proof_a: &'a [u8; 64],
    proof_b: &'a [u8; 128],
    proof_c: &'a [u8; 64],
    public_amount_spl: &'a [u8; 32],
    nullifiers: &'a [[u8; 32]; 2],
    leaves: &'a [[[u8; 32]; 2]; 1],
    public_amount_sol: &'a [u8; 32],
    encrypted_utxos: &'a Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
    checked_public_inputs: &'a Vec<Vec<u8>>,
    pool_type: &'a [u8; 32],
) -> Result<()> {
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        None,
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
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.remaining_accounts,
    )?;

    let mut transaction = Transaction::<1, 2, TransactionConfig>::new(
        None,
        None,
        proof_a,
        proof_b,
        proof_c,
        public_amount_spl,
        public_amount_sol,
        checked_public_inputs,
        nullifiers,
        leaves,
        encrypted_utxos,
        relayer_fee,
        merkle_tree_index.try_into().unwrap(),
        pool_type,
        Some(&accounts),
        &VERIFYINGKEY,
    );

    transaction.transact()
}
