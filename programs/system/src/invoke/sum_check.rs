use crate::{
    errors::SystemProgramError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
};
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::Result;
use light_macros::heap_neutral;

#[inline(never)]
#[heap_neutral]
pub fn sum_check(
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    relay_fee: &Option<u64>,
    compress_or_decompress_lamports: &Option<u64>,
    is_compress: &bool,
) -> Result<(usize, usize)> {
    let mut sum: u64 = 0;
    let mut num_read_only = 0;
    let mut num_prove_by_index_accounts = 0;
    for compressed_account_with_context in input_compressed_accounts_with_merkle_context.iter() {
        if compressed_account_with_context
            .merkle_context
            .queue_index
            .is_some()
        {
            num_prove_by_index_accounts += 1;
        }
        // Readonly accounts are not included in the sum check, since these are
        // not invalidated in this transaction.
        if compressed_account_with_context.read_only {
            // unimplemented!("read_only accounts are not supported. Set read_only to false.");
            num_read_only += 1;
            continue;
        }
        sum = sum
            .checked_add(compressed_account_with_context.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| SystemProgramError::ComputeInputSumFailed)?;
    }

    match compress_or_decompress_lamports {
        Some(lamports) => {
            if *is_compress {
                sum = sum
                    .checked_add(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| SystemProgramError::ComputeOutputSumFailed)?;
            } else {
                sum = sum
                    .checked_sub(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| SystemProgramError::ComputeOutputSumFailed)?;
            }
        }
        None => (),
    }

    for compressed_account in output_compressed_accounts.iter() {
        sum = sum
            .checked_sub(compressed_account.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| SystemProgramError::ComputeOutputSumFailed)?;
    }

    if let Some(relay_fee) = relay_fee {
        sum = sum
            .checked_sub(*relay_fee)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| SystemProgramError::ComputeRpcSumFailed)?;
    }

    if sum == 0 {
        Ok((num_read_only, num_prove_by_index_accounts))
    } else {
        Err(SystemProgramError::SumCheckFailed.into())
    }
}

#[cfg(test)]
mod test {
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;
    use crate::sdk::compressed_account::{CompressedAccount, PackedMerkleContext, QueueIndex};

    #[test]
    fn test_sum_check() {
        // SUCCEED: no relay fee, compression
        sum_check_test(&[100, 50], &[150], None, None, false).unwrap();
        sum_check_test(&[75, 25, 25], &[25, 25, 25, 25, 12, 13], None, None, false).unwrap();

        // FAIL: no relay fee, compression
        sum_check_test(&[100, 50], &[150 + 1], None, None, false).unwrap_err();
        sum_check_test(&[100, 50], &[150 - 1], None, None, false).unwrap_err();
        sum_check_test(&[100, 50], &[], None, None, false).unwrap_err();
        sum_check_test(&[], &[100, 50], None, None, false).unwrap_err();
        sum_check_test(&[100, 50], &[0], None, None, false).unwrap_err();
        sum_check_test(&[0], &[100, 50], None, None, false).unwrap_err();

        // SUCCEED: empty
        sum_check_test(&[], &[], None, None, true).unwrap();
        sum_check_test(&[], &[], None, None, false).unwrap();
        sum_check_test(&[0], &[0], None, None, true).unwrap();
        sum_check_test(&[0], &[0], None, None, false).unwrap();
        // FAIL: empty
        sum_check_test(&[], &[], Some(1), None, false).unwrap_err();
        sum_check_test(&[], &[], None, Some(1), false).unwrap_err();
        sum_check_test(&[], &[], None, Some(1), true).unwrap_err();

        // SUCCEED: with compress
        sum_check_test(&[100], &[123], None, Some(23), true).unwrap();
        sum_check_test(&[], &[150], None, Some(150), true).unwrap();
        // FAIL: compress
        sum_check_test(&[], &[150], None, Some(150 - 1), true).unwrap_err();
        sum_check_test(&[], &[150], None, Some(150 + 1), true).unwrap_err();

        // SUCCEED: with decompress
        sum_check_test(&[100, 50], &[100], None, Some(50), false).unwrap();
        sum_check_test(&[100, 50], &[], None, Some(150), false).unwrap();
        // FAIL: decompress
        sum_check_test(&[100, 50], &[], None, Some(150 - 1), false).unwrap_err();
        sum_check_test(&[100, 50], &[], None, Some(150 + 1), false).unwrap_err();

        // SUCCEED: with relay fee
        sum_check_test(&[100, 50], &[125], Some(25), None, false).unwrap();
        sum_check_test(&[100, 50], &[150], Some(25), Some(25), true).unwrap();
        sum_check_test(&[100, 50], &[100], Some(25), Some(25), false).unwrap();

        // FAIL: relay fee
        sum_check_test(&[100, 50], &[2125], Some(25 - 1), None, false).unwrap_err();
        sum_check_test(&[100, 50], &[2125], Some(25 + 1), None, false).unwrap_err();
        for i in 0..10 {
            for j in i..10 {
                println!("num read only = {}, num by index = {}", i, j);
                let num_non_read_only = j - i;
                if num_non_read_only != 0 {
                    sum_check_test_with_num(
                        &vec![150; j],
                        &vec![150; num_non_read_only],
                        None,
                        None,
                        false,
                        i,
                        j,
                    )
                    .unwrap();
                }
            }
        }
    }
    fn sum_check_test(
        input_amounts: &[u64],
        output_amounts: &[u64],
        relay_fee: Option<u64>,
        compress_or_decompress_lamports: Option<u64>,
        is_compress: bool,
    ) -> Result<()> {
        sum_check_test_with_num(
            input_amounts,
            output_amounts,
            relay_fee,
            compress_or_decompress_lamports,
            is_compress,
            0,
            0,
        )
    }
    fn sum_check_test_with_num(
        input_amounts: &[u64],
        output_amounts: &[u64],
        relay_fee: Option<u64>,
        compress_or_decompress_lamports: Option<u64>,
        is_compress: bool,
        num_read_only: usize,
        num_by_index: usize,
    ) -> Result<()> {
        let mut inputs = Vec::new();
        for (index, i) in input_amounts.iter().enumerate() {
            let queue_index = if index < num_by_index {
                Some(QueueIndex::default())
            } else {
                None
            };
            let read_only = index < num_read_only;
            inputs.push(PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Keypair::new().pubkey(),
                    lamports: *i,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                    queue_index,
                },
                root_index: 1,
                read_only,
            });
        }
        let mut outputs = Vec::new();
        for amount in output_amounts.iter() {
            outputs.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Keypair::new().pubkey(),
                    lamports: *amount,
                    address: None,
                    data: None,
                },
                merkle_tree_index: 0,
            });
        }

        let (calc_num_read_only, calc_num_prove_by_index_accounts) = sum_check(
            &inputs,
            &outputs,
            &relay_fee,
            &compress_or_decompress_lamports,
            &is_compress,
        )?;
        assert_eq!(num_read_only, calc_num_read_only);
        assert_eq!(num_by_index, calc_num_prove_by_index_accounts);
        Ok(())
    }
}
