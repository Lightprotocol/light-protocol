use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    instructions::{InstructionDataTransfer, TransferInstruction},
    ErrorCode,
};
use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use anchor_lang::prelude::*;
use light_macros::heap_neutral;
use light_verifier::{
    verify_create_addresses_and_merkle_proof_zkp, verify_create_addresses_zkp,
    verify_merkle_proof_zkp, CompressedProof,
};

#[inline(never)]
#[heap_neutral]
pub fn fetch_roots<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, input_compressed_account_with_context) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts
                [input_compressed_account_with_context.merkle_tree_pubkey_index as usize],
        )
        .unwrap();
        let merkle_tree = merkle_tree.load()?;
        let fetched_roots = merkle_tree.load_roots()?;

        roots[j] = fetched_roots[inputs.input_root_indices[j] as usize];
    }
    Ok(())
}

// TODO: unify fetch roots and fetch_roots_address_merkle_tree
#[inline(never)]
pub fn fetch_roots_address_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, index_mt_account) in inputs.new_address_params.iter().enumerate() {
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
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: &'a InstructionDataTransfer,
    leaves: &'a mut [[u8; 32]],
    addresses: &'a mut [Option<[u8; 32]>],
) -> Result<()> {
    let mut none_counter = 0;
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

        leaves[j] = input_compressed_account_with_context
            .compressed_account
            .hash(
                &ctx.remaining_accounts
                    [input_compressed_account_with_context.merkle_tree_pubkey_index as usize]
                    .key(),
                &input_compressed_account_with_context.leaf_index,
            )?;
    }
    Ok(())
}

