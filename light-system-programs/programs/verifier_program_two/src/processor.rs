use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Config, Transaction},
};

use crate::LightInstruction;
use anchor_lang::solana_program::keccak::hash;
use light_verifier_sdk::state::VerifierState10Ins;

#[derive(Clone)]
pub struct TransactionConfig;
impl Config for TransactionConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 4;
    /// Number of output utxos.
    const NR_LEAVES: usize = 4;
    /// Number of checked public inputs, Invoking Verifier, connecting hash.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 2;
    /// ProgramId.
    const ID: Pubkey = pubkey!("GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8");
}

pub fn process_shielded_transfer<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
    proof_a: &'a [u8; 64],
    proof_b: &'a [u8; 128],
    proof_c: &'a [u8; 64],
    connecting_hash: &[u8; 32],
) -> Result<()> {
    let verifier_state = VerifierState10Ins::<TransactionConfig>::deserialize(
        &mut &*ctx.accounts.verifier_state.to_account_info().data.take(),
    )?;

    let accounts = Accounts::new(
        ctx.program_id,
        ctx.accounts.signing_address.to_account_info(),
        &ctx.accounts.system_program,
        &ctx.accounts.program_merkle_tree,
        &ctx.accounts.transaction_merkle_tree,
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

    if *ctx.accounts.verifier_state.owner == ctx.accounts.system_program.key() {
        return err!(crate::ErrorCode::InvalidVerifier);
    };

    let checked_inputs = vec![
        [
            vec![0u8],
            hash(&ctx.accounts.verifier_state.owner.to_bytes()).try_to_vec()?[1..].to_vec(),
        ]
        .concat(),
        connecting_hash.to_vec(),
    ];
    let leaves = [
        [verifier_state.leaves[0], verifier_state.leaves[1]],
        [verifier_state.leaves[2], verifier_state.leaves[3]],
    ];

    let nullifiers: [[u8; 32]; 4] = verifier_state.nullifiers.to_vec().try_into().unwrap();
    let pool_type = [0u8; 32];
    let mut tx = Transaction::<2, 4, TransactionConfig>::new(
        proof_a,
        proof_b,
        proof_c,
        &verifier_state.public_amount,
        &verifier_state.fee_amount,
        &checked_inputs,
        &nullifiers,
        &leaves,
        &verifier_state.encrypted_utxos,
        verifier_state.relayer_fee,
        verifier_state.merkle_root_index.try_into().unwrap(),
        &pool_type, //verifier_state.pool_type,
        Some(&accounts),
        &VERIFYINGKEY,
    );

    tx.transact()
}
