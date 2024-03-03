use std::borrow::Borrow;

use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use light_verifier_sdk::light_transaction::ProofCompressed;

use crate::{
    append_state::insert_out_utxos,
    event::{emit_state_transition_event, PublicTransactionEvent},
    nullify_state::insert_nullifiers,
    tlv::TlvDataElement,
    utxo::{OutUtxo, SerializedUtxos, Utxo},
    verify_state::{fetch_roots, hash_in_utxos, out_utxos_to_utxos, sum_check},
    ErrorCode,
};
pub fn process_execute_compressed_transaction<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
) -> anchor_lang::Result<PublicTransactionEvent> {
    // sum check ---------------------------------------------------
    sum_check(&inputs.in_utxos, &inputs.out_utxos, &inputs.rpc_fee)?;
    msg!("sum check success");
    // signer check ---------------------------------------------------
    // TODO: change the match statement so that we signers for every utxo as soon as any in utxo has tlv
    // and we need to use the provided tlv in out utxos
    // TODO: add check that if utxo is program owned that it is signed by the program
    match ctx.accounts.cpi_signature_account.borrow() {
        Some(_cpi_signature_account) => {
            // needs to check every piece of tlv and make sure that signaures exist in cpi_signature_account
            msg!("cpi_signature check is not implemented");
            err!(ErrorCode::CpiSignerCheckFailed)
        }
        None => inputs.in_utxos.iter().try_for_each(|(utxo, _, _)| {
            if utxo.owner != ctx.accounts.signer.key() {
                err!(ErrorCode::SignerCheckFailed)
            } else {
                Ok(())
            }
        }),
    }?;

    let mut roots = vec![[0u8; 32]; inputs.in_utxos.len()];
    fetch_roots(inputs, ctx, &mut roots)?;

    let mut utxo_hashes = vec![[0u8; 32]; inputs.in_utxos.len()];
    let mut out_utxos = vec![Utxo::default(); inputs.out_utxos.len()];
    let mut out_utxo_indices = vec![0u32; inputs.out_utxos.len()];
    // TODO: add heap neutral
    hash_in_utxos(inputs, &mut utxo_hashes)?;
    // TODO: add heap neutral
    out_utxos_to_utxos(inputs, ctx, &mut out_utxos, &mut out_utxo_indices)?;
    // TODO: verify inclusion proof ---------------------------------------------------

    // insert nullifiers (in utxo hashes)---------------------------------------------------
    if !inputs.in_utxos.is_empty() {
        insert_nullifiers(inputs, ctx, &utxo_hashes)?;
    }
    // insert leaves (out utxo hashes) ---------------------------------------------------
    if !inputs.out_utxos.is_empty() {
        insert_out_utxos(inputs, ctx)?;
    }

    emit_state_transition_event(inputs, ctx, &out_utxos, &out_utxo_indices)
}

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each in utxo one queue and Merkle tree account each for each out utxo.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    #[account(mut)]
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
}

#[account]
pub struct CpiSignatureAccount {
    pub signatures: Vec<CpiSignature>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiSignature {
    pub program: Pubkey,
    pub tlv_hash: [u8; 32],
    pub tlv_data: TlvDataElement,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer {
    pub proof: Option<ProofCompressed>,
    // TODO: remove low_element_indices
    pub low_element_indices: Vec<u16>,
    pub root_indices: Vec<u16>,
    pub rpc_fee: Option<u64>,
    pub in_utxos: Vec<(Utxo, u8, u8)>, // index of Merkle tree, nullifier queue account in remaining accounts
    pub out_utxos: Vec<(OutUtxo, u8)>, // index of Merkle tree account in remaining accounts
}
// TODO: add new remaining account indices in SerializedUtxos
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer2 {
    pub proof: Option<ProofCompressed>,
    pub low_element_indices: Vec<u16>,
    pub root_indices: Vec<u16>,
    pub rpc_fee: Option<u64>,
    pub utxos: SerializedUtxos,
}

pub fn into_inputs(
    inputs: InstructionDataTransfer2,
    accounts: &[Pubkey],
    remaining_accounts: &[Pubkey],
) -> Result<InstructionDataTransfer> {
    let in_utxos = inputs
        .utxos
        .in_utxos_from_serialized_utxos(accounts, remaining_accounts)
        .unwrap();
    let out_utxos = inputs
        .utxos
        .out_utxos_from_serialized_utxos(accounts)
        .unwrap();
    Ok(InstructionDataTransfer {
        proof: inputs.proof,
        low_element_indices: inputs.low_element_indices,
        root_indices: inputs.root_indices,
        rpc_fee: inputs.rpc_fee,
        in_utxos,
        out_utxos,
    })
}
