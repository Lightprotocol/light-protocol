use std::cmp::min;

use light_compressed_account::{
    instruction_data::{
        compressed_proof::CompressedProof,
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, InsertNullifierInput},
        traits::{InputAccount, InstructionData, NewAddress},
        zero_copy::{ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
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
        account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
        remaining_account_checks::{self, AcpAccount},
    },
    constants::CPI_AUTHORITY_PDA_BUMP,
    context::{SystemContext, WrappedInstructionData},
    cpi_context::{process_cpi_context, state::deserialize_cpi_context_account_cleared},
    errors::SystemProgramError,
    processor::{
        cpi, create_address_cpi_data, create_inputs_cpi_data,
        create_outputs_cpi_data::{self, check_new_address_assignment},
        read_only_account, read_only_address, sol_compression, sum_check, verify_proof,
    },
    Result,
};

pub trait Processor {
    const ID: Pubkey;

    // === CPI invoke steps ===

    fn cpi_signer_checks<'a, T: InstructionData<'a>>(
        invoking_program_id: &Pubkey,
        authority: &Pubkey,
        inputs: &WrappedInstructionData<'a, T>,
    ) -> Result<()> {
        crate::invoke_cpi::verify_signer::cpi_signer_checks(invoking_program_id, authority, inputs)
    }

    fn process_cpi_context<'a, 'info, T: InstructionData<'a>>(
        instruction_data: WrappedInstructionData<'a, T>,
        cpi_context_account_info: Option<&'info AccountInfo>,
        fee_payer: Pubkey,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Option<(usize, WrappedInstructionData<'a, T>)>> {
        process_cpi_context::process_cpi_context(
            instruction_data,
            cpi_context_account_info,
            fee_payer,
            remaining_accounts,
            &Self::ID,
        )
    }

    // === Pipeline steps ===

