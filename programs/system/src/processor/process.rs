use std::cmp::min;

use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof,
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, InsertNullifierInput},
        traits::InstructionDataTrait,
        zero_copy::ZPackedReadOnlyCompressedAccount,
    },
    tx_hash::create_tx_hash_from_hash_chains,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::slice_mut::ZeroCopySliceMut;
use pinocchio::{
    account_info::AccountInfo, log::sol_log_compute_units, msg, program_error::ProgramError,
    pubkey::Pubkey, sysvars::clock::Clock,
};

#[cfg(feature = "readonly")]
use crate::processor::{
    read_only_account::verify_read_only_account_inclusion_by_index,
    read_only_address::verify_read_only_address_queue_non_inclusion,
};
use crate::{
    accounts::account_traits::{InvokeAccounts, SignerAccounts},
    accounts::check_accounts::try_from_account_infos,
    constants::CPI_AUTHORITY_PDA_BUMP,
    context::WrappedInstructionData,
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
    Result,
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
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
    T: InstructionDataTrait<'a>,
>(
    inputs: WrappedInstructionData<'a, T>,
    invoking_program: Option<Pubkey>,
    ctx: &A,
    cpi_context_inputs: usize,
    remaining_accounts: &'info [AccountInfo],
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_process_compression");
    let num_input_compressed_accounts = inputs.input_len();
    let num_new_addresses = inputs.address_len();
    let num_output_compressed_accounts = inputs.output_len();
    // msg!("num new addresses: {}", num_new_addresses);
    // hashed_pubkeys_capacity is the maximum of hashed pubkey the tx could have.
    // 1 owner pubkey inputs + every remaining account pubkey can be a tree + every output can be owned by a different pubkey
    // + number of times cpi context account was filled.
    let hashed_pubkeys_capacity =
        1 + remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs;

    // 1. Allocate cpi data and initialize context
    let (mut context, mut cpi_ix_bytes) = create_cpi_data_and_context(
        ctx,
        num_output_compressed_accounts as u8,
        num_input_compressed_accounts as u8,
        num_new_addresses as u8,
        hashed_pubkeys_capacity,
        invoking_program,
        remaining_accounts,
    )?;
    //     msg!("processor: post create_cpi_data_and_context");
    // Collect all addresses to check that every address in the output compressed accounts
    // is an input or a new address.
    inputs.input_accounts().for_each(|account| {
        context.addresses.push(account.address());
    });

    // 2. Deserialize and check all Merkle tree and queue accounts.
    #[allow(unused_mut)]
    let mut accounts = try_from_account_infos(remaining_accounts, &mut context)?;
    // 3. Deserialize cpi instruction data as zero copy to fill it.
    let mut cpi_ix_data = InsertIntoQueuesInstructionDataMut::new(
        &mut cpi_ix_bytes[12..],
        num_output_compressed_accounts as u8,
        num_input_compressed_accounts as u8,
        num_new_addresses as u8,
        min(remaining_accounts.len(), num_output_compressed_accounts) as u8,
        min(remaining_accounts.len(), num_input_compressed_accounts) as u8,
        min(remaining_accounts.len(), num_new_addresses) as u8,
    )
    .map_err(ProgramError::from)?;
    cpi_ix_data.set_invoked_by_program(true);
    cpi_ix_data.bump = CPI_AUTHORITY_PDA_BUMP;

    // 4. Create new & verify read-only addresses ---------------------------------------------------
    let read_only_addresses = inputs.read_only_addresses().unwrap_or_default();
    let num_of_read_only_addresses = read_only_addresses.len();
    let num_non_inclusion_proof_inputs = num_new_addresses + num_of_read_only_addresses;

    let mut new_address_roots = Vec::with_capacity(num_non_inclusion_proof_inputs);
    // 5. Read address roots ---------------------------------------------------
    let address_tree_height = read_address_roots(
        accounts.as_slice(),
        inputs.new_addresses(),
        read_only_addresses,
        &mut new_address_roots,
    )?;
    //     msg!("processor: post  Read address roots");

    // 6. Derive new addresses from seed and invoking program
    if num_new_addresses != 0 {
        derive_new_addresses(
            inputs.new_addresses(),
            remaining_accounts,
            &mut context,
            &mut cpi_ix_data,
            accounts.as_slice(),
        )?
    }

    // 7. Verify read only address non-inclusion in bloom filters
    #[cfg(feature = "readonly")]
    verify_read_only_address_queue_non_inclusion(
        accounts.as_mut_slice(),
        inputs.read_only_addresses().unwrap_or_default(),
    )?;
    #[cfg(not(feature = "readonly"))]
    if !read_only_addresses.is_empty() {
        unimplemented!("Read only addresses are not supported in this build.")
    }

    // 8. Insert leaves (output compressed account hashes) ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_append");
    // 9. Create cpi data for outputs
    //      9.1. Compute output compressed hashes
    //      9.2. Collect accounts
    //      9.3. Validate order of output queue/ tree accounts
    let output_compressed_account_hashes = create_outputs_cpi_data::<T>(
        &inputs,
        remaining_accounts,
        &mut context,
        &mut cpi_ix_data,
        accounts.as_slice(),
    )?;
    #[cfg(feature = "debug")]
    check_vec_capacity(
        hashed_pubkeys_capacity,
        &context.hashed_pubkeys,
        "hashed_pubkeys",
    )?;
    //     msg!("processor: post  output_compressed_account_hashes");

    // 10. hash input compressed accounts ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_nullifiers");
    if !inputs.inputs_empty() {
        // currently must be post output accounts since the order of account infos matters
        // for the outputs.
        let input_compressed_account_hashes = create_inputs_cpi_data(
            remaining_accounts,
            &inputs,
            &mut context,
            &mut cpi_ix_data,
            accounts.as_slice(),
        )?;

        #[cfg(feature = "debug")]
        check_vec_capacity(
            hashed_pubkeys_capacity,
            &context.hashed_pubkeys,
            "hashed_pubkeys",
        )?;
        // 8.1. Create a tx hash
        use pinocchio::sysvars::Sysvar;
        let current_slot = Clock::get()?.slot;
        cpi_ix_data.tx_hash = create_tx_hash_from_hash_chains(
            &input_compressed_account_hashes,
            &output_compressed_account_hashes,
            current_slot,
        )
        .map_err(ProgramError::from)?;
    }
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_nullifiers");
    //     msg!("processor: post  input_compressed_account_hashes");

    // 11. Sum check ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_sum_check");
    let num_prove_by_index_input_accounts = sum_check(&inputs, &None, &inputs.is_compress())?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_sum_check");
    // 12. Compress or decompress lamports ---------------------------------------------------
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_process_compression");
    if inputs.compress_or_decompress_lamports().is_some() {
        msg!(format!(
            "inputs.is_compress() {:?} \n,  ctx.get_decompression_recipient().is_some()  {:?}, \n inputs.compress_or_decompress_lamports() {:?}",
            inputs.is_compress(),
            ctx.get_decompression_recipient().is_some(),
            inputs.compress_or_decompress_lamports()
        )
        .as_str());
        if inputs.is_compress() && ctx.get_decompression_recipient().is_some() {
            return Err(SystemProgramError::DecompressionRecipientDefined.into());
        }
        compress_or_decompress_lamports(
            inputs.is_compress(),
            inputs.compress_or_decompress_lamports(),
            ctx,
        )?;
    } else if ctx.get_decompression_recipient().is_some() {
        return Err(SystemProgramError::DecompressionRecipientDefined.into());
    } else if ctx.get_sol_pool_pda().is_some() {
        return Err(SystemProgramError::SolPoolPdaDefined.into());
    }
    //     msg!("processor: post  compress_or_decompress_lamports");

    // 13. Verify read-only account inclusion by index ---------------------------------------------------
    let read_only_accounts = inputs.read_only_accounts().unwrap_or_default();

    #[cfg(feature = "readonly")]
    let num_prove_read_only_accounts_prove_by_index =
        verify_read_only_account_inclusion_by_index(accounts.as_mut_slice(), read_only_accounts)?;
    #[cfg(not(feature = "readonly"))]
    let num_prove_read_only_accounts_prove_by_index = 0;
    #[cfg(not(feature = "readonly"))]
    if !read_only_addresses.is_empty() {
        unimplemented!("Read only addresses are not supported in this build.")
    }

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
    // 14. Read state roots ---------------------------------------------------
    let mut input_compressed_account_roots = Vec::with_capacity(num_inclusion_proof_inputs);
    let state_tree_height = read_input_state_roots(
        accounts.as_slice(),
        inputs.input_accounts(),
        read_only_accounts,
        &mut input_compressed_account_roots,
    )?;
    //     msg!("processor: post  Read state roots ");

    #[cfg(feature = "debug")]
    check_vec_capacity(
        num_inclusion_proof_inputs,
        &input_compressed_account_roots,
        "input_compressed_account_roots",
    )?;

    // 15. Verify Inclusion & Non-inclusion Proof ---------------------------------------------------
    if num_inclusion_proof_inputs != 0 || num_non_inclusion_proof_inputs != 0 {
        if let Some(proof) = inputs.proof().as_ref() {
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
                read_only_accounts,
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
                    // msg!("proof  {:?}", proof);
                    // msg!(
                    //     "proof_input_compressed_account_hashes {:?}",
                    //     proof_input_compressed_account_hashes
                    // );
                    // msg!("input roots {:?}", input_compressed_account_roots);
                    // msg!("read_only_accounts {:?}", read_only_accounts);
                    // msg!(
                    //     "input_compressed_accounts_with_merkle_context: {:?}",
                    //     inputs.input_compressed_accounts_with_merkle_context
                    // );
                    // msg!("new_address_roots {:?}", new_address_roots);
                    // msg!("new_addresses {:?}", new_addresses);
                    // msg!("read_only_addresses {:?}", read_only_addresses);
                    Err(e)
                }
            }?;
            #[cfg(feature = "bench-sbf")]
            bench_sbf_end!("cpda_verify_state_proof");
        } else {
            return Err(SystemProgramError::ProofIsNone.into());
        }
    } else if inputs.proof().is_some() {
        return Err(SystemProgramError::ProofIsSome.into());
    } else if inputs.inputs_empty()
        && inputs.address_empty()
        && inputs.outputs_empty()
        && read_only_accounts.is_empty()
        && read_only_addresses.is_empty()
    {
        return Err(SystemProgramError::EmptyInputs.into());
    }

    // 16. Transfer network, address, and rollover fees ---------------------------------------------------
    //      Note: we transfer rollover fees from the system program instead
    //      of the account compression program to reduce cpi depth.
    context.transfer_fees(remaining_accounts, ctx.get_fee_payer())?;
    //     msg!("processor: post  transfer_fees ");

    // No elements are to be inserted into the queue.
    // -> tx only contains read only accounts.
    if inputs.inputs_empty() && inputs.address_empty() && inputs.outputs_empty() {
        return Ok(());
    }
    sol_log_compute_units();
    // 17. CPI account compression program ---------------------------------------------------

    // msg!("start_acp_cpi");
    // sol_log_compute_units();

    cpi_account_compression_program(context, cpi_ix_bytes)?;
    // sol_log_compute_units();
    // msg!("end_acp_cpi");
    Ok(())
}

#[cfg(feature = "debug")]
#[inline(always)]
fn check_vec_capacity<T>(expected_capacity: usize, vec: &Vec<T>, _vec_name: &str) -> Result<()> {
    if vec.capacity() != expected_capacity {
        // msg!(
        //     "{} exceeded capacity. Used {}, allocated {}.",
        //     _vec_name,
        //     vec.capacity(),
        //     expected_capacity
        // );
        return Err(SystemProgramError::InvalidCapacity.into());
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
