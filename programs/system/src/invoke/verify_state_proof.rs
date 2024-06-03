use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use anchor_lang::{prelude::*, Bumps};
use light_hasher::Poseidon;
use light_macros::heap_neutral;
use light_utils::hash_to_bn254_field_size_be;
use light_verifier::{
    verify_create_addresses_and_merkle_proof_zkp, verify_create_addresses_zkp,
    verify_merkle_proof_zkp, CompressedProof,
};

use crate::{
    errors::SystemProgramError,
    invoke::InstructionDataInvoke,
    sdk::{accounts::InvokeAccounts, compressed_account::PackedCompressedAccountWithMerkleContext},
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

#[inline(never)]
#[heap_neutral]
pub fn fetch_roots<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    input_compressed_accounts_with_merkle_context: &'a [PackedCompressedAccountWithMerkleContext],
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, input_compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts[input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as usize],
        )
        .unwrap();
        let merkle_tree = merkle_tree.load()?;
        let fetched_roots = merkle_tree.load_roots()?;

        roots[j] =
            fetched_roots[input_compressed_accounts_with_merkle_context[j].root_index as usize];
    }
    Ok(())
}

#[inline(never)]
pub fn fetch_roots_address_merkle_tree<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + Bumps,
>(
    new_address_params: &'a [NewAddressParamsPacked],
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, index_mt_account) in new_address_params.iter().enumerate() {
        let merkle_tree = AccountLoader::<AddressMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts[index_mt_account.address_merkle_tree_account_index as usize],
        )
        .unwrap();
        let merkle_tree = merkle_tree.load()?;
        let fetched_roots = merkle_tree.load_roots()?;

        roots[j] = fetched_roots[index_mt_account.address_merkle_tree_root_index as usize];
    }
    Ok(())
}

#[inline(never)]
#[heap_neutral]
#[allow(unused_mut)]
pub fn hash_input_compressed_accounts<'a, 'b, 'c: 'info, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    inputs: &'a InstructionDataInvoke,
    leaves: &'a mut [[u8; 32]],
    addresses: &'a mut [Option<[u8; 32]>],
    hashed_pubkeys: &'a mut Vec<(Pubkey, [u8; 32])>,
) -> Result<()> {
    let mut owner_pubkey = inputs.input_compressed_accounts_with_merkle_context[0]
        .compressed_account
        .owner;
    let mut hashed_owner = hash_to_bn254_field_size_be(&owner_pubkey.to_bytes())
        .unwrap()
        .0;
    hashed_pubkeys.push((owner_pubkey, hashed_owner));
    let mut current_hashed_mt = [0u8; 32];

    let mut current_mt_index: i16 = -1;
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        // For heap neutrality we cannot allocate new heap memory in this function.
        match &input_compressed_account_with_context
            .compressed_account
            .address
        {
            Some(address) => addresses[j] = Some(*address),
            None => {}
        };
        if current_mt_index
            != input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as i16
        {
            current_mt_index = input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as i16;
            let merkle_tree_pubkey = remaining_accounts[input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index
                as usize]
                .key();
            current_hashed_mt = hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                .unwrap()
                .0;
            hashed_pubkeys.push((merkle_tree_pubkey, current_hashed_mt));
        }

        // Without cpi context all input compressed accounts have the same owner.
        #[cfg(feature = "cpi-context")]
        if owner_pubkey
            != input_compressed_account_with_context
                .compressed_account
                .owner
        {
            owner_pubkey = input_compressed_account_with_context
                .compressed_account
                .owner;
            hashed_owner = match hashed_pubkeys.iter().find(|x| {
                x.0 == inputs.output_compressed_accounts[j]
                    .compressed_account
                    .owner
            }) {
                Some(hashed_owner) => hashed_owner.1,
                None => {
                    let hashed_owner = hash_to_bn254_field_size_be(
                        &inputs.output_compressed_accounts[j]
                            .compressed_account
                            .owner
                            .to_bytes(),
                    )
                    .unwrap()
                    .0;
                    hashed_pubkeys.push((
                        inputs.output_compressed_accounts[j]
                            .compressed_account
                            .owner,
                        hashed_owner,
                    ));
                    hashed_owner
                }
            };
        }
        leaves[j] = input_compressed_account_with_context
            .compressed_account
            .hash_with_hashed_values::<Poseidon>(
                &hashed_owner,
                &current_hashed_mt,
                &input_compressed_account_with_context
                    .merkle_context
                    .leaf_index,
            )?;
    }
    Ok(())
}

#[heap_neutral]
pub fn verify_state_proof(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> anchor_lang::Result<()> {
    if !addresses.is_empty() && !leaves.is_empty() {
        verify_create_addresses_and_merkle_proof_zkp(
            roots,
            leaves,
            address_roots,
            addresses,
            compressed_proof,
        )
        .map_err(ProgramError::from)?;
    } else if !addresses.is_empty() {
        verify_create_addresses_zkp(address_roots, addresses, compressed_proof)
            .map_err(ProgramError::from)?;
    } else {
        verify_merkle_proof_zkp(roots, leaves, compressed_proof).map_err(ProgramError::from)?;
    }
    Ok(())
}

#[inline(never)]
#[heap_neutral]
pub fn sum_check(
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    relay_fee: &Option<u64>,
    compress_or_decompress_lamports: &Option<u64>,
    is_compress: &bool,
) -> Result<()> {
    let mut sum: u64 = 0;
    for compressed_account_with_context in input_compressed_accounts_with_merkle_context.iter() {
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
        Ok(())
    } else {
        Err(SystemProgramError::SumCheckFailed.into())
    }
}

#[cfg(test)]
mod test {
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;
    use crate::sdk::compressed_account::{CompressedAccount, PackedMerkleContext};

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
    }

    fn sum_check_test(
        input_amounts: &[u64],
        output_amounts: &[u64],
        relay_fee: Option<u64>,
        compress_or_decompress_lamports: Option<u64>,
        is_compress: bool,
    ) -> Result<()> {
        let mut inputs = Vec::new();
        for i in input_amounts.iter() {
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
                },
                root_index: 1,
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

        sum_check(
            &inputs,
            &outputs,
            &relay_fee,
            &compress_or_decompress_lamports,
            &is_compress,
        )
    }
}
