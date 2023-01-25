use crate::verifying_key::VERIFYINGKEY;
use crate::LightInstructionFirst;
use crate::LightInstructionSecond;
use anchor_lang::prelude::*;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::{
    cpi_instructions::get_seeds,
    light_app_transaction::AppTransaction,
    light_transaction::{Config, Transaction},
};

use anchor_lang::solana_program::sysvar;

#[derive(Clone)]
pub struct TransactionsConfig;
impl Config for TransactionsConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 4;
    /// Number of output utxos.
    const NR_LEAVES: usize = 4;
    /// Number of checked public inputs, Kyc, Invoking Verifier, Apphash.
    const NR_CHECKED_PUBLIC_INPUTS: usize = 3;
    /// ProgramId in bytes.
    const ID: [u8; 32] = [
        218, 7, 92, 178, 255, 94, 198, 129, 118, 19, 222, 83, 11, 105, 42, 135, 53, 71, 119, 105,
        218, 71, 67, 12, 189, 129, 84, 51, 92, 74, 131, 39,
    ];
}

pub fn process_transfer_4_ins_4_outs_4_checked_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
    proof: Vec<u8>,
    public_amount: Vec<u8>,
    nullifiers: Vec<Vec<u8>>,
    leaves: Vec<Vec<Vec<u8>>>,
    fee_amount: Vec<u8>,
    checked_public_inputs: Vec<Vec<u8>>,
    encrypted_utxos: Vec<u8>,
    pool_type: Vec<u8>,
    root_index: &u64,
    relayer_fee: &u64,
) -> Result<()> {
    let tx = Transaction::<TransactionsConfig>::new(
        proof,
        public_amount,
        fee_amount,
        checked_public_inputs,
        nullifiers,
        leaves,
        encrypted_utxos,
        *relayer_fee,
        (*root_index).try_into().unwrap(),
        pool_type, //pool_type
        None,
        &VERIFYINGKEY,
    );
    ctx.accounts.verifier_state.set_inner(tx.into());
    ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;
    Ok(())
}

pub fn process_transfer_4_ins_4_outs_4_checked_second<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionSecond<'info>>,
    proof_app: Vec<u8>,
    proof_verifier: Vec<u8>,
) -> Result<()> {
    // verify app proof
    let mut app_verifier = AppTransaction::<TransactionsConfig>::new(
        proof_app,
        ctx.accounts.verifier_state.checked_public_inputs.clone(),
        &VERIFYINGKEY,
    );

    app_verifier.verify()?;
    let seed = [
        ctx.accounts.signing_address.key().to_bytes().as_ref(),
        VERIFIER_STATE_SEED.as_ref(),
    ];
    let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[
            ctx.accounts.signing_address.key().to_bytes().as_ref(),
            VERIFIER_STATE_SEED.as_ref(),
        ],
        ctx.program_id,
    );

    let bump = &[bump];
    let accounts = verifier_program_two::cpi::accounts::LightInstruction {
        verifier_state: ctx.accounts.verifier_state.to_account_info().clone(),
        signing_address: ctx.accounts.signing_address.to_account_info().clone(),
        authority: ctx.accounts.authority.to_account_info().clone(),
        system_program: ctx.accounts.system_program.to_account_info().clone(),
        registered_verifier_pda: ctx
            .accounts
            .registered_verifier_pda
            .to_account_info()
            .clone(),
        program_merkle_tree: ctx.accounts.program_merkle_tree.to_account_info().clone(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info().clone(),
        pre_inserted_leaves_index: ctx
            .accounts
            .pre_inserted_leaves_index
            .to_account_info()
            .clone(),
        token_program: ctx.accounts.token_program.to_account_info().clone(),
        sender: ctx.accounts.sender.to_account_info().clone(),
        recipient: ctx.accounts.recipient.to_account_info().clone(),
        sender_fee: ctx.accounts.sender_fee.to_account_info().clone(),
        recipient_fee: ctx.accounts.recipient_fee.to_account_info().clone(),
        relayer_recipient: ctx.accounts.relayer_recipient.to_account_info().clone(),
        escrow: ctx.accounts.escrow.to_account_info().clone(),
        token_authority: ctx.accounts.token_authority.to_account_info().clone(),
    };

    let seed = &ctx.accounts.signing_address.key().to_bytes();
    let domain_separation_seed = VERIFIER_STATE_SEED;
    let cpi_seed = &[seed, domain_separation_seed, bump];
    let final_seed = &[&cpi_seed[..]];
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.verifier_program.to_account_info().clone(),
        accounts,
        final_seed,
    );
    cpi_ctx = cpi_ctx.with_remaining_accounts(ctx.remaining_accounts.to_vec());

    verifier_program_two::cpi::shielded_transfer_inputs(
        cpi_ctx,
        proof_verifier,
        ctx.accounts.verifier_state.checked_public_inputs[1].clone(),
    )
}
