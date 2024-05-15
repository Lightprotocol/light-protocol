use anchor_lang::{prelude::*, Bumps};

use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use light_hasher::Poseidon;
use light_macros::heap_neutral;
use light_utils::hash_to_bn254_field_size_be;
use light_verifier::{
    verify_create_addresses_and_merkle_proof_zkp, verify_create_addresses_zkp,
    verify_merkle_proof_zkp, CompressedProof,
};

use crate::{
    errors::CompressedPdaError,
    invoke::InstructionDataInvoke,
    sdk::{accounts::InvokeAccounts, compressed_account::PackedCompressedAccountWithMerkleContext},
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

#[inline(never)]
#[heap_neutral]
pub fn fetch_roots<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
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

        roots[j] = fetched_roots
            [inputs.input_compressed_accounts_with_merkle_context[j].root_index as usize];
    }
    Ok(())
}

// TODO: unify fetch roots and fetch_roots_address_merkle_tree
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
    // TODO: bench whether it cheaper to keep the counter or just use the hash table
    let mut none_counter = 0;
    let mut current_mt_index: i16 = -1;
    // let mut current_hashed_mt = [0u8; 32];
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        // TODO: revisit whether we can find a prettier solution
        // For heap neutrality we cannot allocate new heap memory in this function.
        // For efficiency we want to remove None elements from the addresses vector.
        match &input_compressed_account_with_context
            .compressed_account
            .address
        {
            Some(address) => addresses[j - none_counter] = Some(*address),
            None => {
                none_counter += 1;
                // TODO: debug
                // Vec::remove(addresses, j);
            }
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
        } else if current_mt_index
            < input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as i16
        {
            // TODO: add failing test
            msg!("Invalid Merkle tree index: {} current index {} (Merkle tree indices need to be in ascendin order.", input_compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey_index as i16, current_mt_index);
            return err!(CompressedPdaError::InvalidMerkleTreeIndex);
        }
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
    output_compressed_account: &[OutputCompressedAccountWithPackedContext],
    relay_fee: &Option<u64>,
    compression_lamports: &Option<u64>,
    is_compress: &bool,
) -> Result<()> {
    let mut sum: u64 = 0;
    for compressed_account_with_context in input_compressed_accounts_with_merkle_context.iter() {
        sum = sum
            .checked_add(compressed_account_with_context.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| CompressedPdaError::ComputeInputSumFailed)?;
    }

    match compression_lamports {
        Some(lamports) => {
            if *is_compress {
                sum = sum
                    .checked_add(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| CompressedPdaError::ComputeOutputSumFailed)?;
            } else {
                sum = sum
                    .checked_sub(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| CompressedPdaError::ComputeOutputSumFailed)?;
            }
        }
        None => (),
    }

    for compressed_account in output_compressed_account.iter() {
        sum = sum
            .checked_sub(compressed_account.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| CompressedPdaError::ComputeOutputSumFailed)?;
    }

    if let Some(relay_fee) = relay_fee {
        sum = sum
            .checked_sub(*relay_fee)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| CompressedPdaError::ComputeRpcSumFailed)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(CompressedPdaError::SumCheckFailed.into())
    }
}

#[cfg(test)]
mod test {
    use crate::sdk::compressed_account::{CompressedAccount, PackedMerkleContext};

    use super::*;

    #[test]
    fn test_sum_check_passes() {
        let input_compressed_accounts_with_merkle_context: Vec<
            PackedCompressedAccountWithMerkleContext,
        > = vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                },
                root_index: 1,
            },
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 1,
                },
                root_index: 1,
            },
        ];

        let output_compressed_account: Vec<OutputCompressedAccountWithPackedContext> =
            vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 150,
                    address: None,
                    data: None,
                },
                merkle_tree_index: 0,
            }];

        let relay_fee = None; // No RPC fee

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &None,
            &false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_sum_check_with_compress_passes() {
        let input_compressed_accounts_with_merkle_context: Vec<
            PackedCompressedAccountWithMerkleContext,
        > = vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                },
                root_index: 1,
            },
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 1,
                },
                root_index: 1,
            },
        ];

        let output_compressed_account: Vec<OutputCompressedAccountWithPackedContext> =
            vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 150,
                    address: None,
                    data: None,
                },
                merkle_tree_index: 0,
            }];

        let relay_fee = None; // No RPC fee

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(50),
            &true,
        );
        println!("{:?}", result);
        assert!(result.is_ok());
        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(49),
            &true,
        );
        assert!(result.is_err());
        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(50),
            &false,
        );
        assert!(result.is_err());
    }
    #[test]
    fn test_sum_check_with_decompress_passes() {
        let input_compressed_accounts_with_merkle_context: Vec<
            PackedCompressedAccountWithMerkleContext,
        > = vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                },
                root_index: 1,
            },
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 1,
                },
                root_index: 1,
            },
        ];

        let output_compressed_account: Vec<OutputCompressedAccountWithPackedContext> =
            vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_tree_index: 0,
            }];

        let relay_fee = None; // No RPC fee

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(50),
            &false,
        );
        println!("{:?}", result);
        assert!(result.is_ok());
        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(49),
            &false,
        );
        assert!(result.is_err());
        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &Some(50),
            &true,
        );
        assert!(result.is_err());
    }
    // TODO: add test for relay fee
    #[test]
    fn test_sum_check_fails() {
        let input_compressed_accounts_with_merkle_context: Vec<
            PackedCompressedAccountWithMerkleContext,
        > = vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 0,
                },
                root_index: 1,
            },
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: 1,
                },
                root_index: 1,
            },
        ];

        let output_compressed_account: Vec<OutputCompressedAccountWithPackedContext> =
            vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 25,
                    address: None,
                    data: None,
                },
                merkle_tree_index: 0,
            }];

        let relay_fee = Some(50); // Adding an RPC fee to ensure the sums don't match

        let result = sum_check(
            &input_compressed_accounts_with_merkle_context,
            &output_compressed_account,
            &relay_fee,
            &None,
            &false,
        );
        assert!(result.is_err());
    }
}
