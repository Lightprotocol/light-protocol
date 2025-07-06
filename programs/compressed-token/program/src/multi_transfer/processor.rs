use anchor_compressed_token::process_transfer::sum_check;
use anchor_lang::prelude::{AccountInfo, ProgramError};
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::{
    InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
    ZInstructionDataInvokeCpiWithReadOnlyMut,
};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use solana_pubkey::Pubkey;

use crate::{
    multi_transfer::{
        accounts::{MultiTransferPackedAccounts, MultiTransferValidatedAccounts},
        instruction_data::{
            CompressedTokenInstructionDataMultiTransfer,
            ZCompressedTokenInstructionDataMultiTransfer,
        },
    },
    shared::{
        context::TokenContext,
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        inputs::create_input_compressed_account,
        outputs::create_output_compressed_account,
    },
    LIGHT_CPI_SIGNER,
};

const NOT_FROZEN: bool = false;

/// Validate instruction data consistency (lamports and TLV checks)
fn validate_instruction_data(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
) -> Result<(), ProgramError> {
    if let Some(ref in_lamports) = inputs.in_lamports {
        if in_lamports.len() > inputs.in_token_data.len() {
            unimplemented!("Tlv is unimplemented");
        }
    }
    if let Some(ref out_lamports) = inputs.out_lamports {
        if out_lamports.len() > inputs.out_token_data.len() {
            unimplemented!("Tlv is unimplemented");
        }
    }
    if inputs.in_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }
    if inputs.out_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }
    Ok(())
}

/// Build CPI configuration from instruction data
fn build_cpi_config_input(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
) -> (Vec<u8>, InstructionDataInvokeCpiWithReadOnlyConfig) {
    // Build CPI configuration based on delegate flags
    let mut input_delegate_flags = ArrayVec::new();
    for input_data in inputs.in_token_data.iter() {
        input_delegate_flags.push(input_data.with_delegate != 0);
    }

    let mut output_delegate_flags = ArrayVec::new();
    for output_data in inputs.out_token_data.iter() {
        // Check if output has delegate (delegate index != 0 means delegate is present)
        output_delegate_flags.push(output_data.delegate != 0);
    }

    let config_input = CpiConfigInput {
        input_accounts: input_delegate_flags,
        output_accounts: output_delegate_flags,
        has_proof: inputs.proof.is_some(),
        compressed_mint: false,
        compressed_mint_with_freeze_authority: false,
    };
    let config = cpi_bytes_config(config_input);
    (allocate_invoke_with_read_only_cpi_bytes(&config), config)
}

/// Process input compressed accounts and return total input lamports
fn assign_input_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    context: &mut TokenContext,
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Result<u64, ProgramError> {
    let mut total_input_lamports = 0u64;

    for (i, input_data) in inputs.in_token_data.iter().enumerate() {
        let input_lamports = if let Some(lamports) = inputs.in_lamports.as_ref() {
            if let Some(input_lamports) = lamports.get(i) {
                input_lamports.get()
            } else {
                0
            }
        } else {
            0
        };

        total_input_lamports += input_lamports;

        create_input_compressed_account::<NOT_FROZEN>(
            cpi_instruction_struct
                .input_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            context,
            input_data,
            packed_accounts.accounts,
            input_lamports,
        )?;
    }

    Ok(total_input_lamports)
}

/// Process output compressed accounts and return total output lamports
fn assign_output_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    context: &mut TokenContext,
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Result<u64, ProgramError> {
    let mut total_output_lamports = 0u64;

    for (i, output_data) in inputs.out_token_data.iter().enumerate() {
        let output_lamports = if let Some(lamports) = inputs.out_lamports.as_ref() {
            if let Some(lamports) = lamports.get(i) {
                lamports.get()
            } else {
                0
            }
        } else {
            0
        };

        total_output_lamports += output_lamports;

        // Get mint account using mint index from input data (all transfers should use same mint)
        let mint_index = if let Some(first_input) = inputs.in_token_data.first() {
            first_input.mint
        } else {
            return Err(ProgramError::InvalidInstructionData);
        };
        let mint_account = packed_accounts.get_u8(mint_index)?;
        let hashed_mint = context.get_or_hash_pubkey(mint_account.key);

        // Get owner account using owner index
        let owner_account = packed_accounts.get_u8(output_data.owner)?;
        let owner_pubkey = *owner_account.key;

        // Get delegate if present
        let delegate_pubkey = if output_data.delegate != 0 {
            let delegate_account = packed_accounts.get_u8(output_data.delegate)?;
            Some(*delegate_account.key)
        } else {
            None
        };

        create_output_compressed_account(
            cpi_instruction_struct
                .output_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            context,
            owner_pubkey.into(),
            delegate_pubkey.map(|d| d.into()),
            output_data.amount,
            if output_lamports > 0 {
                Some(output_lamports)
            } else {
                None
            },
            mint_account.key.into(),
            &hashed_mint,
            output_data.merkle_tree,
        )?;
    }

    Ok(total_output_lamports)
}

