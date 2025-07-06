use anchor_compressed_token::process_transfer::sum_check;
use anchor_lang::prelude::{AccountInfo, ProgramError};
use light_heap::{bench_sbf_end, bench_sbf_start};

use crate::{
    multi_transfer::{
        accounts::MultiTransferValidatedAccounts,
        instruction_data::CompressedTokenInstructionDataMultiTransfer,
    },
    shared::{inputs::create_input_compressed_account, outputs::create_output_compressed_account},
};
use light_zero_copy::borsh::{Deserialize, DeserializeMut};

const NOT_FROZEN: bool = true;

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
    accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data first to determine optional accounts
    let (inputs, _) = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;
    
    // Determine optional account flags from instruction data
    let with_sol_pool = inputs.compress_or_decompress_amount.is_some();
    let with_cpi_context = inputs.cpi_context.is_some();
    
    // Validate and parse accounts
    let (validated_accounts, packed_accounts) = MultiTransferValidatedAccounts::validate_and_parse(
        accounts,
        &light_compressed_token::ID,
        with_sol_pool,
        with_cpi_context,
    )?;
    if inputs.in_lamports.len() > inputs.in_token_data.len() {
        unimplemented!("Tlv is unimplemented");
    }
    if inputs.out_lamports.len() > inputs.out_token_data.len() {
        unimplemented!("Tlv is unimplemented");
    }
    if inputs.in_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }
    if inputs.out_tlv.is_some() {
        unimplemented!("Tlv is unimplemented");
    }

    bench_sbf_start!("t_context_and_check_sig");
    if inputs.input_token_data_with_context.is_empty()
        && inputs.compress_or_decompress_amount.is_none()
    {
        return Err(crate::ErrorCode::NoInputTokenAccountsProvided);
    }

    // TODO: create TokenContext
    // TODO: create cpi bytes
    // TODO: create cpi zero copy

    create_input_compressed_account::<NOT_FROZEN>()?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check(
        &inputs.in_token_data.as_slice(),
        &inputs.out_token_data.as_slice(),
        inputs.compress_or_decompress_amount.as_ref(),
        inputs.is_compress,
    )?;
    // TODO: add later
    // bench_sbf_end!("t_sum_check");
    // bench_sbf_start!("t_process_compression");
    // if inputs.compress_or_decompress_amount.is_some() {
    //     process_compression_or_decompression(&inputs, &ctx)?;
    // }
    // bench_sbf_end!("t_process_compression");
    // bench_sbf_start!("t_create_output_compressed_accounts");

    let output_lamports = create_output_compressed_account()?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    // TODO: calculate lamports
    // If input and output lamports are unbalanced create a change account
    // without token data.
    let change_lamports = input_lamports - output_lamports;
    if change_lamports > 0 {
        // TODO: use zero copy
        output_compressed_accounts.resize(
            new_len,
            OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: ctx.accounts.authority.key().into(),
                    lamports: change_lamports,
                    data: None,
                    address: None,
                },
                merkle_tree_index: inputs.output_compressed_accounts[0].merkle_tree_index,
            },
        );
    }

    execute_cpi_invoke()
}
