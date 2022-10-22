use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_verifier_sdk::{
    accounts::Accounts,
    errors::VerifierSdkError,
    light_transaction::{LightTransaction, TxConfig},
};
use solana_program::log::sol_log_compute_units;

use crate::LightInstruction;
struct TransactionConfig;
impl TxConfig for TransactionConfig {
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
    const UTXO_SIZE: usize = 256;
}

pub fn process_shielded_transfer_2_inputs<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
    proof: Vec<u8>,
    merkle_root: Vec<u8>,
    public_amount: Vec<u8>,
    ext_data_hash: Vec<u8>,
    nullifiers: Vec<Vec<u8>>,
    leaves: Vec<Vec<Vec<u8>>>,
    fee_amount: Vec<u8>,
    mint_pubkey: Vec<u8>,
    encrypted_utxos: Vec<u8>,
    merkle_tree_index: u64,
    relayer_fee: u64,
    checked_public_inputs: Vec<Vec<u8>>
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

    let mut transaction = LightTransaction::<TransactionConfig>::new(
        proof,
        merkle_root,
        public_amount,
        ext_data_hash,
        fee_amount,
        mint_pubkey,
        checked_public_inputs,
        nullifiers,
        leaves,
        encrypted_utxos,
        relayer_fee,
        merkle_tree_index.try_into().unwrap(),
        Some(&accounts),
        &VERIFYINGKEY,
    );

    transaction.transact()
}
