use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Config, Transaction},
};

use crate::LightInstruction;
use anchor_lang::solana_program::{keccak::hash, msg};
use light_verifier_sdk::state::VerifierState10Ins;

#[derive(Clone)]
pub struct TransactionConfig;
impl Config for TransactionConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 4;
    /// Number of output utxos.
    const NR_LEAVES: usize = 4;
    /// Number of checked public inputs, Kyc, Invoking Verifier, Apphash.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 2;
    /// ProgramId in bytes.
    const ID: [u8; 32] = [
        252, 178, 75, 149, 78, 219, 142, 17, 53, 237, 47, 4, 42, 105, 173, 204, 248, 16, 209, 38,
        219, 222, 123, 242, 5, 68, 240, 131, 3, 211, 184, 81,
    ];
    // /// Verifier stores 512 bytes for example 4 utxos of 128 bytes each.
    // const UTXO_SIZE: usize = 512;
}

pub fn process_shielded_transfer<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstruction<'info>>,
    proof: Vec<u8>,
    connecting_hash: Vec<u8>,
) -> Result<()> {
    let verifier_state = VerifierState10Ins::<TransactionConfig>::deserialize(
        &mut &*ctx.accounts.verifier_state.to_account_info().data.take(),
    )?;

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
    // msg!("here1 {:?}", verifier_state);
    // for i in verifier_state.nullifiers.iter() {
    //     msg!("nullifier: {:?}", i);
    // }
    // for i in verifier_state.leaves.iter() {
    //     msg!("leaves: {:?}", i);
    // }
    let checked_inputs = vec![
        [
            vec![0u8],
            hash(&ctx.accounts.invoking_verifier.owner.to_bytes()).try_to_vec()?[1..].to_vec(),
        ]
        .concat(),
        connecting_hash,
    ];
    let mut tx = Transaction::<TransactionConfig>::new(
        proof,
        verifier_state.public_amount.to_vec(),
        verifier_state.fee_amount.to_vec(),
        checked_inputs,
        verifier_state.nullifiers.to_vec(),
        vec![
            vec![
                verifier_state.leaves[0].to_vec(),
                verifier_state.leaves[1].to_vec(),
            ],
            vec![
                verifier_state.leaves[2].to_vec(),
                verifier_state.leaves[3].to_vec(),
            ],
        ],
        verifier_state.encrypted_utxos.to_vec(),
        verifier_state.relayer_fee,
        verifier_state.merkle_root_index.try_into().unwrap(),
        vec![0u8; 32], //verifier_state.pool_type,
        Some(&accounts),
        &VERIFYINGKEY,
    );

    tx.transact()
}
