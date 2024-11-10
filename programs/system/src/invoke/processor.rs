use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use anchor_lang::{prelude::*, Bumps};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_verifier::CompressedProof as CompressedVerifierProof;

use crate::{
    errors::SystemProgramError,
    invoke::{
        address::{derive_new_addresses, insert_addresses_into_address_merkle_tree_queue},
        append_state::insert_output_compressed_accounts_into_state_merkle_tree,
        emit_event::emit_state_transition_event,
        nullify_state::insert_nullifiers,
        sol_compression::compress_or_decompress_lamports,
        sum_check::sum_check,
        verify_state_proof::{
            create_tx_hash, fetch_input_compressed_account_roots, fetch_roots_address_merkle_tree,
            hash_input_compressed_accounts, verify_state_proof,
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

/// Steps:
/// 1. Sum check
/// 2. Compression lamports
/// 3. Verify state inclusion & address non-inclusion proof
/// 4. Insert nullifiers
/// 5. Insert output compressed accounts into state Merkle tree
/// 6. Emit state transition event
pub fn process<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    mut inputs: InstructionDataInvoke,
    invoking_program: Option<Pubkey>,
    ctx: Context<'a, 'b, 'c, 'info, A>,
    cpi_context_inputs: usize,
) -> Result<()> {
    if inputs.relay_fee.is_some() {
        unimplemented!("Relay fee is not implemented yet.");
    }
    // Sum check ---------------------------------------------------
    bench_sbf_start!("cpda_sum_check");
    sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
        &inputs.compress_or_decompress_lamports,
        &inputs.is_compress,
    )?;
    bench_sbf_end!("cpda_sum_check");
    // Compress or decompress lamports ---------------------------------------------------
    bench_sbf_start!("cpda_process_compression");
    if inputs.compress_or_decompress_lamports.is_some() {
        if inputs.is_compress && ctx.accounts.get_decompression_recipient().is_some() {
            return err!(SystemProgramError::DecompressionRecipientDefined);
        }
        compress_or_decompress_lamports(&inputs, &ctx)?;
    } else if ctx.accounts.get_decompression_recipient().is_some() {
        return err!(SystemProgramError::DecompressionRecipientDefined);
    } else if ctx.accounts.get_sol_pool_pda().is_some() {
        return err!(SystemProgramError::SolPoolPdaDefined);
    }
    bench_sbf_end!("cpda_process_compression");

    // Allocate heap memory here so that we can free memory after function invocations.
    let num_input_compressed_accounts = inputs.input_compressed_accounts_with_merkle_context.len();
    let num_new_addresses = inputs.new_address_params.len();
    let num_output_compressed_accounts = inputs.output_compressed_accounts.len();
    let mut input_compressed_account_hashes = vec![[0u8; 32]; num_input_compressed_accounts];

    let mut compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; num_input_compressed_accounts + num_new_addresses];
    let mut output_leaf_indices = vec![0u32; num_output_compressed_accounts];
    let mut output_compressed_account_hashes = vec![[0u8; 32]; num_output_compressed_accounts];
    // hashed_pubkeys_capacity is the maximum of hashed pubkey the tx could have.
    // 1 owner pubkey inputs + every remaining account pubkey can be a tree + every output can be owned by a different pubkey
    // + number of times cpi context account was filled.
    let hashed_pubkeys_capacity =
        1 + ctx.remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs;
    let mut hashed_pubkeys = Vec::<(Pubkey, [u8; 32])>::with_capacity(hashed_pubkeys_capacity);
    let mut num_prove_by_index_accounts: usize = 0;

    // Verify state and or address proof ---------------------------------------------------

    // Allocate heap memory here because roots are only used for proof verification.
    let mut new_address_roots = vec![[0u8; 32]; num_new_addresses];
    let mut input_compressed_account_roots = Vec::with_capacity(num_input_compressed_accounts);
    // hash input compressed accounts ---------------------------------------------------
    bench_sbf_start!("cpda_hash_input_compressed_accounts");
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        hash_input_compressed_accounts(
            ctx.remaining_accounts,
            &inputs.input_compressed_accounts_with_merkle_context,
            &mut input_compressed_account_hashes,
            &mut compressed_account_addresses,
            &mut hashed_pubkeys,
        )?;
        // # Safety this is a safeguard for memory safety.
        // This error should never be triggered.
        if hashed_pubkeys.capacity() != hashed_pubkeys_capacity {
            msg!(
                "hashed_pubkeys exceeded capacity. Used {}, allocated {}.",
                hashed_pubkeys.capacity(),
                hashed_pubkeys_capacity
            );
            return err!(SystemProgramError::InvalidCapacity);
        }

        fetch_input_compressed_account_roots(
            &inputs.input_compressed_accounts_with_merkle_context,
            &ctx,
            &mut input_compressed_account_roots,
        )?;
    }

    bench_sbf_end!("cpda_hash_input_compressed_accounts");
    let mut new_addresses = vec![[0u8; 32]; num_new_addresses];
    // Insert addresses into address merkle tree queue ---------------------------------------------------
    let address_network_fee_bundle = if !new_addresses.is_empty() {
        derive_new_addresses(
            &inputs.new_address_params,
            num_input_compressed_accounts,
            ctx.remaining_accounts,
            &mut compressed_account_addresses,
            &mut new_addresses,
        )?;
        fetch_roots_address_merkle_tree(&inputs.new_address_params, &ctx, &mut new_address_roots)?;

        insert_addresses_into_address_merkle_tree_queue(
            &ctx,
            &new_addresses,
            &inputs.new_address_params,
            &invoking_program,
        )?
    } else {
        None
    };

    // Allocate space for sequence numbers with remaining account length as a
    // proxy. We cannot allocate heap memory in
    // insert_output_compressed_accounts_into_state_merkle_tree because it is
    // heap neutral.
    let mut sequence_numbers = Vec::with_capacity(ctx.remaining_accounts.len());
    // Insert leaves (output compressed account hashes) ---------------------------------------------------
    let output_network_fee_bundle = if !inputs.output_compressed_accounts.is_empty() {
        bench_sbf_start!("cpda_append");
        let network_fee_bundle = insert_output_compressed_accounts_into_state_merkle_tree(
            &mut inputs.output_compressed_accounts,
            &ctx,
            &mut output_leaf_indices,
            &mut output_compressed_account_hashes,
            &mut compressed_account_addresses,
            &invoking_program,
            &mut hashed_pubkeys,
            &mut sequence_numbers,
        )?;
        // # Safety this is a safeguard for memory safety.
        // This error should never be triggered.
        if hashed_pubkeys.capacity() != hashed_pubkeys_capacity {
            msg!(
                "hashed_pubkeys exceeded capacity. Used {}, allocated {}.",
                hashed_pubkeys.capacity(),
                hashed_pubkeys_capacity
            );
            return err!(SystemProgramError::InvalidCapacity);
        }
        bench_sbf_end!("cpda_append");
        network_fee_bundle
    } else {
        None
    };
    bench_sbf_start!("emit_state_transition_event");
    // Reduce the capacity of the sequence numbers vector.
    sequence_numbers.shrink_to_fit();

    // insert nullifiers (input compressed account hashes)---------------------------------------------------
    // Nullifiers need to be inserted befor proof verification because the
    // in certain cases we zero out roots in batched input queues.
    // These roots need to be zero prior to proof verification.
    bench_sbf_start!("cpda_nullifiers");
    let input_network_fee_bundle = if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        // Access the current slot
        let current_slot = Clock::get()?.slot;
        let tx_hash = create_tx_hash(
            &input_compressed_account_hashes,
            &output_compressed_account_hashes,
            current_slot,
        );
        msg!("tx_hash {:?}", tx_hash);
        msg!("current_slot {:?}", current_slot);

        let leaf_indices: Vec<u32> = inputs
            .input_compressed_accounts_with_merkle_context
            .iter()
            .map(|x| x.merkle_context.leaf_index)
            .collect();
        insert_nullifiers(
            &inputs.input_compressed_accounts_with_merkle_context,
            &ctx,
            &input_compressed_account_hashes,
            &leaf_indices,
            &invoking_program,
            tx_hash,
            &mut num_prove_by_index_accounts,
        )?
    } else {
        None
    };
    bench_sbf_end!("cpda_nullifiers");

    // Transfer network fee
    transfer_network_fee(
        &ctx,
        input_network_fee_bundle,
        address_network_fee_bundle,
        output_network_fee_bundle,
    )?;
    let proof_input_input_compressed_account_hashes = input_compressed_account_hashes
        .iter()
        .enumerate()
        .filter(|(x, _)| {
            inputs.input_compressed_accounts_with_merkle_context[*x]
                .merkle_context
                .queue_index
                .is_none()
        })
        .map(|x| *x.1)
        .collect::<Vec<[u8; 32]>>();
    if !proof_input_input_compressed_account_hashes.is_empty()
        || !inputs.new_address_params.is_empty()
    {
        bench_sbf_start!("cpda_verify_state_proof");
        if let Some(proof) = inputs.proof.as_ref() {
            bench_sbf_start!("cpda_verify_state_proof");
            let compressed_verifier_proof = CompressedVerifierProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            };

            match verify_state_proof(
                &input_compressed_account_roots,
                &proof_input_input_compressed_account_hashes,
                &new_address_roots,
                &new_addresses,
                &compressed_verifier_proof,
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    msg!("proof  {:?}", proof);
                    msg!(
                        "input_compressed_account_hashes {:?}",
                        proof_input_input_compressed_account_hashes
                    );
                    msg!("input roots {:?}", input_compressed_account_roots);
                    msg!(
                        "input_compressed_accounts_with_merkle_context: {:?}",
                        inputs.input_compressed_accounts_with_merkle_context
                    );
                    Err(e)
                }
            }?;
            bench_sbf_end!("cpda_verify_state_proof");
        }
        // Proof can be None if all input compressed accounts are proven by index.
        else if num_prove_by_index_accounts
            == inputs.input_compressed_accounts_with_merkle_context.len()
        {
            // Do nothing
        } else {
            return err!(SystemProgramError::ProofIsNone);
        }
    } else if inputs.proof.is_some() {
        return err!(SystemProgramError::ProofIsSome);
    } else if inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        && inputs.new_address_params.is_empty()
        && inputs.output_compressed_accounts.is_empty()
    {
        return err!(SystemProgramError::EmptyInputs);
    }
    bench_sbf_end!("cpda_nullifiers");

    // Emit state transition event ---------------------------------------------------
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

