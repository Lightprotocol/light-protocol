use std::cmp::min;

use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof,
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, InsertNullifierInput},
        traits::InstructionData,
        zero_copy::ZPackedReadOnlyCompressedAccount,
    },
    tx_hash::create_tx_hash_from_hash_chains,
};
use light_program_profiler::profile;
use light_zero_copy::slice_mut::ZeroCopySliceMut;
use pinocchio::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvars::clock::Clock,
};

use crate::{
    accounts::{
        account_traits::{InvokeAccounts, SignerAccounts},
        remaining_account_checks::try_from_account_infos,
    },
    constants::CPI_AUTHORITY_PDA_BUMP,
    context::WrappedInstructionData,
    cpi_context::process_cpi_context::copy_cpi_context_outputs,
    errors::SystemProgramError,
    processor::{
        cpi::{cpi_account_compression_program, create_cpi_data_and_context},
        create_address_cpi_data::derive_new_addresses,
        create_inputs_cpi_data::create_inputs_cpi_data,
        create_outputs_cpi_data::{check_new_address_assignment, create_outputs_cpi_data},
        read_only_account::verify_read_only_account_inclusion_by_index,
        read_only_address::verify_read_only_address_queue_non_inclusion,
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
///    `inputs.input_compressed_accounts_with_merkle_context`
///    1.1. Sum check lamports
///    Check that sum of lamports of in and
///    output compressed accounts add up +- (de)compression.
///    1.2. Compress or decompress lamports
///    1.3. Hash input compressed accounts
///    1.4. Insert Output compressed accounts
///    1.4.1. hash output compressed accounts
///    1.4.2. Validate Tree is writable by signer
///    1.4.3. Check that only existing addresses are used.
///    1.4.4. Enforce that Merkle tree indices are in order
///    1.5. Insert nullifiers
///    1.5.1. Validate Tree is writable by signer.
///    1.6. Verify inclusion
///    1.6.1 by index
///    1.6.2 by zkp
///    1.7. Cpi account compression program to insert new addresses,
///    nullify input and append output state.
/// 2. Read-only compressed accounts
///    `read_only_accounts`
///     - is already hashed we only verify inclusion
///       2.1. Verify inclusion
///       2.1.1 by index
///       2.1.2 by zkp
/// 3. New addresses
///    `inputs.new_address_params`
///    3.1. Derive addresses from seed
///    3.2. Insert addresses into address Merkle tree queue
///    3.3. Verify non-inclusion
/// 4. Read-only addresses
///    `read_only_addresses`
///    4.1. Verify non-inclusion in queue
///    4.2. Verify inclusion by zkp
#[profile]
pub fn process<
    'a,
    'info,
    const ADDRESS_ASSIGNMENT: bool,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
    T: InstructionData<'a>,
>(
    inputs: WrappedInstructionData<'a, T>,
    invoking_program: Option<Pubkey>,
    ctx: &A,
    cpi_context_inputs_len: usize,
    remaining_accounts: &'info [AccountInfo],
) -> Result<()> {
    let num_input_accounts = inputs.input_len();
    let num_new_addresses = inputs.address_len();
    let num_output_compressed_accounts = inputs.output_len();

    // hashed_pubkeys_capacity is the maximum of hashed pubkey the tx could have.
    // 1 owner pubkey inputs + every remaining account pubkey
    // can be a tree + every output can be owned by a different pubkey
    // + number of times cpi context account was filled.
    let hashed_pubkeys_capacity =
        1 + remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs_len;

    let cpi_outputs_data_len =
        inputs.get_cpi_context_outputs_end_offset() - inputs.get_cpi_context_outputs_start_offset();
    // 1. Allocate cpi data and initialize context
    let (mut context, mut cpi_ix_bytes) = create_cpi_data_and_context(
        ctx,
        num_output_compressed_accounts as u8,
        num_input_accounts as u8,
        num_new_addresses as u8,
        hashed_pubkeys_capacity,
        cpi_outputs_data_len,
        invoking_program,
        remaining_accounts,
    )?;

    // 2. Deserialize and check all Merkle tree and queue accounts.
    let mut accounts = try_from_account_infos(remaining_accounts, &mut context)?;
    // 3. Deserialize cpi instruction data as zero copy to fill it.
    let (mut cpi_ix_data, bytes) = InsertIntoQueuesInstructionDataMut::new_at(
        &mut cpi_ix_bytes[12..], // 8 bytes instruction discriminator + 4 bytes vector length
        num_output_compressed_accounts as u8,
        num_input_accounts as u8,
        num_new_addresses as u8,
        min(remaining_accounts.len(), num_output_compressed_accounts) as u8,
        min(remaining_accounts.len(), num_input_accounts) as u8,
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

    // 6. Collect all addresses to check that every address in the output compressed accounts
    // is an existing address or a new address.
    inputs.input_accounts().for_each(|account| {
        context.addresses.push(account.address());
    });

    // 7. Derive new addresses from seed and invoking program
    if num_new_addresses != 0 {
        derive_new_addresses::<ADDRESS_ASSIGNMENT>(
            inputs.new_addresses(),
            remaining_accounts,
            &mut context,
            &mut cpi_ix_data,
            accounts.as_slice(),
        )?;

        if ADDRESS_ASSIGNMENT {
            check_new_address_assignment(&inputs, &cpi_ix_data)?;
        } else if inputs
            .new_addresses()
            .any(|x| x.assigned_compressed_account_index().is_some())
        {
            return Err(SystemProgramError::InvalidAddress.into());
        }
    }

    // 7. Verify read only address non-inclusion in bloom filters
    verify_read_only_address_queue_non_inclusion(
        accounts.as_mut_slice(),
        inputs.read_only_addresses().unwrap_or_default(),
    )?;

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
    let read_only_accounts = inputs.read_only_accounts().unwrap_or_default();

    // 10. hash input compressed accounts ---------------------------------------------------
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

        // 10.1. Create a tx hash
        if inputs.with_transaction_hash() {
            use pinocchio::sysvars::Sysvar;
            let current_slot = Clock::get()?.slot;
            cpi_ix_data.tx_hash = create_tx_hash_from_hash_chains(
                &input_compressed_account_hashes,
                &output_compressed_account_hashes,
                current_slot,
            )
            .map_err(ProgramError::from)?;
        }

        // 10.2. Check for duplicate accounts between inputs and read-only ---------------------------------------------------
        check_no_duplicate_accounts_in_inputs_and_read_only(
            &cpi_ix_data.nullifiers,
            read_only_accounts,
        )?;
    }
    // 11. Sum check ---------------------------------------------------
    let num_input_accounts_by_index = sum_check(&inputs, &None, &inputs.is_compress())?;

    // 12. Compress or decompress lamports ---------------------------------------------------
    compress_or_decompress_lamports::<A, T>(&inputs, ctx)?;

    // 14. Verify read-only account inclusion by index ---------------------------------------------------
    let num_read_only_accounts_by_index =
        verify_read_only_account_inclusion_by_index(accounts.as_mut_slice(), read_only_accounts)?;

    // Get num of elements proven by zkp, for inclusion and non-inclusion.
    let num_inclusion_proof_inputs = {
        let num_read_only_accounts_by_zkp =
            read_only_accounts.len() - num_read_only_accounts_by_index;
        let num_accounts_by_zkp = num_input_accounts - num_input_accounts_by_index;
        num_accounts_by_zkp + num_read_only_accounts_by_zkp
    };

    // 15. Read state roots ---------------------------------------------------
    let mut input_compressed_account_roots = Vec::with_capacity(num_inclusion_proof_inputs);
    let state_tree_height = read_input_state_roots(
        accounts.as_slice(),
        inputs.input_accounts(),
        read_only_accounts,
        &mut input_compressed_account_roots,
    )?;

    // 16. Verify Inclusion & Non-inclusion Proof ---------------------------------------------------
    if num_inclusion_proof_inputs != 0 || num_non_inclusion_proof_inputs != 0 {
        if let Some(proof) = inputs.proof().as_ref() {
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
                    msg!(format!("proof  {:?}", proof).as_str());
                    msg!(format!(
                        "proof_input_compressed_account_hashes {:?}",
                        proof_input_compressed_account_hashes
                    )
                    .as_str());
                    msg!(format!("input roots {:?}", input_compressed_account_roots).as_str());
                    msg!(format!("read_only_accounts {:?}", read_only_accounts).as_str());
                    msg!(format!(
                        "input_compressed_accounts_with_merkle_context: {:?}",
                        inputs.input_accounts().collect::<Vec<_>>()
                    )
                    .as_str());
                    msg!(format!("new_address_roots {:?}", new_address_roots).as_str());
                    msg!(format!("new_addresses {:?}", new_addresses).as_str());
                    msg!(format!("read_only_addresses {:?}", read_only_addresses).as_str());
                    Err(e)
                }
            }?;
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
    // 17. Transfer network, address, and rollover fees ---------------------------------------------------
    //      Note: we transfer rollover fees from the system program instead
    //      of the account compression program to reduce cpi depth.
    context.transfer_fees(remaining_accounts, ctx.get_fee_payer())?;

    // No elements are to be inserted into the queue.
    // -> tx only contains read only accounts.
    if inputs.inputs_empty() && inputs.address_empty() && inputs.outputs_empty() {
        return Ok(());
    }

    // 18. Copy CPI context outputs ---------------------------------------------------
    copy_cpi_context_outputs(inputs.get_cpi_context_account(), bytes)?;
    // 19. CPI account compression program ---------------------------------------------------
    cpi_account_compression_program(context, cpi_ix_bytes)
}

/// Check that no read only account is an input account.
/// Multiple reads of the same account are allowed.
/// Multiple writes of the same account will fail at nullifier queue insertion.
#[inline(always)]
#[profile]
fn check_no_duplicate_accounts_in_inputs_and_read_only(
    input_nullifiers: &ZeroCopySliceMut<'_, u8, InsertNullifierInput, false>,
    read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
) -> Result<()> {
    for read_only_account in read_only_accounts {
        for input_nullifier in input_nullifiers.iter() {
            if read_only_account.account_hash == input_nullifier.account_hash {
                return Err(SystemProgramError::DuplicateAccountInInputsAndReadOnly.into());
            }
        }
    }
    Ok(())
}

#[inline(always)]
#[profile]
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
