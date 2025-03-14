use std::cmp::min;

use anchor_lang::{prelude::*, solana_program::log::sol_log_compute_units, Bumps};
use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof,
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, InsertNullifierInput},
        zero_copy::{
            ZInstructionDataInvoke, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
        },
    },
    tx_hash::create_tx_hash_from_hash_chains,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{slice::ZeroCopySliceBorsh, slice_mut::ZeroCopySliceMut};

#[cfg(feature = "readonly")]
use crate::processor::{
    read_only_account::verify_read_only_account_inclusion_by_index,
    read_only_address::verify_read_only_address_queue_non_inclusion,
};
use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    check_accounts::try_from_account_infos,
    constants::CPI_AUTHORITY_PDA_BUMP,
    errors::SystemProgramError,
    processor::{
        cpi::{cpi_account_compression_program, create_cpi_data_and_context},
        create_address_cpi_data::derive_new_addresses,
        create_inputs_cpi_data::create_inputs_cpi_data,
        create_outputs_cpi_data::create_outputs_cpi_data,
        sol_compression::compress_or_decompress_lamports,
        sum_check::sum_check,
        verify_proof::{read_address_roots, read_input_state_roots, verify_proof},
    },
};

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
///     1.5. Insert nullifiers
///         1.5.1. Validate Tree is writable by signer.
///     1.6. Verify inclusion
///         1.5.1 by index
///         1.5.2 by zkp
///     1.7. Cpi account compression program to insert new addresses,
///         nullify input and append output state.
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
pub fn process<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: ZInstructionDataInvoke<'a>,
    invoking_program: Option<Pubkey>,
    ctx: Context<'a, 'b, 'c, 'info, A>,
    cpi_context_inputs: usize,
    read_only_addresses: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>>,
    read_only_accounts: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>>,
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_process_compression");
    let num_input_compressed_accounts = inputs.input_compressed_accounts_with_merkle_context.len();
    let num_new_addresses = inputs.new_address_params.len();
    let num_output_compressed_accounts = inputs.output_compressed_accounts.len();
    // msg!("num new addresses: {}", num_new_addresses);
    // hashed_pubkeys_capacity is the maximum of hashed pubkey the tx could have.
    // 1 owner pubkey inputs + every remaining account pubkey can be a tree + every output can be owned by a different pubkey
    // + number of times cpi context account was filled.
    let hashed_pubkeys_capacity =
        1 + ctx.remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs;

    // 1. Allocate cpi data and initialize context
    let (mut context, mut cpi_ix_bytes) = create_cpi_data_and_context(
        &ctx,
        num_output_compressed_accounts as u8,
        num_input_compressed_accounts as u8,
        num_new_addresses as u8,
        hashed_pubkeys_capacity,
        invoking_program,
    )?;
    msg!("context init done");
    sol_log_compute_units();
    // Collect all addresses to check that every address in the output compressed accounts
    // is an input or a new address.
    inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .for_each(|account| {
            if let Some(address) = account.compressed_account.address {
                context.addresses.push(Some(*address));
            }
        });

    msg!("context.addresses done");
    sol_log_compute_units();
    // 2. Deserialize and check all Merkle tree and queue accounts.
    #[allow(unused_mut)]
    let mut accounts = try_from_account_infos(ctx.remaining_accounts, &mut context)?;
    msg!("accounts done");
    sol_log_compute_units();
    // 3. Deserialize cpi instruction data as zero copy to fill it.
    let mut cpi_ix_data = InsertIntoQueuesInstructionDataMut::new(
        &mut cpi_ix_bytes,
        num_output_compressed_accounts as u8,
        num_input_compressed_accounts as u8,
        num_new_addresses as u8,
        min(ctx.remaining_accounts.len(), num_output_compressed_accounts) as u8,
        min(ctx.remaining_accounts.len(), num_input_compressed_accounts) as u8,
        min(ctx.remaining_accounts.len(), num_new_addresses) as u8,
    )
    .map_err(ProgramError::from)?;
    msg!("cpi_ix_data done");
    sol_log_compute_units();
    cpi_ix_data.set_invoked_by_program(true);
    cpi_ix_data.bump = CPI_AUTHORITY_PDA_BUMP;

    // 4. Create new & verify read-only addresses ---------------------------------------------------
    let read_only_addresses =
        read_only_addresses.unwrap_or(ZeroCopySliceBorsh::from_bytes(&[0, 0, 0, 0]).unwrap());
    let num_of_read_only_addresses = read_only_addresses.len();
    let num_non_inclusion_proof_inputs = num_new_addresses + num_of_read_only_addresses;

    msg!("read_only_addresses allocation done");
    sol_log_compute_units();

    let mut new_address_roots = Vec::with_capacity(num_non_inclusion_proof_inputs);
    // 5. Read address roots ---------------------------------------------------
    let address_tree_height = read_address_roots(
        &accounts,
        inputs.new_address_params.as_slice(),
        read_only_addresses.as_slice(),
        &mut new_address_roots,
    )?;

    msg!("read_address_roots done");
    sol_log_compute_units();

    // 6. Derive new addresses from seed and invoking program
    if num_new_addresses != 0 {
        derive_new_addresses(
            inputs.new_address_params.as_slice(),
            ctx.remaining_accounts,
            &mut context,
            &mut cpi_ix_data,
            &accounts,
        )?
    }

    msg!("derive_new_addresses done");
    sol_log_compute_units();

    // 7. Verify read only address non-inclusion in bloom filters
    #[cfg(feature = "readonly")]
    verify_read_only_address_queue_non_inclusion(&mut accounts, read_only_addresses.as_slice())?;
    #[cfg(not(feature = "readonly"))]
    if !read_only_addresses.is_empty() {
        unimplemented!("Read only addresses are not supported in this build.")
    }

    msg!("verify_read_only_address_queue_non_inclusion done");
    sol_log_compute_units();

    // 8. Insert leaves (output compressed account hashes) ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_append");
    // 9. Create cpi data for outputs
    //      9.1. Compute output compressed hashes
    //      9.2. Collect accounts
    //      9.3. Validate order of output queue/ tree accounts
    let output_compressed_account_hashes = create_outputs_cpi_data(
        inputs.output_compressed_accounts.as_slice(),
        ctx.remaining_accounts,
        &mut context,
        &mut cpi_ix_data,
        &accounts,
    )?;

    msg!("create_outputs_cpi_data done");
    sol_log_compute_units();

    #[cfg(feature = "debug")]
    check_vec_capacity(
        hashed_pubkeys_capacity,
        &context.hashed_pubkeys,
        "hashed_pubkeys",
    )?;

    msg!("check_vec_capacity #0 done");
    sol_log_compute_units();

    // 10. hash input compressed accounts ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_nullifiers");
    if !inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        // currently must be post output accounts since the order of account infos matters
        // for the outputs.
        let input_compressed_account_hashes = create_inputs_cpi_data(
            ctx.remaining_accounts,
            inputs
                .input_compressed_accounts_with_merkle_context
                .as_slice(),
            &mut context,
            &mut cpi_ix_data,
            &accounts,
        )?;
        msg!("create_inputs_cpi_data done");
        sol_log_compute_units();

        #[cfg(feature = "debug")]
        check_vec_capacity(
            hashed_pubkeys_capacity,
            &context.hashed_pubkeys,
            "hashed_pubkeys",
        )?;
        msg!("check_vec_capacity #1 done");
        sol_log_compute_units();
        // 8.1. Create a tx hash
        let current_slot = Clock::get()?.slot;
        cpi_ix_data.tx_hash = create_tx_hash_from_hash_chains(
            &input_compressed_account_hashes,
            &output_compressed_account_hashes,
            current_slot,
        )
        .map_err(ProgramError::from)?;
        msg!("create_tx_hash_from_hash_chains done");
        sol_log_compute_units();
    }
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_nullifiers");

    // 11. Sum check ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_sum_check");
    let num_prove_by_index_input_accounts = sum_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
        &inputs.relay_fee.map(|x| (*x).into()),
        &inputs.compress_or_decompress_lamports.map(|x| (*x).into()),
        &inputs.is_compress,
    )?;
    msg!("sum_check done");
    sol_log_compute_units();
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_sum_check");
    // 12. Compress or decompress lamports ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
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

    msg!("compress_or_decompress_lamports done");
    sol_log_compute_units();

    // 13. Verify read-only account inclusion by index ---------------------------------------------------
    let read_only_accounts = read_only_accounts.unwrap_or_else(|| {
        ZeroCopySliceBorsh::<ZPackedReadOnlyCompressedAccount>::from_bytes(&[0u8, 0u8, 0u8, 0u8])
            .unwrap()
    });

    msg!("read_only_accounts allocation done");
    sol_log_compute_units();

    #[cfg(feature = "readonly")]
    let num_prove_read_only_accounts_prove_by_index =
        verify_read_only_account_inclusion_by_index(&mut accounts, read_only_accounts.as_slice())?;
    #[cfg(not(feature = "readonly"))]
    let num_prove_read_only_accounts_prove_by_index = 0;
    #[cfg(not(feature = "readonly"))]
    if !read_only_addresses.is_empty() {
        unimplemented!("Read only addresses are not supported in this build.")
    }

    msg!("verify_read_only_account_inclusion_by_index done");
    sol_log_compute_units();

    let num_read_only_accounts = read_only_accounts.len();
    let num_read_only_accounts_proof =
        num_read_only_accounts - num_prove_read_only_accounts_prove_by_index;
    let num_writable_accounts_proof =
        num_input_compressed_accounts - num_prove_by_index_input_accounts;
    let num_inclusion_proof_inputs = num_writable_accounts_proof + num_read_only_accounts_proof;

    #[cfg(feature = "debug")]
    check_vec_capacity(
        num_non_inclusion_proof_inputs,
        &new_address_roots,
        "new_address_roots",
    )?;

    msg!("allocs and check_vec_capacity #2 done");
    sol_log_compute_units();

    // 14. Read state roots ---------------------------------------------------
    let mut input_compressed_account_roots = Vec::with_capacity(num_inclusion_proof_inputs);
    let state_tree_height = read_input_state_roots(
        &accounts,
        inputs
            .input_compressed_accounts_with_merkle_context
            .as_slice(),
        read_only_accounts.as_slice(),
        &mut input_compressed_account_roots,
    )?;

    msg!("read_input_state_roots done");
    sol_log_compute_units();

    #[cfg(feature = "debug")]
    check_vec_capacity(
        num_inclusion_proof_inputs,
        &input_compressed_account_roots,
        "input_compressed_account_roots",
    )?;

    msg!("check_vec_capacity #3 done");
    sol_log_compute_units();

    // 15. Verify Inclusion & Non-inclusion Proof ---------------------------------------------------
    if num_inclusion_proof_inputs != 0 || num_non_inclusion_proof_inputs != 0 {
        if let Some(proof) = inputs.proof.as_ref() {
            #[cfg(feature = "bench-sbf")]
            bench_sbf_start!("cpda_verify_state_proof");
            let mut new_addresses = Vec::with_capacity(num_non_inclusion_proof_inputs);
            // 15.1. Copy the new addresses to new addresses vec
            //      (Remove and compute hash chain directly once concurrent trees are phased out.)
            for new_address in cpi_ix_data.addresses.iter() {
                new_addresses.push(new_address.address);
            }
            // 15.1. Add read only addresses to new addresses vec before proof verification.
            // We don't add read only addresses before since
            // read-only addresses must not be used in output compressed accounts.
            for read_only_address in read_only_addresses.iter() {
                new_addresses.push(read_only_address.address);
            }

            // 15.2. Select accounts account hashes for ZKP.
            // We need to filter out accounts that are proven by index.
            let mut proof_input_compressed_account_hashes =
                Vec::with_capacity(num_inclusion_proof_inputs);
            filter_for_accounts_not_proven_by_index(
                read_only_accounts.as_slice(),
                &cpi_ix_data.nullifiers,
                &mut proof_input_compressed_account_hashes,
            );
            #[cfg(feature = "debug")]
            check_vec_capacity(
                num_inclusion_proof_inputs,
                &proof_input_compressed_account_hashes,
                "proof_input_compressed_account_hashes",
            )?;

            let compressed_proof = CompressedProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            };
            // 15.3. Verify proof
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
            #[cfg(feature = "bench-sbf")]
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

    msg!("provebyindex done");
    sol_log_compute_units();

    // 16. Transfer network, address, and rollover fees ---------------------------------------------------
    //      Note: we transfer rollover fees from the system program instead
    //      of the account compression program to reduce cpi depth.
    context.transfer_fees(ctx.remaining_accounts, ctx.accounts.get_fee_payer())?;
    msg!("transfer_fees done");
    sol_log_compute_units();
    // No elements are to be inserted into the queue.
    // -> tx only contains read only accounts.
    if inputs
        .input_compressed_accounts_with_merkle_context
        .is_empty()
        && inputs.new_address_params.is_empty()
        && inputs.output_compressed_accounts.is_empty()
    {
        return Ok(());
    }
    // 17. CPI account compression program ---------------------------------------------------
    cpi_account_compression_program(context, cpi_ix_bytes)
}

#[cfg(feature = "debug")]
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
    read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
    input_compressed_account_hashes: &ZeroCopySliceMut<'_, u8, InsertNullifierInput, false>,
    proof_input_compressed_account_hashes: &mut Vec<[u8; 32]>,
) {
    for input_account in input_compressed_account_hashes.iter() {
        if !input_account.prove_by_index() {
            proof_input_compressed_account_hashes.push(input_account.account_hash);
        }
    }
    for read_only_account in read_only_accounts.iter() {
        // only push read only account hashes which are not marked as proof by index
        if !read_only_account.merkle_context.prove_by_index() {
            proof_input_compressed_account_hashes.push(read_only_account.account_hash);
        }
    }
}
