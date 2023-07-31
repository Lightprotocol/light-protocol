use crate::verifying_key::VERIFYINGKEY;
use crate::LightInstructionFirst;
use crate::LightInstructionThird;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use light_macros::pubkey;
use light_verifier_sdk::light_transaction::Amounts;
use light_verifier_sdk::light_transaction::Proof;
use light_verifier_sdk::light_transaction::TransactionInput;
use light_verifier_sdk::light_transaction::VERIFIER_STATE_SEED;
use light_verifier_sdk::{
    light_app_transaction::AppTransaction,
    light_transaction::{Config, Transaction},
};

#[derive(Clone)]
pub struct TransactionsConfig;
impl Config for TransactionsConfig {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize = 4;
    /// Number of output utxos.
    const NR_LEAVES: usize = 4;
    /// ProgramId.
    const ID: Pubkey = pubkey!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
}

<<<<<<< HEAD
pub fn process_transfer_4_ins_4_outs_4_checked_first<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info>>,
=======
pub fn process_transfer_4_ins_4_outs_4_checked_first<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_PUBLIC_INPUTS: usize,
>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionFirst<'info, NR_CHECKED_INPUTS>>,
>>>>>>> main
    proof: &'a Proof,
    public_amount: &'a Amounts,
    input_nullifier: &'a [[u8; 32]; 4],
    output_commitment: &'a [[u8; 32]; 4],
<<<<<<< HEAD
    checked_public_inputs: &'a Vec<Vec<u8>>,
=======
    checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
>>>>>>> main
    encrypted_utxos: &'a Vec<u8>,
    pool_type: &'a [u8; 32],
    root_index: &'a u64,
    relayer_fee: &'a u64,
) -> Result<()> {
    let output_commitment = [
        [output_commitment[0], output_commitment[1]],
        [output_commitment[2], output_commitment[3]],
    ];
    let input = TransactionInput {
        message: None,
        proof,
        public_amount,
        nullifiers: input_nullifier,
        leaves: &output_commitment,
        encrypted_utxos,
        relayer_fee: *relayer_fee,
        merkle_root_index: (*root_index).try_into().unwrap(),
        pool_type,
        checked_public_inputs,
        accounts: None,
        verifyingkey: &VERIFYINGKEY,
    };
<<<<<<< HEAD
    let tx = Transaction::<2, 4, TransactionsConfig>::new(input);
=======
    let tx =
        Transaction::<NR_CHECKED_INPUTS, 2, 4, NR_PUBLIC_INPUTS, TransactionsConfig>::new(input);
>>>>>>> main
    ctx.accounts.verifier_state.set_inner(tx.into());
    ctx.accounts.verifier_state.signer = *ctx.accounts.signing_address.key;
    Ok(())
}

<<<<<<< HEAD
pub fn process_transfer_4_ins_4_outs_4_checked_third<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionThird<'info>>,
=======
pub fn process_transfer_4_ins_4_outs_4_checked_third<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
>(
    ctx: Context<'a, 'b, 'c, 'info, LightInstructionThird<'info, NR_CHECKED_INPUTS>>,
>>>>>>> main
    proof_app: &'a Proof,
    proof_verifier: &'a Proof,
) -> Result<()> {
    let current_slot = <Clock as sysvar::Sysvar>::get()?.slot;
    msg!(
        "{} > {}",
        current_slot,
        u64::from_be_bytes(
            ctx.accounts.verifier_state.checked_public_inputs[2][24..32]
                .try_into()
                .unwrap(),
        )
    );
    if current_slot
        < u64::from_be_bytes(
            ctx.accounts.verifier_state.checked_public_inputs[2][24..32]
                .try_into()
                .unwrap(),
        )
    {
        panic!("Escrow still locked");
    }
    // verify app proof
<<<<<<< HEAD
    let mut app_verifier = AppTransaction::<TransactionsConfig>::new(
        proof_app,
        ctx.accounts.verifier_state.checked_public_inputs.clone(),
=======
    let mut app_verifier = AppTransaction::<NR_CHECKED_INPUTS, TransactionsConfig>::new(
        proof_app,
        &ctx.accounts.verifier_state.checked_public_inputs,
>>>>>>> main
        &VERIFYINGKEY,
    );

    app_verifier.verify()?;

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
        transaction_merkle_tree: ctx
            .accounts
            .transaction_merkle_tree
            .to_account_info()
            .clone(),
        token_program: ctx.accounts.token_program.to_account_info().clone(),
        sender_spl: ctx.accounts.sender_spl.to_account_info().clone(),
        recipient_spl: ctx.accounts.recipient_spl.to_account_info().clone(),
        sender_sol: ctx.accounts.sender_sol.to_account_info().clone(),
        recipient_sol: ctx.accounts.recipient_sol.to_account_info().clone(),
        // relayer recipient and escrow will never be used in the same transaction
        relayer_recipient_sol: ctx.accounts.relayer_recipient_sol.to_account_info().clone(),
        token_authority: ctx.accounts.token_authority.to_account_info().clone(),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
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
        proof_verifier.a,
        proof_verifier.b,
        proof_verifier.c,
        <Vec<u8> as TryInto<[u8; 32]>>::try_into(
            ctx.accounts.verifier_state.checked_public_inputs[1].to_vec(),
        )
        .unwrap(),
    )
}