/// Network fee distribution:
/// - if any account is created or modified -> transfer network fee (5000 lamports)
/// (Previously we didn't charge for appends now we have to since values go into a queue.)
/// - if an address is created -> transfer an additional network fee (5000 lamports)
///
/// Examples:
/// 1. create account with address    network fee 10,000 lamports
/// 2. token transfer                 network fee 5,000 lamports
/// 3. mint to                        network fee 5,000 lamports
#[inline(always)]
fn transfer_network_fee<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    ctx: &Context<'a, 'b, 'c, 'info, A>,
    input_network_fee_bundle: Option<(u8, u64)>,
    address_network_fee_bundle: Option<(u8, u64)>,
    output_network_fee_bundle: Option<(u8, u64)>,
) -> Result<()> {
    if let Some(network_fee_bundle) = input_network_fee_bundle {
        let address_fee = if let Some(network_fee_bundle) = address_network_fee_bundle {
            let (_, network_fee) = network_fee_bundle;
            network_fee
        } else {
            0
        };
        let (remaining_account_index, mut network_fee) = network_fee_bundle;
        network_fee += address_fee;
        transfer_lamports_cpi(
            ctx.accounts.get_fee_payer(),
            &ctx.remaining_accounts[remaining_account_index as usize],
            network_fee,
        )?;
    } else if let Some(network_fee_bundle) = output_network_fee_bundle {
        let address_fee = if let Some(network_fee_bundle) = address_network_fee_bundle {
            let (_, network_fee) = network_fee_bundle;
            network_fee
        } else {
            0
        };
        let (remaining_account_index, mut network_fee) = network_fee_bundle;
        network_fee += address_fee;
        transfer_lamports_cpi(
            ctx.accounts.get_fee_payer(),
            &ctx.remaining_accounts[remaining_account_index as usize],
            network_fee,
        )?;
    } else if let Some(network_fee_bundle) = address_network_fee_bundle {
        let (remaining_account_index, network_fee) = network_fee_bundle;
        transfer_lamports_cpi(
            ctx.accounts.get_fee_payer(),
            &ctx.remaining_accounts[remaining_account_index as usize],
            network_fee,
        )?;
    }
    Ok(())
}
