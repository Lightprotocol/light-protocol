use anchor_lang::prelude::{AccountInfo, ProgramError};

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
pub fn process_transfer<'a, 'b, 'c, 'info>(
    accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let inputs = CompressedTokenInstructionDataMultiTransfer::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;
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
    let (mut compressed_input_accounts, input_token_data, input_lamports) =
        create_input_compressed_account::<NOT_FROZEN>()?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check(
        &input_token_data,
        &inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.amount)
            .collect::<Vec<u64>>(),
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

    let output_lamports = create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.owner)
            .collect::<Vec<Pubkey>>()
            .as_slice(),
        delegate,
        is_delegate,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data: &PackedTokenTransferOutputData| data.amount)
            .collect::<Vec<u64>>()
            .as_slice(),
        Some(
            inputs
                .output_compressed_accounts
                .iter()
                .map(|data: &PackedTokenTransferOutputData| data.lamports)
                .collect::<Vec<Option<u64>>>(),
        ),
        &hashed_mint,
        &inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.merkle_tree_index)
            .collect::<Vec<u8>>(),
        ctx.remaining_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    // If input and output lamports are unbalanced create a change account
    // without token data.
    let change_lamports = input_lamports - output_lamports;
    if change_lamports > 0 {
        let new_len = output_compressed_accounts.len() + 1;
        // Resize vector to new_len so that no unnecessary memory is allocated.
        // (Rust doubles the size of the vector when pushing to a full vector.)
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