    #[allow(clippy::too_many_arguments)]
    fn create_cpi_data_and_context<'info, A: InvokeAccounts<'info> + SignerAccounts<'info>>(
        ctx: &A,
        num_leaves: u8,
        num_nullifiers: u8,
        num_new_addresses: u8,
        hashed_pubkeys_capacity: usize,
        cpi_data_len: usize,
        invoking_program_id: Option<Pubkey>,
        remaining_accounts: &'info [AccountInfo],
    ) -> Result<(SystemContext<'info>, Vec<u8>)> {
        cpi::create_cpi_data_and_context(
            ctx,
            num_leaves,
            num_nullifiers,
            num_new_addresses,
            hashed_pubkeys_capacity,
            cpi_data_len,
            invoking_program_id,
            remaining_accounts,
        )
    }

    fn try_from_account_infos<'info>(
        remaining_accounts: &'info [AccountInfo],
        context: &mut SystemContext<'info>,
    ) -> Result<Vec<AcpAccount<'info>>> {
        Ok(remaining_account_checks::try_from_account_infos(
            remaining_accounts,
            context,
        )?)
    }

    fn read_address_roots<'a, 'b: 'a>(
        accounts: &[AcpAccount<'_>],
        new_address_params: impl Iterator<Item = &'a (dyn NewAddress<'b> + 'a)>,
        read_only_addresses: &'a [ZPackedReadOnlyAddress],
        address_roots: &'a mut Vec<[u8; 32]>,
    ) -> std::result::Result<u8, SystemProgramError> {
        verify_proof::read_address_roots(
            accounts,
            new_address_params,
            read_only_addresses,
            address_roots,
        )
    }

    fn derive_new_addresses<'info, 'a, 'b: 'a, const ADDRESS_ASSIGNMENT: bool>(
        new_address_params: impl Iterator<Item = &'a (dyn NewAddress<'b> + 'a)>,
        remaining_accounts: &'info [AccountInfo],
        context: &mut SystemContext<'info>,
        cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
        accounts: &[AcpAccount<'info>],
    ) -> Result<()> {
        create_address_cpi_data::derive_new_addresses::<ADDRESS_ASSIGNMENT>(
            new_address_params,
            remaining_accounts,
            context,
            cpi_ix_data,
            accounts,
        )
    }

    fn verify_read_only_address_queue_non_inclusion(
        accounts: &mut [AcpAccount<'_>],
        read_only_addresses: &[ZPackedReadOnlyAddress],
    ) -> Result<()> {
        read_only_address::verify_read_only_address_queue_non_inclusion(
            accounts,
            read_only_addresses,
        )
    }

    fn create_outputs_cpi_data<'a, 'info, T: InstructionData<'a>>(
        inputs: &WrappedInstructionData<'a, T>,
        remaining_accounts: &'info [AccountInfo],
        context: &mut SystemContext<'info>,
        cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
        accounts: &[AcpAccount<'info>],
    ) -> Result<[u8; 32]> {
        create_outputs_cpi_data::create_outputs_cpi_data::<T>(
            inputs,
            remaining_accounts,
            context,
            cpi_ix_data,
            accounts,
        )
    }

    fn create_inputs_cpi_data<'a, 'info, T: InstructionData<'a>>(
        remaining_accounts: &'info [AccountInfo],
        inputs: &WrappedInstructionData<'a, T>,
        context: &mut SystemContext<'info>,
        cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
        accounts: &[AcpAccount<'info>],
    ) -> Result<[u8; 32]> {
        create_inputs_cpi_data::create_inputs_cpi_data(
            remaining_accounts,
            inputs,
            context,
            cpi_ix_data,
            accounts,
        )
    }

    fn sum_check<'a, T: InstructionData<'a>>(
        inputs: &WrappedInstructionData<'a, T>,
        relay_fee: &Option<u64>,
        is_compress: &bool,
    ) -> Result<usize> {
        sum_check::sum_check(inputs, relay_fee, is_compress)
    }

    fn compress_or_decompress_lamports<'a, 'info, A, T>(
        inputs: &WrappedInstructionData<'a, T>,
        ctx: &A,
    ) -> Result<()>
    where
        A: InvokeAccounts<'info> + SignerAccounts<'info>,
        T: InstructionData<'a>,
    {
        sol_compression::compress_or_decompress_lamports::<A, T>(inputs, ctx)
    }

    fn verify_read_only_account_inclusion_by_index(
        accounts: &mut [AcpAccount<'_>],
        read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
    ) -> Result<usize> {
        read_only_account::verify_read_only_account_inclusion_by_index(accounts, read_only_accounts)
    }

    fn read_input_state_roots<'a: 'b, 'b>(
        accounts: &[AcpAccount<'_>],
        input_accounts: impl Iterator<Item = &'b (dyn InputAccount<'a> + 'b)>,
        read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
        input_roots: &mut Vec<[u8; 32]>,
    ) -> std::result::Result<u8, SystemProgramError> {
        verify_proof::read_input_state_roots(
            accounts,
            input_accounts,
            read_only_accounts,
            input_roots,
        )
    }

    fn verify_proof(
        roots: &[[u8; 32]],
        leaves: &[[u8; 32]],
        address_roots: &[[u8; 32]],
        addresses: &[[u8; 32]],
        compressed_proof: &CompressedProof,
        address_tree_height: u8,
        state_tree_height: u8,
    ) -> Result<()> {
        verify_proof::verify_proof(
            roots,
            leaves,
            address_roots,
            addresses,
            compressed_proof,
            address_tree_height,
            state_tree_height,
        )
    }

    fn transfer_fees(
        context: &SystemContext<'_>,
        remaining_accounts: &[AccountInfo],
        fee_payer: &AccountInfo,
    ) -> Result<()> {
        context.transfer_fees(remaining_accounts, fee_payer)
    }

    fn cpi_account_compression_program(
        context: SystemContext<'_>,
        cpi_ix_bytes: Vec<u8>,
    ) -> Result<()> {
        cpi::cpi_account_compression_program(context, cpi_ix_bytes)
    }

    fn reinit_cpi_context_account(accounts: &[AccountInfo]) -> Result<()> {
        crate::accounts::init_context_account::reinit_cpi_context_account(accounts, &Self::ID)
    }

    // === Orchestrators ===

    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    #[profile]
    fn process<
        'a,
        'info,
        const ADDRESS_ASSIGNMENT: bool,
        A: InvokeAccounts<'info> + SignerAccounts<'info>,
        T: InstructionData<'a> + 'a,
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

        let hashed_pubkeys_capacity =
            1 + remaining_accounts.len() + num_output_compressed_accounts + cpi_context_inputs_len;

        let cpi_outputs_data_len = inputs.get_cpi_context_outputs_end_offset()
            - inputs.get_cpi_context_outputs_start_offset();
        // 1. Allocate cpi data and initialize context
        let (mut context, mut cpi_ix_bytes) = Self::create_cpi_data_and_context(
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
        let mut accounts = Self::try_from_account_infos(remaining_accounts, &mut context)?;
        // 3. Deserialize cpi instruction data as zero copy to fill it.
        let (mut cpi_ix_data, bytes) = InsertIntoQueuesInstructionDataMut::new_at(
            &mut cpi_ix_bytes[12..],
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

        // 4. Create new & verify read-only addresses
        let read_only_addresses = inputs.read_only_addresses().unwrap_or_default();
        let num_of_read_only_addresses = read_only_addresses.len();
        let num_non_inclusion_proof_inputs = num_new_addresses + num_of_read_only_addresses;

        let mut new_address_roots = Vec::with_capacity(num_non_inclusion_proof_inputs);
        // 5. Read address roots
        let address_tree_height = Self::read_address_roots(
            accounts.as_slice(),
            inputs.new_addresses(),
            read_only_addresses,
            &mut new_address_roots,
        )?;

        // 6. Collect all addresses
        inputs.input_accounts().for_each(|account| {
            context.addresses.push(account.address());
        });

        // 7. Derive new addresses from seed and invoking program
        if num_new_addresses != 0 {
            Self::derive_new_addresses::<ADDRESS_ASSIGNMENT>(
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
        Self::verify_read_only_address_queue_non_inclusion(
            accounts.as_mut_slice(),
            inputs.read_only_addresses().unwrap_or_default(),
        )?;

        // 9. Create cpi data for outputs
        let output_compressed_account_hashes = Self::create_outputs_cpi_data::<T>(
            &inputs,
            remaining_accounts,
            &mut context,
            &mut cpi_ix_data,
            accounts.as_slice(),
        )?;
        let read_only_accounts = inputs.read_only_accounts().unwrap_or_default();

        // 10. hash input compressed accounts
        if !inputs.inputs_empty() {
            let input_compressed_account_hashes = Self::create_inputs_cpi_data(
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

            // 10.2. Check for duplicate accounts between inputs and read-only
            check_no_duplicate_accounts_in_inputs_and_read_only(
                &cpi_ix_data.nullifiers,
                read_only_accounts,
            )?;
        }
        // 11. Sum check
        let num_input_accounts_by_index = Self::sum_check(&inputs, &None, &inputs.is_compress())?;

        // 12. Compress or decompress lamports
        Self::compress_or_decompress_lamports::<A, T>(&inputs, ctx)?;

        // 14. Verify read-only account inclusion by index
        let num_read_only_accounts_by_index = Self::verify_read_only_account_inclusion_by_index(
            accounts.as_mut_slice(),
            read_only_accounts,
        )?;

        // Get num of elements proven by zkp
        let num_inclusion_proof_inputs = {
            let num_read_only_accounts_by_zkp =
                read_only_accounts.len() - num_read_only_accounts_by_index;
            let num_accounts_by_zkp = num_input_accounts - num_input_accounts_by_index;
            num_accounts_by_zkp + num_read_only_accounts_by_zkp
        };

        // 15. Read state roots
        let mut input_compressed_account_roots = Vec::with_capacity(num_inclusion_proof_inputs);
        let state_tree_height = Self::read_input_state_roots(
            accounts.as_slice(),
            inputs.input_accounts(),
            read_only_accounts,
            &mut input_compressed_account_roots,
        )?;

        // 16. Verify Inclusion & Non-inclusion Proof
        if num_inclusion_proof_inputs != 0 || num_non_inclusion_proof_inputs != 0 {
            if let Some(proof) = inputs.proof().as_ref() {
                let mut new_addresses = Vec::with_capacity(num_non_inclusion_proof_inputs);
                for new_address in cpi_ix_data.addresses.iter() {
                    new_addresses.push(new_address.address);
                }
                for read_only_address in read_only_addresses.iter() {
                    new_addresses.push(read_only_address.address);
                }
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
                match Self::verify_proof(
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
        // 17. Transfer network, address, and rollover fees
        Self::transfer_fees(&context, remaining_accounts, ctx.get_fee_payer())?;

        // No elements are to be inserted into the queue.
        if inputs.inputs_empty() && inputs.address_empty() && inputs.outputs_empty() {
            return Ok(());
        }

        // 18. Copy CPI context outputs
        process_cpi_context::copy_cpi_context_outputs(inputs.get_cpi_context_account(), bytes)?;
        // 19. CPI account compression program
        Self::cpi_account_compression_program(context, cpi_ix_bytes)
    }

    #[inline(never)]
    #[profile]
    #[allow(unused_mut)]
    fn process_invoke_cpi<
        'a,
        'info,
        const ADDRESS_ASSIGNMENT: bool,
        A: SignerAccounts<'info> + InvokeAccounts<'info> + CpiContextAccountTrait<'info>,
        T: InstructionData<'a> + 'a,
    >(
        invoking_program: Pubkey,
        accounts: A,
        instruction_data: T,
        remaining_accounts: &'info [AccountInfo],
    ) -> Result<()> {
        let instruction_data = WrappedInstructionData::new(instruction_data)?;

        Self::cpi_signer_checks::<T>(
            &invoking_program,
            accounts.get_authority().key(),
            &instruction_data,
        )?;

        let (cpi_context_inputs_len, instruction_data) = match Self::process_cpi_context(
            instruction_data,
            accounts.get_cpi_context_account(),
            *accounts.get_fee_payer().key(),
            remaining_accounts,
        ) {
            Ok(Some(instruction_data)) => instruction_data,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };
        // 3. Process input data and cpi the account compression program.
        Self::process::<ADDRESS_ASSIGNMENT, A, T>(
            instruction_data,
            Some(invoking_program),
            &accounts,
            cpi_context_inputs_len,
            remaining_accounts,
        )?;

        // 4. clear cpi context account
        if cpi_context_inputs_len > 0 {
            deserialize_cpi_context_account_cleared(
                accounts.get_cpi_context_account().unwrap(),
                &Self::ID,
            )?;
        }
        Ok(())
    }
}

/// Check that no read only account is an input account.
#[inline(always)]
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
        if !read_only_account.merkle_context.prove_by_index() {
            proof_input_compressed_account_hashes.push(read_only_account.account_hash);
        }
    }
}
