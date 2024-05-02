use anchor_lang::{prelude::*, Bumps};
use light_verifier::CompressedProof as CompressedVerifierProof;

use crate::{
    errors::CompressedPdaError,
    invoke::{
        address::{derive_new_addresses, insert_addresses_into_address_merkle_tree_queue},
        append_state::insert_output_compressed_accounts_into_state_merkle_tree,
        emit_event::emit_state_transition_event,
        nullify_state::insert_nullifiers,
        sol_compression::compression_lamports,
        verify_state_proof::{
            fetch_roots, fetch_roots_address_merkle_tree, hash_input_compressed_accounts,
            sum_check, verify_state_proof,
        },
    },
    sdk::accounts::{InvokeAccounts, SignerAccounts},
    InstructionDataInvoke,
};

// TODO: remove once upgraded to anchor 0.30.0 (right now it's required for idl generation)
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

pub fn process<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: InstructionDataInvoke,
    invoking_program: Option<Pubkey>,
    ctx: Context<'a, 'b, 'c, 'info, A>,
) -> Result<()> {
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
    // compression lamports ---------------------------------------------------
    compression_lamports(&inputs, &ctx)?;

    let mut input_compressed_account_hashes =
        vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    let mut input_compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; inputs.input_compressed_accounts_with_merkle_context.len()];

    let mut output_leaf_indices = vec![0u32; inputs.output_compressed_accounts.len()];
    let mut output_compressed_account_hashes =
        vec![[0u8; 32]; inputs.output_compressed_accounts.len()];

    // TODO: add heap neutral
    hash_input_compressed_accounts(
        &ctx,
        &inputs,
        &mut input_compressed_account_hashes,
        &mut input_compressed_account_addresses,
    )?;
    let mut new_addresses = vec![[0u8; 32]; inputs.new_address_params.len()];
    // insert addresses into address merkle tree queue ---------------------------------------------------
    if !new_addresses.is_empty() {
        derive_new_addresses(
            &inputs,
            &ctx,
            &mut input_compressed_account_addresses,
            &mut new_addresses,
        );
        insert_addresses_into_address_merkle_tree_queue(
            &ctx,
            &new_addresses,
            &inputs.new_address_params,
            &invoking_program,
        )?;
    }
    // verify state and or address proof ---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        || !inputs.new_address_params.is_empty()
    {
        let mut new_address_roots = vec![[0u8; 32]; inputs.new_address_params.len()];
        // TODO: enable once address merkle tree init is debugged
        fetch_roots_address_merkle_tree(&inputs.new_address_params, &ctx, &mut new_address_roots)?;
        let mut roots = vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
        fetch_roots(&inputs, &ctx, &mut roots)?;
        let proof = match &inputs.proof {
            Some(proof) => proof,
            None => return err!(CompressedPdaError::ProofIsNone),
        };
        let compressed_verifier_proof = CompressedVerifierProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        };
        verify_state_proof(
            &roots,
            &input_compressed_account_hashes,
            &new_address_roots,
            new_addresses.as_slice(),
            &compressed_verifier_proof,
        )?;
    }

    // insert nullifies (input compressed account hashes)---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        insert_nullifiers(
            &inputs,
            &ctx,
            &input_compressed_account_hashes,
            &invoking_program,
        )?;
    }

    const ITER_SIZE: usize = 14;
    // insert leaves (output compressed account hashes) ---------------------------------------------------
    if !inputs.output_compressed_accounts.is_empty() {
        let mut i = 0;
        for _ in inputs.output_compressed_accounts.iter().step_by(ITER_SIZE) {
            insert_output_compressed_accounts_into_state_merkle_tree::<ITER_SIZE, A>(
                &inputs,
                &ctx,
                &mut output_leaf_indices,
                &mut output_compressed_account_hashes,
                &mut input_compressed_account_addresses,
                &mut i,
                &invoking_program,
            )?;
        }
    }

    // emit state transition event ---------------------------------------------------
    emit_state_transition_event(
        inputs,
        &ctx,
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_leaf_indices,
    )?;

    Ok(())
}

// TODO: refactor to compressed_account
// #[derive(Debug)]
// #[account]
// pub struct InstructionDataInvoke2 {
//     pub proof: Option<CompressedProof>,
//     pub low_element_indices: Vec<u16>,
//     pub root_indices: Vec<u16>,
//     pub relay_fee: Option<u64>,
//     pub utxos: SerializedUtxos,
// }

// pub fn into_inputs(
//     inputs: InstructionDataInvoke2,
//     accounts: &[Pubkey],
//     remaining_accounts: &[Pubkey],
// ) -> Result<InstructionDataInvoke> {
//     let input_compressed_accounts_with_merkle_context = inputs
//         .utxos
//         .input_compressed_accounts_from_serialized_utxos(accounts, remaining_accounts)
//         .unwrap();
//     let output_compressed_accounts = inputs
//         .utxos
//         .output_compressed_accounts_from_serialized_utxos(accounts)
//         .unwrap();
//     Ok(InstructionDataInvoke {
//         proof: inputs.proof,
//         low_element_indices: inputs.low_element_indices,
//         root_indices: inputs.root_indices,
//         relay_fee: inputs.relay_fee,
//         input_compressed_accounts_with_merkle_context,
//         output_compressed_accounts,
//     })
// }