/// Extract tree accounts from merkle contexts for CPI call
fn get_cpi_tree_accounts(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Vec<Pubkey> {
    //  don't pass any tree accounts if we write into the cpi context
    if inputs.cpi_context.is_some()
        && (inputs.cpi_context.unwrap().first_set_context
            || inputs.cpi_context.unwrap().set_context)
    {
        return vec![];
    }
    let mut tree_accounts = Vec::new();

    // Add input merkle trees and queues (skip non-tree accounts)
    for input_data in inputs.in_token_data.iter() {
        let merkle_tree_index = input_data.merkle_context.merkle_tree_pubkey_index;
        let queue_index = input_data.merkle_context.queue_pubkey_index;

        // Only add accounts that are actually trees/queues (typically higher indices)
        if let Some(merkle_tree_account) = packed_accounts.accounts.get(merkle_tree_index as usize)
        {
            tree_accounts.push(*merkle_tree_account.key);
        }
        if let Some(queue_account) = packed_accounts.accounts.get(queue_index as usize) {
            tree_accounts.push(*queue_account.key);
        }
    }

    // Add output merkle trees (skip non-tree accounts)
    for output_data in inputs.out_token_data.iter() {
        if let Some(tree_account) = packed_accounts
            .accounts
            .get(output_data.merkle_tree as usize)
        {
            tree_accounts.push(*tree_account.key);
        }
    }

    tree_accounts
}

/// Process a token transfer instruction
/// build inputs -> sum check -> build outputs -> add token data to inputs -> invoke cpi
/// 1.  Unpack compressed input accounts and input token data, this uses
///     standardized signer / delegate and will fail in proof verification in
///     case either is invalid.
/// 2.  Check that compressed accounts are of same mint.
/// 3.  Check that sum of input compressed accounts is equal to sum of output
///     compressed accounts
/// 4.  create_output_compressed_accounts
/// 5.  Serialize and add token_data data to in compressed_accounts.
/// 6.  Invoke light_system_program::execute_compressed_transaction.
#[inline(always)]
pub fn process_multi_transfer<'info>(
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Determine optional account flags from instruction data
    let with_sol_pool = inputs.compress_or_decompress_amount.is_some();
    let with_cpi_context = inputs.cpi_context.is_some();

    // Validate and parse accounts
    // TODO: only return remaining accounts into fn validate ix data
    let (validated_accounts, packed_accounts) = MultiTransferValidatedAccounts::validate_and_parse(
        accounts,
        &crate::ID,
        with_sol_pool,
        with_cpi_context,
    )?;
    // Validate instruction data consistency
    validate_instruction_data(&inputs)?;
    bench_sbf_start!("t_context_and_check_sig");
    if inputs.in_token_data.is_empty() && inputs.compress_or_decompress_amount.is_none() {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Create TokenContext for hash caching
    let mut context = TokenContext::new();

    // Allocate CPI bytes and create zero-copy structure
    let (mut cpi_bytes, config) = build_cpi_config_input(&inputs);

    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;

    // Set CPI signer information
    cpi_instruction_struct.bump = LIGHT_CPI_SIGNER.bump;
    cpi_instruction_struct.invoking_program_id = LIGHT_CPI_SIGNER.program_id.into();

    // Process input compressed accounts
    let total_input_lamports = assign_input_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check(
        &inputs.in_token_data,
        &inputs.out_token_data,
        inputs.compress_or_decompress_amount.as_ref().map(|x| **x),
        inputs.is_compress(),
    )?;
    bench_sbf_end!("t_sum_check");

    // Process output compressed accounts
    let total_output_lamports = assign_output_compressed_accounts(
        &mut cpi_instruction_struct,
        &mut context,
        &inputs,
        &packed_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    // If input and output lamports are unbalanced, handle the difference
    // Note: For now, we assume they should be balanced. Add change account logic later if needed.
    if total_input_lamports != total_output_lamports {
        // For multi-transfer, lamports should typically be balanced
        // Future enhancement: create change account for lamport differences
        // // Handle compression/decompression amount
        // if let Some(compress_amount) = inputs.compress_or_decompress_amount {
        //     cpi_instruction_struct.compress_or_decompress_lamports = *compress_amount;
        //     cpi_instruction_struct.is_compress = if inputs.is_compress() { 1 } else { 0 };
        // }
    }

    // Extract tree accounts from merkle contexts for CPI call
    let tree_accounts = get_cpi_tree_accounts(&inputs, &packed_accounts);

    // Execute CPI call to light-system-program
    execute_cpi_invoke(
        accounts,
        cpi_bytes,
        &tree_accounts,
        with_sol_pool,
        validated_accounts.cpi_context_account.map(|x| *x.key),
    )?;

    Ok(())
}
