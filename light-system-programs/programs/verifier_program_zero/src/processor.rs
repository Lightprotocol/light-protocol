use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
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
    /// Number of checked public inputs.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
    /// ProgramId in bytes.
    const ID: [u8; 32] = [
        252, 178, 75, 149, 78, 219, 142, 17, 53, 237, 47, 4, 42, 105, 173, 204, 248, 16, 209, 38,
        219, 222, 123, 242, 5, 68, 240, 131, 3, 211, 184, 81,
    ];
}

pub fn process_shielded_transfer_2_in_2_out<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
    proof: Vec<u8>,
    public_amount: Vec<u8>,
    nullifiers: Vec<Vec<u8>>,
    leaves: Vec<Vec<Vec<u8>>>,
    fee_amount: Vec<u8>,
    encrypted_utxos: Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
    checked_public_inputs: Vec<Vec<u8>>,
    pool_type: Vec<u8>,
) -> Result<()> {
    msg!("sneder fee: {:?}", ctx.accounts.sender_fee);
    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.merkle_tree,
        ctx.accounts.authority.to_account_info(),
        Some(&ctx.accounts.token_program),
        Some(ctx.accounts.sender.to_account_info()),
        Some(ctx.accounts.recipient.to_account_info()),
        Some(ctx.accounts.sender_fee.to_account_info()),
        Some(ctx.accounts.recipient_fee.to_account_info()),
        Some(ctx.accounts.relayer_recipient.to_account_info()),
        Some(ctx.accounts.token_authority.to_account_info()),
        &ctx.accounts.registered_verifier_pda,
        ctx.remaining_accounts,
    )?;

    let mut transaction = Transaction::<TransactionConfig>::new(
        proof,
        public_amount,
        fee_amount,
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
