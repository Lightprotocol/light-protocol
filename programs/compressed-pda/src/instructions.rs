use std::borrow::Borrow;

use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;

// use light_verifier_sdk::light_transaction::CompressedProof;
use crate::{
    append_state::insert_output_compressed_accounts_into_state_merkle_tree,
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    event::{emit_state_transition_event, PublicTransactionEvent},
    nullify_state::insert_nullifiers,
    utils::CompressedProof,
    verify_state::{
        fetch_roots, hash_input_compressed_accounts, sum_check, verify_merkle_proof_zkp,
    },
    ErrorCode,
};
pub fn process_execute_compressed_transaction<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
) -> anchor_lang::Result<PublicTransactionEvent> {
    // sum check ---------------------------------------------------
    sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
    )?;
    msg!("sum check success");
    // signer check ---------------------------------------------------
    // TODO: remove match statement to use the same signer check always
    // TODO: if not proof store the instruction in cpi_signature_account and set cpi account slot to current slot, if slot is not current slot override the vector with a new vector
    // TODO: add security check that only data from the current transaction stored in cpi account can be used in the current transaction
    // TODO: add check that cpi account was derived from a Merkle tree account in the current transaction
    // TODO: add check that if compressed account is program owned that it is signed by the program (if an account has data it is program owned, if the program account is set compressed accounts are program owned)
    match ctx.accounts.cpi_signature_account.borrow() {
        Some(_cpi_signature_account) => {
            // needs to check every compredssed account and make sure that signaures exist in cpi_signature_account
            msg!("cpi_signature check is not implemented");
            err!(ErrorCode::CpiSignerCheckFailed)
        }
        None => inputs
            .input_compressed_accounts_with_merkle_context
            .iter()
            .try_for_each(|compressed_accounts: &CompressedAccountWithMerkleContext| {
                // TODO(@ananas-block): revisit program signer check
                // Two options:
                // 1. we require the program as an account and reconstruct the cpi signer to check that the cpi signer is a pda of the program
                //   - The advantage is that the compressed account can be owned by the program_id
                // 2. we set a deterministic pda signer for every program eg seeds = [b"cpi_authority"]
                //   - The advantages are that the program does not need to be an account, and we don't need to reconstruct the pda -> more efficient (costs are just low hundreds of cu though)
                //   - The drawback is that the pda signer is the owner of the compressed account which is confusing
                if compressed_accounts.compressed_account.data.is_some() {
                    let invoking_program_id = ctx.accounts.invoking_program.as_ref().unwrap().key();
                    let signer = anchor_lang::prelude::Pubkey::find_program_address(
                        &[b"cpi_authority"],
                        &invoking_program_id,
                    )
                    .0;
                    if signer != ctx.accounts.signer.key()
                        && invoking_program_id != compressed_accounts.compressed_account.owner
                    {
                        msg!(
                            "program signer check failed derived cpi signer {} !=  signer {}",
                            compressed_accounts.compressed_account.owner,
                            ctx.accounts.signer.key()
                        );
                        msg!(
                            "program signer check failed compressed account owner {} !=  invoking_program_id {}",
                            compressed_accounts.compressed_account.owner,
                            invoking_program_id
                        );
                        err!(ErrorCode::SignerCheckFailed)
                    } else {
                        Ok(())
                    }
                } else if compressed_accounts.compressed_account.owner != ctx.accounts.signer.key()
                {
                    msg!(
                        "signer check failed compressed account owner {} !=  signer {}",
                        compressed_accounts.compressed_account.owner,
                        ctx.accounts.signer.key()
                    );
                    err!(ErrorCode::SignerCheckFailed)
                } else {
                    Ok(())
                }
            }),
    }?;

    let mut roots = vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    fetch_roots(inputs, ctx, &mut roots)?;

    let mut input_compressed_account_hashes =
        vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    let mut output_leaf_indices = vec![0u32; inputs.output_compressed_accounts.len()];
    let mut output_compressed_account_hashes =
        vec![[0u8; 32]; inputs.output_compressed_accounts.len()];
    // TODO: add heap neutral
    hash_input_compressed_accounts(ctx, inputs, &mut input_compressed_account_hashes)?;
    // TODO: add heap neutral
    // verify inclusion proof ---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        verify_merkle_proof_zkp(
            &roots,
            &input_compressed_account_hashes,
            inputs.proof.as_ref().unwrap(),
        )?;
    }
    // insert nullifiers (input compressed account hashes)---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        insert_nullifiers(inputs, ctx, &input_compressed_account_hashes)?;
    }
    // insert leaves (output compressed account hashes) ---------------------------------------------------
    if !inputs.output_compressed_accounts.is_empty() {
        insert_output_compressed_accounts_into_state_merkle_tree(
            inputs,
            ctx,
            &mut output_leaf_indices,
            &mut output_compressed_account_hashes,
        )?;
    }

    emit_state_transition_event(
        inputs,
        ctx,
        &input_compressed_account_hashes,
        &output_compressed_account_hashes,
        &output_leaf_indices,
    )
}

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    pub signer: Signer<'info>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program: Program<'info, AccountCompression>,
    pub cpi_signature_account: Option<Account<'info, CpiSignatureAccount>>,
    pub invoking_program: Option<UncheckedAccount<'info>>,
}

/// collects invocations without proofs
/// invocations are collected and processed when an invocation with a proof is received
#[account]
pub struct CpiSignatureAccount {
    pub slot: u64,
    pub signatures: Vec<InstructionDataTransfer>,
}

#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub input_root_indices: Vec<u16>,
    pub input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    /// The indices of the accounts in the output state merkle tree.
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub relay_fee: Option<u64>,
}

// TODO: refactor to compressed_account
// #[derive(Debug)]
// #[account]
// pub struct InstructionDataTransfer2 {
//     pub proof: Option<CompressedProof>,
//     pub low_element_indices: Vec<u16>,
//     pub root_indices: Vec<u16>,
//     pub relay_fee: Option<u64>,
//     pub utxos: SerializedUtxos,
// }

// pub fn into_inputs(
//     inputs: InstructionDataTransfer2,
//     accounts: &[Pubkey],
//     remaining_accounts: &[Pubkey],
// ) -> Result<InstructionDataTransfer> {
//     let input_compressed_accounts_with_merkle_context = inputs
//         .utxos
//         .input_compressed_accounts_from_serialized_utxos(accounts, remaining_accounts)
//         .unwrap();
//     let output_compressed_accounts = inputs
//         .utxos
//         .output_compressed_accounts_from_serialized_utxos(accounts)
//         .unwrap();
//     Ok(InstructionDataTransfer {
//         proof: inputs.proof,
//         low_element_indices: inputs.low_element_indices,
//         root_indices: inputs.root_indices,
//         relay_fee: inputs.relay_fee,
//         input_compressed_accounts_with_merkle_context,
//         output_compressed_accounts,
//     })
// }
