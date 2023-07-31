use crate::verifying_key::VERIFYINGKEY;
use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_verifier_sdk::{
    accounts::Accounts,
    light_transaction::{Amounts, Config, Proof, Transaction, TransactionInput},
};

use crate::{LightInstructionFirst, LightInstructionSecond};
#[derive(Clone)]
pub struct TransactionConfig;
impl Config for TransactionConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 10;
    /// Number of output utxos.
    const NR_LEAVES: usize = 2;
    /// ProgramId.
    const ID: Pubkey = pubkey!("J85SuNBBsba7FQS66BiBCQjiQrQTif7v249zL2ffmRZc");
}

#[allow(clippy::too_many_arguments)]
pub fn process_transfer_10_ins_2_outs_first<'a, 'info>(
<<<<<<< HEAD
    ctx: Context<'a, '_, '_, 'info, LightInstructionFirst<'info>>,
=======
    ctx: Context<'a, '_, '_, 'info, LightInstructionFirst<'info, 0>>,
>>>>>>> main
    proof: &'a Proof,
    public_amount: &'a Amounts,
    nullifiers: &'a [[u8; 32]; 10],
    leaves: &'a [[[u8; 32]; 2]; 1],
    encrypted_utxos: &'a Vec<u8>,
    merkle_root_index: u64,
    relayer_fee: u64,
) -> Result<()> {
    let pool_type = [0u8; 32];
<<<<<<< HEAD
=======

>>>>>>> main
    let input = TransactionInput {
        message: None,
        proof,
        public_amount,
        nullifiers,
        leaves,
        encrypted_utxos,
        relayer_fee,
        merkle_root_index: merkle_root_index as usize,
        pool_type: &pool_type,
<<<<<<< HEAD
        checked_public_inputs: &checked_public_inputs,
        accounts: None,
        verifyingkey: &VERIFYINGKEY,
    };
    let tx = Transaction::<1, 10, TransactionConfig>::new(input);
=======
        checked_public_inputs: &[],
        accounts: None,
        verifyingkey: &VERIFYINGKEY,
    };
    let tx = Transaction::<0, 1, 10, 17, TransactionConfig>::new(input);
>>>>>>> main
    ctx.accounts.verifier_state.set_inner(tx.into());
    ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;
    Ok(())
}

pub fn process_transfer_10_ins_2_outs_second<'a, 'info>(
<<<<<<< HEAD
    ctx: Context<'a, '_, '_, 'info, LightInstructionSecond<'info>>,
=======
    ctx: Context<'a, '_, '_, 'info, LightInstructionSecond<'info, 0>>,
>>>>>>> main
    proof: &'a Proof,
    pool_type: [u8; 32],
) -> Result<()> {
    let public_amount = Amounts {
        sol: ctx.accounts.verifier_state.public_amount_sol,
        spl: ctx.accounts.verifier_state.public_amount_spl,
    };
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

    let input = TransactionInput {
        message: None,
        proof,
        public_amount: &public_amount,
        nullifiers: &nullifier,
        leaves: &leaves,
        encrypted_utxos: &ctx.accounts.verifier_state.encrypted_utxos,
        relayer_fee: ctx.accounts.verifier_state.relayer_fee,
        merkle_root_index: ctx
            .accounts
            .verifier_state
            .merkle_root_index
            .try_into()
            .unwrap(),
        pool_type: &pool_type,
<<<<<<< HEAD
        checked_public_inputs: &checked_public_inputs,
        accounts: Some(&accounts),
        verifyingkey: &VERIFYINGKEY,
    };
    let mut tx = Transaction::<1, 10, TransactionConfig>::new(input);
=======
        checked_public_inputs: &[],
        accounts: Some(&accounts),
        verifyingkey: &VERIFYINGKEY,
    };
    let mut tx = Transaction::<0, 1, 10, 17, TransactionConfig>::new(input);
>>>>>>> main
    tx.transact()
}