#[inline(never)]
#[heap_neutral]
pub fn sum_check(
    input_compressed_accounts_with_merkle_context: &[CompressedAccountWithMerkleContext],
    output_compressed_account: &[CompressedAccount],
    relay_fee: &Option<u64>,
    compression_lamports: &Option<u64>,
    is_compress: &bool,
) -> Result<()> {
    let mut sum: u64 = 0;
    for compressed_account_with_context in input_compressed_accounts_with_merkle_context.iter() {
        sum = sum
            .checked_add(compressed_account_with_context.compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    match compression_lamports {
        Some(lamports) => {
            if *is_compress {
                sum = sum
                    .checked_add(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
            } else {
                sum = sum
                    .checked_sub(*lamports)
                    .ok_or(ProgramError::ArithmeticOverflow)
                    .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
            }
        }
        None => (),
    }

    for compressed_account in output_compressed_account.iter() {
        sum = sum
            .checked_sub(compressed_account.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
    }

    if let Some(relay_fee) = relay_fee {
        sum = sum
            .checked_sub(*relay_fee)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeRpcSumFailed)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

/// If signer seeds are not provided, invoking program is required.
/// If invoking program is provided signer seeds are required.
/// If signer seeds are provided, the derived signer has to match the signer.
#[inline(never)]
#[heap_neutral]
pub fn cpi_signer_check(
    signer_seeds: &Option<Vec<Vec<u8>>>,
    invoking_program: &Option<UncheckedAccount>,
    signer: &Pubkey,
) -> Result<()> {
    if signer_seeds.is_none() && invoking_program.is_some() {
        msg!("signer seeds not provided but trying to create compressed output account with data");
        return err!(crate::ErrorCode::SignerSeedsNotProvided);
    }
    if signer_seeds.is_some() {
        match invoking_program {
            Some(invoking_program_id) => {
                let seeds = match signer_seeds.as_ref() {
                    Some(seeds) => seeds.iter().map(|x| x.as_slice()).collect::<Vec<&[u8]>>(),
                    None => {
                        msg!("signer seeds not provided but trying to create compressed output account with data");
                        return err!(crate::ErrorCode::SignerSeedsNotProvided);
                    }
                };
                let derived_signer =
                    Pubkey::create_program_address(&seeds[..], &invoking_program_id.key())
                        .map_err(ProgramError::from)?;
                if derived_signer != *signer {
                    msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed derived cpi signer {} !=  signer {}",
                    signer,
                    signer
                );
                    msg!("seeds: {:?}", seeds);
                    return err!(crate::ErrorCode::SignerCheckFailed);
                }

                Ok(())
            }
            None => {
                msg!("invoking program id not provided but trying to create compressed output account with data");
                err!(crate::ErrorCode::InvokingProgramNotProvided)
            }
        }
    } else {
        Ok(())
    }
}

/// Checks the signer for input compressed accounts.
/// 1. If any compressed account has data the invoking program must be defined.
/// 2. If any compressed account has data the owner has to be the invokinging program.
/// 3. If no compressed account has data the owner has to be the signer.
#[inline(never)]
#[heap_neutral]
pub fn input_compressed_accounts_signer_check(
    inputs: &InstructionDataTransfer,
    invoking_program_id: &Option<UncheckedAccount>,
    signer: &Pubkey,
) -> Result<()> {
    inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(|compressed_account_with_context: &CompressedAccountWithMerkleContext| {

            if compressed_account_with_context.compressed_account.data.is_some()
            {
                // CHECK 1
                let invoking_program_id = match invoking_program_id {
                    Some(invoking_program_id) => Ok(invoking_program_id.key()),
                    None => {
                        msg!("invoking program id not provided but trying to create compressed output account with data");
                        err!(crate::ErrorCode::InvokingProgramNotProvided)
                    }
                }?;
                // CHECK 2
                if invoking_program_id != compressed_account_with_context.compressed_account.owner {
                    msg!(
                        "Signer/Program cannot read from an account it doesn't own. Read access check failed compressed account owner {} !=  invoking_program_id {}",
                        compressed_account_with_context.compressed_account.owner,
                        invoking_program_id
                    );
                    err!(crate::ErrorCode::SignerCheckFailed)
                } else {
                    Ok(())
                }
            }
            // CHECK 3
            else if compressed_account_with_context.compressed_account.owner != *signer {
                msg!(
                    "signer check failed compressed account owner {} !=  signer {}",
                    compressed_account_with_context.compressed_account.owner,
                    signer
                );
                err!(ErrorCode::SignerCheckFailed)
            } else {
                Ok(())
            }
        })?;
    Ok(())
}

/// Checks the write access for output compressed accounts.
/// Only program owned output accounts can hold data.
/// Every output account that holds data has to be owned by the invoking_program.
/// For every account that has data, the owner has to be the invoking_program.
// #[heap_neutral] //TODO: investigate why owned becomes mint when heap_neutral is used
pub fn output_compressed_accounts_write_access_check(
    inputs: &InstructionDataTransfer,
    invoking_program_id: &Option<UncheckedAccount>,
) -> Result<()> {
    // is triggered if one output account has data
    let output_account_with_data = inputs
        .output_compressed_accounts
        .iter()
        .filter(|compressed_account| compressed_account.data.is_some())
        .collect::<Vec<&CompressedAccount>>();
    if !output_account_with_data.is_empty() {
        // If a compressed account has data invoking_program has to be provided.
        let invoking_program_id = match invoking_program_id {
            Some(invoking_program_id) => Ok(invoking_program_id.key()),
            None => {
                msg!("invoking program id not provided but trying to create compressed output account with data");
                err!(crate::ErrorCode::InvokingProgramNotProvided)
            }
        }?;
        output_account_with_data.iter().try_for_each(|compressed_account| {
                if compressed_account.owner == invoking_program_id.key() {
                    Ok(())
                } else {
                    msg!(
                        "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {} !=  invoking_program_id {}",
                        compressed_account.owner,
                        invoking_program_id.key()
                    );

                    msg!("compressed_account: {:?}", compressed_account);
                    err!(crate::ErrorCode::WriteAccessCheckFailed)
                }
            })?;
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

pub fn check_program_owner_state_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<UncheckedAccount>,
) -> Result<()> {
    let merkle_tree =
        AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    // TODO: rename delegate to program_owner
    if merkle_tree_unpacked.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if invoking_program.key() == merkle_tree_unpacked.delegate {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
                    invoking_program.key(),
                    merkle_tree_unpacked.delegate
                );
                return Ok(());
            }
        }
        return Err(crate::ErrorCode::InvalidMerkleTreeOwner.into());
    }
    Ok(())
}

pub fn check_program_owner_address_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<UncheckedAccount>,
) -> Result<()> {
    let merkle_tree =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    // TODO: rename delegate to program_owner
    if merkle_tree_unpacked.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if invoking_program.key() == merkle_tree_unpacked.delegate {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
                    invoking_program.key(),
                    merkle_tree_unpacked.delegate
                );
                return Ok(());
            }
        }
        return Err(crate::ErrorCode::InvalidMerkleTreeOwner.into());
    }
    Ok(())
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext};

    #[test]
    fn test_sum_check_passes() {
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 150,
            address: None,
            data: None,
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
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 150,
            address: None,
            data: None,
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
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 100,
            address: None,
            data: None,
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
        let input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext> = vec![
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 0,
            },
            CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
                    lamports: 50,
                    address: None,
                    data: None,
                },
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 0,
                leaf_index: 1,
            },
        ];

        let output_compressed_account: Vec<CompressedAccount> = vec![CompressedAccount {
            owner: Pubkey::new_unique(),
            lamports: 25,
            address: None,
            data: None,
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
