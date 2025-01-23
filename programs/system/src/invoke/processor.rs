use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use anchor_lang::{prelude::*, Bumps};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_utils::hashchain::create_tx_hash;
use light_verifier::CompressedProof as CompressedVerifierProof;

use super::PackedReadOnlyAddress;
use crate::{
    errors::SystemProgramError,
    invoke::{
        address::{derive_new_addresses, insert_addresses_into_address_merkle_tree_queue},
        append_state::insert_output_compressed_accounts_into_state_merkle_tree,
        emit_event::emit_state_transition_event,
        nullify_state::insert_nullifiers,
        sol_compression::compress_or_decompress_lamports,
        sum_check::sum_check,
        verify_proof::{
            hash_input_compressed_accounts, read_address_roots, read_input_state_roots,
            verify_proof, verify_read_only_account_inclusion_by_index,
            verify_read_only_address_queue_non_inclusion,
        },
    },
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        compressed_account::{
            PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
        },
    },
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

/// Inputs:
/// (Note this is a high level overview and not in order.
///  See Steps for checks in implementation order.)
/// 1. Writable Compressed accounts
///     `inputs.input_compressed_accounts_with_merkle_context`
///     1.1. Sum check lamports
///         Check that sum of lamports of in and
///         output compressed accounts add up +- (de)compression.
///     1.2. Compress or decompress lamports
///     1.3. Hash input compressed accounts
///     1.4. Insert Output compressed accounts
///         1.4.1. hash output compressed accounts
///         1.4.2. Validate Tree is writable by signer
///         1.4.3. Check that only existing addresses are used.
///         1.4.4. Enforce that Merkle tree indices are in order
///         1.4.5. Cpi account compression program to insert into output queue or v1 state tree
///     1.5. Insert nullifiers
///         1.5.1. Validate Tree is writable by signer.
///         1.5.2. Cpi account compression program to insert into nullifier queue.
///     1.6. Verify inclusion
///         1.5.1 by index
///         1.5.2 by zkp
/// 2. Read-only compressed accounts
///     `read_only_accounts`
///     - is already hashed we only verify inclusion
///        2.1. Verify inclusion
///         2.1.1 by index
///         2.1.2 by zkp
/// 3. New addresses
///     `inputs.new_address_params`
///    3.1. Derive addresses from seed
///    3.2. Insert addresses into address Merkle tree queue
///    3.3. Verify non-inclusion
/// 4. Read-only addresses
///    `read_only_addresses`
///     4.1. Verify non-inclusion in queue
///     4.2. Verify inclusion by zkp
///
/// Steps:
/// 1. Sum check
///     1.1. Count num_prove_by_index_input_accounts
/// 2. Compression lamports
/// 3. Allocate heap memory
/// 4. Hash input compressed accounts
///     4.1. Collect addresses that exist in input accounts
/// 5. Create new & verify read-only addresses
///     5.1. Verify read only address non-inclusion in bloom filters
///     5.2. Derive new addresses from seed and invoking program
///     5.3. cpi ACP to Insert new addresses into address merkle tree queue
/// 6. Verify read-only account inclusion by index
/// 7. Insert leaves (output compressed account hashes)
///     8.1. Validate Tree is writable by signer
///     8.2. Check that only existing addresses are used.
///     8.3. Enforce that Merkle tree indices are in order
///     8.4. Compute output compressed hashes
///     8.5. cpi ACP to insert output compressed accounts
///         into state Merkle tree v1 or output queue
/// 8. Insert nullifiers (input compressed account hashes)
///     8.1. Create a tx hash
///     8.2. check_program_owner_state_merkle_tree (in sub fn)
///     8.3. Cpi ACP to insert nullifiers
/// 9. Transfer network fee.
/// 10. Read Address and State tree roots
///     - For state roots get roots prior to modifying the tree (for v1 trees).
///     - For v2 and address trees (v1 & v2) the tree isn't modified
///         -> it doesn't matter when we fetch the roots.
///       10.1 Read address roots from accounts
///       10.2 Read state roots from accounts
/// 11. Verify Inclusion & Non-inclusion Proof
///     11.1. Add read only addresses to new addresses vec
///     11.2. filter_for_accounts_not_proven_by_index
/// 12. Emit state transition event
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
    cpi_context_inputs: usize,
    read_only_addresses: Option<Vec<PackedReadOnlyAddress>>,
    read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
) -> Result<()> {
    if inputs.relay_fee.is_some() {
        unimplemented!("Relay fee is not implemented yet.");
    }
    // 1. Sum check ---------------------------------------------------
    bench_sbf_start!("cpda_sum_check");
    let num_prove_by_index_input_accounts = sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee,
        &inputs.compress_or_decompress_lamports,
        &inputs.is_compress,
    )?;
    bench_sbf_end!("cpda_sum_check");
    // 2. Compress or decompress lamports ---------------------------------------------------
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
    let read_only_accounts = read_only_accounts.unwrap_or_default();

    // 3. Allocate heap memory here so that we can free memory after function invocations.
    let num_input_compressed_accounts = inputs.input_compressed_accounts_with_merkle_context.len();
    let num_read_only_accounts = read_only_accounts.len();
    let num_new_addresses = inputs.new_address_params.len();
    let num_output_compressed_accounts = inputs.output_compressed_accounts.len();

    let mut input_compressed_account_hashes = Vec::with_capacity(num_input_compressed_accounts);
    let mut compressed_account_addresses: Vec<Option<[u8; 32]>> =
        vec![None; num_input_compressed_accounts + num_new_addresses];
    let mut output_compressed_account_indices = vec![0u32; num_output_compressed_accounts];
    let mut output_compressed_account_hashes = vec![[0u8; 32]; num_output_compressed_accounts];
    // hashed_pubkeys_capacity is the maximum of hashed pubkey the tx could have.
    // 1 owner pubkey inputs + every remaining account pubkey can be a tree + every output can be owned by a different pubkey
    // + number of times cpi context account was filled.
    let hashed_pubkeys_capacity =
        1 + ctx.remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs;
    let mut hashed_pubkeys = Vec::<(Pubkey, [u8; 32])>::with_capacity(hashed_pubkeys_capacity);

    // 4. hash input compressed accounts ---------------------------------------------------
    // 4.1. collects addresses that exist in input accounts
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
        check_vec_capacity(hashed_pubkeys_capacity, &hashed_pubkeys, "hashed_pubkeys")?;
    }
    bench_sbf_end!("cpda_hash_input_compressed_accounts");

    // 5. Create new & verify read-only addresses ---------------------------------------------------
    let read_only_addresses = read_only_addresses.unwrap_or_default();
    let num_of_read_only_addresses = read_only_addresses.len();
    let num_non_inclusion_proof_inputs = num_new_addresses + num_of_read_only_addresses;
    let mut new_addresses = Vec::with_capacity(num_non_inclusion_proof_inputs);

    // 5.1. Verify read only address non-inclusion in bloom filters
    // Execute prior to inserting new addresses.
    verify_read_only_address_queue_non_inclusion(ctx.remaining_accounts, &read_only_addresses)?;

    let address_network_fee_bundle = if num_new_addresses != 0 {
        // 5.2. Derive new addresses from seed and invoking program
        derive_new_addresses(
            &invoking_program,
            &inputs.new_address_params,
            num_input_compressed_accounts,
            ctx.remaining_accounts,
            &mut compressed_account_addresses,
            &mut new_addresses,
        )?;
        // 5.3. Insert new addresses into address merkle tree queue ---------------------------------------------------
        insert_addresses_into_address_merkle_tree_queue(
            &ctx,
            &new_addresses,
            &inputs.new_address_params,
            &invoking_program,
        )?
    } else {
        None
    };

    // 6. Verify read-only account inclusion by index ---------------------------------------------------
    // Verify prior to creating new state in output queues so that
    // reading an account is successful even when it is modified in the same transaction.
    let num_prove_read_only_accounts_prove_by_index =
        verify_read_only_account_inclusion_by_index(ctx.remaining_accounts, &read_only_accounts)?;

    let num_read_only_accounts_proof =
        num_read_only_accounts - num_prove_read_only_accounts_prove_by_index;
    let num_writable_accounts_proof =
        num_input_compressed_accounts - num_prove_by_index_input_accounts;
    let num_inclusion_proof_inputs = num_writable_accounts_proof + num_read_only_accounts_proof;

    // Allocate space for sequence numbers with remaining account length as a
    // proxy. We cannot allocate heap memory in
    // insert_output_compressed_accounts_into_state_merkle_tree because it is
    // heap neutral.
    let mut sequence_numbers = Vec::with_capacity(ctx.remaining_accounts.len());
    // 7. Insert leaves (output compressed account hashes) ---------------------------------------------------
    let output_network_fee_bundle = if !inputs.output_compressed_accounts.is_empty() {
        bench_sbf_start!("cpda_append");
        let network_fee_bundle = insert_output_compressed_accounts_into_state_merkle_tree(
            &inputs.output_compressed_accounts,
            &ctx,
            &mut output_compressed_account_indices,
            &mut output_compressed_account_hashes,
            &mut compressed_account_addresses,
            &invoking_program,
            &mut hashed_pubkeys,
            &mut sequence_numbers,
        )?;
        // # Safety this is a safeguard for memory safety.
        // This error should never be triggered.
        check_vec_capacity(hashed_pubkeys_capacity, &hashed_pubkeys, "hashed_pubkeys")?;
        bench_sbf_end!("cpda_append");
        network_fee_bundle
    } else {
        None
    };
    bench_sbf_start!("emit_state_transition_event");
    // Reduce the capacity of the sequence numbers vector.
    sequence_numbers.shrink_to_fit();

    // 8. insert nullifiers (input compressed account hashes)---------------------------------------------------
    // Note: It would make sense to nullify prior to appending new state.
    //      Since output compressed account hashes are inputs
    //      for the tx hash on which the nullifier depends
    //      and the logic to compute output hashes is higly optimized
    //      and entangled with the cpi we leave it as is for now.
    bench_sbf_start!("cpda_nullifiers");
    let input_network_fee_bundle = if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        let current_slot = Clock::get()?.slot;
        // 8.1. Create a tx hash
        let tx_hash = create_tx_hash(
            &input_compressed_account_hashes,
            &output_compressed_account_hashes,
            current_slot,
        )
        .map_err(ProgramError::from)?;
        // 8.2. Insert nullifiers for compressed input account hashes into nullifier
        // queue.
        insert_nullifiers(
            &inputs.input_compressed_accounts_with_merkle_context,
            &ctx,
            &input_compressed_account_hashes,
            &invoking_program,
            tx_hash,
        )?
    } else {
        None
    };
    bench_sbf_end!("cpda_nullifiers");

    // 9. Transfer network fee
    transfer_network_fee(
        &ctx,
        input_network_fee_bundle,
        address_network_fee_bundle,
        output_network_fee_bundle,
    )?;

    // 10. Read Address and State tree roots ---------------------------------------------------
    let mut new_address_roots = Vec::with_capacity(num_non_inclusion_proof_inputs);
    // 10.1 Read address roots ---------------------------------------------------
    let address_tree_height = read_address_roots(
        ctx.remaining_accounts,
        &inputs.new_address_params,
        &read_only_addresses,
        &mut new_address_roots,
    )?;
    // # Safety this is a safeguard for memory safety.
    // This error should never be triggered.
    check_vec_capacity(
        num_non_inclusion_proof_inputs,
        &new_address_roots,
        "new_address_roots",
    )?;
    // 10.2. Read state roots ---------------------------------------------------
    let mut input_compressed_account_roots = Vec::with_capacity(num_inclusion_proof_inputs);
    let state_tree_height = read_input_state_roots(
        ctx.remaining_accounts,
        &inputs.input_compressed_accounts_with_merkle_context,
        &read_only_accounts,
        &mut input_compressed_account_roots,
    )?;
    // # Safety this is a safeguard for memory safety.
    // This error should never be triggered.
    check_vec_capacity(
        num_inclusion_proof_inputs,
        &input_compressed_account_roots,
        "input_compressed_account_roots",
    )?;

    // 11. Verify Inclusion & Non-inclusion Proof ---------------------------------------------------
    if num_inclusion_proof_inputs != 0 || num_non_inclusion_proof_inputs != 0 {
        if let Some(proof) = inputs.proof.as_ref() {
            bench_sbf_start!("cpda_verify_state_proof");

            // 11.1. Add read only addresses to new addresses vec before proof verification.
            // We don't add read only addresses before since
            // read-only addresses must not be used in output compressed accounts.
            for read_only_address in read_only_addresses.iter() {
                new_addresses.push(read_only_address.address);
            }

            // 11.2. Select accounts account hashes for ZKP.
            // We need to filter out accounts that are proven by index.
            let mut proof_input_compressed_account_hashes =
                Vec::with_capacity(num_inclusion_proof_inputs);
            filter_for_accounts_not_proven_by_index(
                &inputs.input_compressed_accounts_with_merkle_context,
                &read_only_accounts,
                &input_compressed_account_hashes,
                &mut proof_input_compressed_account_hashes,
            );
            check_vec_capacity(
                num_inclusion_proof_inputs,
                &proof_input_compressed_account_hashes,
                "proof_input_compressed_account_hashes",
            )?;

            let compressed_proof = CompressedVerifierProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            };
            // 11.3. Verify proof
            // Proof inputs order:
            // 1. input compressed accounts
            // 2. read-only compressed accounts
            // 3. new addresses
            // 4. read-only addresses
            match verify_proof(
                &input_compressed_account_roots,
                &proof_input_compressed_account_hashes,
                &new_address_roots,
                &new_addresses,
                &compressed_proof,
                address_tree_height,
                state_tree_height,
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    msg!("proof  {:?}", proof);
                    msg!(
                        "proof_input_compressed_account_hashes {:?}",
                        proof_input_compressed_account_hashes
                    );
                    msg!("input roots {:?}", input_compressed_account_roots);
                    msg!("read_only_accounts {:?}", read_only_accounts);
                    msg!(
                        "input_compressed_accounts_with_merkle_context: {:?}",
                        inputs.input_compressed_accounts_with_merkle_context
                    );
                    msg!("new_address_roots {:?}", new_address_roots);
                    msg!("new_addresses {:?}", new_addresses);
                    msg!("read_only_addresses {:?}", read_only_addresses);
                    Err(e)
                }
            }?;
            bench_sbf_end!("cpda_verify_state_proof");
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
        && read_only_accounts.is_empty()
        && read_only_addresses.is_empty()
    {
        return err!(SystemProgramError::EmptyInputs);
    }

    // 12. Emit state transition event ---------------------------------------------------
    bench_sbf_start!("emit_state_transition_event");
    emit_state_transition_event(
        inputs,
        &ctx,
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_compressed_account_indices,
        sequence_numbers,
    )?;
    bench_sbf_end!("emit_state_transition_event");

    Ok(())
}

