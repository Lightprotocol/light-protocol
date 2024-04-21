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

#[inline(never)]
#[heap_neutral]
pub fn signer_check(
    inputs: &InstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction<'_>>,
) -> Result<()> {
    inputs
    .input_compressed_accounts_with_merkle_context
    .iter()
    .try_for_each(|compressed_accounts: &CompressedAccountWithMerkleContext| {
        // TODO(@ananas-block): revisit program signer check
        // Two options: (1 is currently implemented)
        // 1. we require the program as an account and reconstruct the cpi signer to check that the cpi signer is a pda of the program
        //   - The advantage is that the compressed account can be owned by the program_id
        // 2. we set a deterministic pda signer for every program eg seeds = [b"cpi_authority"]
        //   - The advantages are that the program does not need to be an account, and we don't need to reconstruct the pda -> more efficient (costs are just low hundreds of cu though)
        //   - The drawback is that the pda signer is the owner of the compressed account which is confusing
        if compressed_accounts.compressed_account.data.is_some() {
            let invoking_program_id = ctx.accounts.invoking_program.as_ref().unwrap().key();
            let signer = Pubkey::create_program_address(
                &inputs.signer_seeds.as_ref().unwrap().iter().map(|x|x.as_slice()).collect::<Vec::<&[u8]>>()[..],
                &invoking_program_id,
            ).map_err(ProgramError::from)?;
            if signer != ctx.accounts.authority.key()
                && invoking_program_id != compressed_accounts.compressed_account.owner
            {
                Ok(())
            } else {
                msg!(
                    "program signer check failed derived cpi signer {} !=  signer {}",
                    compressed_accounts.compressed_account.owner,
                    ctx.accounts.authority.key()
                );
                msg!(
                    "program signer check failed compressed account owner {} !=  invoking_program_id {}",
                    compressed_accounts.compressed_account.owner,
                    invoking_program_id
                );
                err!(ErrorCode::SignerCheckFailed)

            }
        } else if compressed_accounts.compressed_account.owner != ctx.accounts.authority.key()
        {
            Ok(())
        } else {
            msg!(
                "signer check failed compressed account owner {} !=  signer {}",
                compressed_accounts.compressed_account.owner,
                ctx.accounts.authority.key()
            );
            err!(ErrorCode::SignerCheckFailed)
        }
    })?;
    Ok(())
}

/// Checks the write access for output compressed accounts.
/// Only program owned output accounts can hold data.
/// Every output account that holds data has to be owned by the invoking_program.
/// For every account that has data, the owner has to be the invoking_program.
// #[heap_neutral] //TODO: investigate why owned becomes mint when heap_neutral is used
pub fn write_access_check(
    inputs: &InstructionDataTransfer,
    invoking_program_id: &Option<UncheckedAccount>,
    signer: &Pubkey,
) -> Result<()> {
    // is triggered if one output account has data
    let output_account_with_data = inputs
        .output_compressed_accounts
        .iter()
        .filter(|compressed_account| compressed_account.data.is_some())
        .collect::<Vec<&CompressedAccount>>();
    if !output_account_with_data.is_empty() {
        match invoking_program_id {
            Some(invoking_program_id) => {
                let seeds = match inputs.signer_seeds.as_ref() {
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
