use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use anchor_lang::{prelude::*, Bumps};
use light_heap::{bench_sbf_end, bench_sbf_start};
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
    bench_sbf_start!("cpda_sum_check");
    sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
        &inputs.compression_lamports,
        &inputs.is_compress,
    )?;
    bench_sbf_end!("cpda_sum_check");
    // compression lamports ---------------------------------------------------
    bench_sbf_start!("cpda_process_compression");
    if inputs.compression_lamports.is_some() {
        compression_lamports(&inputs, &ctx)?;
    }
    bench_sbf_end!("cpda_process_compression");

    let mut input_compressed_account_hashes =
        vec![[0u8; 32]; inputs.input_compressed_accounts_with_merkle_context.len()];
    let mut input_compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; inputs.input_compressed_accounts_with_merkle_context.len()];

    let mut output_leaf_indices = vec![0u32; inputs.output_compressed_accounts.len()];
    let mut output_compressed_account_hashes =
        vec![[0u8; 32]; inputs.output_compressed_accounts.len()];
    // Bench vs vector
    let mut hashed_pubkeys =
        Vec::<(Pubkey, [u8; 32])>::with_capacity(ctx.remaining_accounts.len() + 1);

    // verify state and or address proof ---------------------------------------------------
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        || !inputs.new_address_params.is_empty()
    {
        // hash input compressed accounts ---------------------------------------------------
        bench_sbf_start!("cpda_hash_input_compressed_accounts");
        if !inputs
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            hash_input_compressed_accounts(
                ctx.remaining_accounts,
                &inputs,
                &mut input_compressed_account_hashes,
                &mut input_compressed_account_addresses,
                &mut hashed_pubkeys,
            )?;
        }

        bench_sbf_end!("cpda_hash_input_compressed_accounts");
        let mut new_addresses = vec![[0u8; 32]; inputs.new_address_params.len()];
        // insert addresses into address merkle tree queue ---------------------------------------------------
        if !new_addresses.is_empty() {
            derive_new_addresses(
                &inputs,
                &ctx,
                &mut input_compressed_account_addresses,
                &mut new_addresses,
            );
            let network_fee_bundle = insert_addresses_into_address_merkle_tree_queue(
                &ctx,
                &new_addresses,
                &inputs.new_address_params,
                &invoking_program,
            )?;
            if let Some(network_fee_bundle) = network_fee_bundle {
                let (remaining_account_index, network_fee) = network_fee_bundle;
                transfer_lamports_cpi(
                    ctx.accounts.get_fee_payer(),
                    &ctx.remaining_accounts[remaining_account_index as usize],
                    network_fee,
                )?;
            }
        }
        bench_sbf_start!("cpda_verify_state_proof");
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
        match verify_state_proof(
            &roots,
            &input_compressed_account_hashes,
            &new_address_roots,
            new_addresses.as_slice(),
            &compressed_verifier_proof,
        ) {
            Ok(_) => anchor_lang::Result::Ok(()),
            Err(e) => {
                msg!(
                    "input_compressed_accounts_with_merkle_context: {:?}",
                    inputs.input_compressed_accounts_with_merkle_context
                );
                Err(e)
            }
        }?;
        bench_sbf_end!("cpda_verify_state_proof");
        // insert nullifiers (input compressed account hashes)---------------------------------------------------
        bench_sbf_start!("cpda_nullifiers");
        if !inputs
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            let network_fee_bundle = insert_nullifiers(
                &inputs,
                &ctx,
                &input_compressed_account_hashes,
                &invoking_program,
            )?;
            if let Some(network_fee_bundle) = network_fee_bundle {
                let (remaining_account_index, network_fee) = network_fee_bundle;
                transfer_lamports_cpi(
                    ctx.accounts.get_fee_payer(),
                    &ctx.remaining_accounts[remaining_account_index as usize],
                    network_fee,
                )?;
            }
        }
        bench_sbf_end!("cpda_nullifiers");
    } else if inputs.proof.is_some() {
        msg!("Proof is some but no input compressed accounts or new addresses provided.");
        return err!(CompressedPdaError::ProofIsSome);
    }
    bench_sbf_end!("cpda_nullifiers");

    // Allocate space for sequence numbers with remaining account length as a proxy.
    // We cannot do that inside of the insert_output_compressed_accounts_into_state_merkle_tree
    // because the function is heap neutral.
    let mut sequence_numbers = Vec::with_capacity(ctx.remaining_accounts.len());
    // insert leaves (output compressed account hashes) ---------------------------------------------------
    if !inputs.output_compressed_accounts.is_empty() {
        bench_sbf_start!("cpda_append");
        insert_output_compressed_accounts_into_state_merkle_tree::<A>(
            &inputs,
            &ctx,
            &mut output_leaf_indices,
            &mut output_compressed_account_hashes,
            &mut input_compressed_account_addresses,
            &invoking_program,
            &mut hashed_pubkeys,
            &mut sequence_numbers,
        )?;
        bench_sbf_end!("cpda_append");
    }
    bench_sbf_start!("emit_state_transition_event");
    // handle the case of unordered multiple output Merkle trees
    sequence_numbers.dedup_by(|a, b| a.pubkey == b.pubkey);
    // reduce the capacity of the sequence numbers vector
    sequence_numbers.shrink_to_fit();
    // emit state transition event ---------------------------------------------------
    bench_sbf_start!("emit_state_transition_event");
    emit_state_transition_event(
        inputs,
        &ctx,
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_leaf_indices,
        sequence_numbers,
    )?;
    bench_sbf_end!("emit_state_transition_event");

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