#[inline(always)]
fn check_vec_capacity<T>(expected_capacity: usize, vec: &Vec<T>, vec_name: &str) -> Result<()> {
    if vec.capacity() != expected_capacity {
        msg!(
            "{} exceeded capacity. Used {}, allocated {}.",
            vec_name,
            vec.capacity(),
            expected_capacity
        );
        return err!(SystemProgramError::InvalidCapacity);
    }
    Ok(())
}

#[inline(always)]
fn filter_for_accounts_not_proven_by_index(
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    read_only_accounts: &[PackedReadOnlyCompressedAccount],
    input_compressed_account_hashes: &[[u8; 32]],
    proof_input_compressed_account_hashes: &mut Vec<[u8; 32]>,
) {
    for (hash, input_account) in input_compressed_account_hashes
        .iter()
        .zip(input_compressed_accounts_with_merkle_context.iter())
    {
        if !input_account.merkle_context.queue_index {
            proof_input_compressed_account_hashes.push(*hash);
        }
    }
    for read_only_account in read_only_accounts.iter() {
        // only push read only account hashes which are not marked as proof by index
        if !read_only_account.merkle_context.queue_index {
            proof_input_compressed_account_hashes.push(read_only_account.account_hash);
        }
    }
}

/// Network fee distribution:
/// - if any account is created or modified -> transfer network fee (5000 lamports)
///   (Previously we didn't charge for appends now we have to since values go into a queue.)
/// - if an address is created -> transfer an additional network fee (5000 lamports)
///
/// Examples:
/// 1. create account with address    network fee 10,000 lamports
/// 2. token transfer                 network fee 5,000 lamports
/// 3. mint token                     network fee 5,000 lamports
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
