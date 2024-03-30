use std::borrow::Borrow;

use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;

use crate::{
    append_state::insert_output_compressed_accounts_into_state_merkle_tree,
    compressed_account::{derive_address, CompressedAccount, CompressedAccountWithMerkleContext},
    compression_lamports,
    create_address::insert_addresses_into_address_merkle_tree_queue,
    event::{emit_state_transition_event, PublicTransactionEvent},
    nullify_state::insert_nullifiers,
    utils::CompressedProof,
    verify_state::{
        fetch_roots, hash_input_compressed_accounts, signer_check, sum_check, verify_state_proof,
    },
    CompressedSolPda, ErrorCode,
};
pub fn process_execute_compressed_transaction<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
) -> anchor_lang::Result<PublicTransactionEvent> {
    // sum check ---------------------------------------------------
    // the sum of in compressed accounts and compressed accounts must be equal minus the relay fee
    sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
        &inputs.compression_lamports,
        &inputs.is_compress,
    )?;
    msg!("sum check success");
    // signer check ---------------------------------------------------
    signer_check(inputs, ctx)?;
    // TODO: if not proof store the instruction in cpi_signature_account and set cpi account slot to current slot, if slot is not current slot override the vector with a new vector
    // TODO: add security check that only data from the current transaction stored in cpi account can be used in the current transaction
    // TODO: add check that cpi account was derived from a Merkle tree account in the current transaction
    // TODO: add check that if compressed account is program owned that it is signed by the program (if an account has data it is program owned, if the program account is set compressed accounts are program owned)
    match ctx.accounts.cpi_signature_account.borrow() {
        Some(_cpi_signature_account) => {
            // needs to check every compressed account and make sure that signaures exist in cpi_signature_account
            msg!("cpi_signature check is not implemented");
            err!(ErrorCode::CpiSignerCheckFailed)
        }
        None => Ok(()),
    }?;
    // compression_lamports ---------------------------------------------------
    compression_lamports(inputs, ctx)?;

    let mut roots = vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    fetch_roots(inputs, ctx, &mut roots)?;
    let address_roots = vec![[0u8; 32]; inputs.address_merkle_tree_root_indices.len()];
    // TODO: enable once address merkle tree init is debugged
    // fetch_roots_address_merkle_tree(
    //     inputs,
    //     ctx,
    //     &inputs.address_merkle_tree_root_indices,
    //     &mut address_roots,
    // )?;

    let mut input_compressed_account_hashes =
        vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    let mut input_compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; inputs.input_compressed_accounts_with_merkle_context.len()];

    let mut output_leaf_indices = vec![0u32; inputs.output_compressed_accounts.len()];
    let mut output_compressed_account_hashes =
        vec![[0u8; 32]; inputs.output_compressed_accounts.len()];
    let mut new_addresses = vec![[0u8; 32]; inputs.new_address_seeds.len()];
    // insert addresses into address merkle tree queue ---------------------------------------------------
    if !inputs.new_address_seeds.is_empty() {
        derive_new_addresses(
            inputs,
            ctx,
            &mut input_compressed_account_addresses,
            &mut new_addresses,
        );
        insert_addresses_into_address_merkle_tree_queue(inputs, ctx, &new_addresses)?;
    }
    // TODO: add heap neutral
    hash_input_compressed_accounts(
        ctx,
        inputs,
        &mut input_compressed_account_hashes,
        &mut input_compressed_account_addresses,
    )?;

    // TODO: add heap neutral
    // verify inclusion proof ---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        || !inputs.new_address_seeds.is_empty()
    {
        verify_state_proof(
            &roots,
            &input_compressed_account_hashes,
            &address_roots,
            new_addresses.as_slice(),
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
            &mut input_compressed_account_addresses,
        )?;
    }

    // emit state transition event ---------------------------------------------------
    emit_state_transition_event(
        inputs,
        ctx,
        &input_compressed_account_hashes,
        &output_compressed_account_hashes,
        &output_leaf_indices,
    )
}

// DO NOT MAKE HEAP NEUTRAL: this function allocates new heap memory
pub fn derive_new_addresses(
    inputs: &InstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction<'_>>,
    input_compressed_account_addresses: &mut Vec<Option<[u8; 32]>>,
    new_addresses: &mut [[u8; 32]],
) {
    inputs
        .new_address_seeds
        .iter()
        .enumerate()
        .for_each(|(i, seed)| {
            let address = derive_address(
                &ctx.remaining_accounts[inputs.address_merkle_tree_account_indices[i] as usize]
                    .key(),
                seed,
            )
            .unwrap();
            input_compressed_account_addresses.push(Some(address));
            new_addresses[i] = address;
        });
}

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    pub signer: Signer<'info>,
    /// CHECK: this account
    #[account(
    seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
    )]
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program: Program<'info, AccountCompression>,
    pub cpi_signature_account: Option<Account<'info, CpiSignatureAccount>>,
    pub invoking_program: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub compressed_sol_pda: Option<Account<'info, CompressedSolPda>>,
    #[account(mut)]
    pub compression_recipient: Option<UncheckedAccount<'info>>,
    pub system_program: Option<Program<'info, System>>,
}

/// collects invocations without proofs
/// invocations are collected and processed when an invocation with a proof is received
#[account]
pub struct CpiSignatureAccount {
    pub slot: u64,
    pub signatures: Vec<InstructionDataTransfer>,
}

// TODO: add checks for lengths of vectors
#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub input_root_indices: Vec<u16>,
    pub new_address_seeds: Vec<[u8; 32]>,
    pub address_queue_account_indices: Vec<u8>,
    pub address_merkle_tree_account_indices: Vec<u8>,
    pub address_merkle_tree_root_indices: Vec<u16>,
    pub input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    /// The indices of the accounts in the output state merkle tree.
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub relay_fee: Option<u64>,
    pub compression_lamports: Option<u64>,
    pub is_compress: bool,
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
