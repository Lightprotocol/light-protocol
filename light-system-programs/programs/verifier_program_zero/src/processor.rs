use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Amounts, Config, Proof, Transaction, TransactionInput},
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

#[allow(clippy::too_many_arguments)]
pub fn process_shielded_transfer_2_in_2_out<'a, 'info, const NR_CHECKED_INPUTS: usize>(
    ctx: Context<'a, '_, '_, 'info, LightInstruction<'info>>,
    proof: &'a Proof,
    public_amount: &'a Amounts,
    nullifiers: &'a [[u8; 32]; 2],
    leaves: &'a [[[u8; 32]; 2]; 1],
    encrypted_utxos: &'a Vec<u8>,
    merkle_root_index: u64,
    relayer_fee: u64,
    checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
    pool_type: &'a [u8; 32],
) -> Result<()> {
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.event_merkle_tree,
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

    let input = TransactionInput {
        message: None,
        proof,
        public_amount,
        nullifiers,
        leaves,
        encrypted_utxos,
        merkle_root_index: merkle_root_index as usize,
        relayer_fee,
        checked_public_inputs,
        pool_type,
        accounts: Some(&accounts),
        verifyingkey: &VERIFYINGKEY,
    };
    let mut transaction = Transaction::<NR_CHECKED_INPUTS, 1, 2, 9, TransactionConfig>::new(input);

    transaction.transact()
}
