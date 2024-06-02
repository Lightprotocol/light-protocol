use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    errors::SystemProgramError,
    invoke_cpi::verify_signer::check_program_owner_state_merkle_tree,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        event::MerkleTreeSequenceNumber,
    },
    InstructionDataInvoke,
};
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Bumps};
use light_hasher::Poseidon;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_macros::heap_neutral;
use light_utils::hash_to_bn254_field_size_be;

#[allow(clippy::too_many_arguments)]
#[heap_neutral]
pub fn insert_output_compressed_accounts_into_state_merkle_tree<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    output_compressed_account_indices: &'a mut [u32],
    output_compressed_account_hashes: &'a mut [[u8; 32]],
    compressed_account_addresses: &'a mut Vec<Option<[u8; 32]>>,
    invoking_program: &Option<Pubkey>,
    hashed_pubkeys: &'a mut Vec<(Pubkey, [u8; 32])>,
    sequence_numbers: &'a mut Vec<MerkleTreeSequenceNumber>,
) -> Result<()> {
    bench_sbf_start!("cpda_append_data_init");
    let mut account_infos = vec![
        ctx.accounts.get_fee_payer().to_account_info(),
        ctx.accounts
            .get_account_compression_authority()
            .to_account_info(),
        ctx.accounts.get_registered_program_pda().to_account_info(),
        ctx.accounts.get_noop_program().to_account_info(),
        ctx.accounts.get_system_program().to_account_info(),
    ];
    let mut accounts = vec![
        AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[3].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta::new_readonly(account_infos[4].key(), false),
    ];

    let mut current_index: u8 = 0;
    let num_leaves = output_compressed_account_hashes.len();
    let mut num_leaves_in_tree: u32 = 0;
    let mut mt_next_index = 0;
    let vec_capacity = 12 + 33 * num_leaves;
    let mut instruction_data = Vec::<u8>::with_capacity(vec_capacity);
    let mut hashed_merkle_tree = [0u8; 32];
    // anchor instruction signature
    instruction_data.extend_from_slice(&[199, 144, 10, 82, 247, 142, 143, 7]);
    // leaves vector length (for borsh compat)
    instruction_data.extend_from_slice(&(num_leaves as u32).to_le_bytes());
    if inputs.output_compressed_accounts[0].merkle_tree_index == 0 {
        let account_info = ctx.remaining_accounts
            [inputs.output_compressed_accounts[current_index as usize].merkle_tree_index as usize]
            .to_account_info();
        let seq;
        (mt_next_index, _, seq) = check_program_owner_state_merkle_tree(
            &ctx.remaining_accounts[current_index as usize],
            invoking_program,
        )?;
        sequence_numbers.push(MerkleTreeSequenceNumber {
            pubkey: account_info.key(),
            seq,
        });
        hashed_merkle_tree = match hashed_pubkeys.iter().find(|x| x.0 == account_info.key()) {
            Some(hashed_merkle_tree) => hashed_merkle_tree.1,
            None => {
                // we do not insert here because Merkle trees are ordered and will not repeat.
                hash_to_bn254_field_size_be(&account_info.key().to_bytes())
                    .unwrap()
                    .0
            }
        };
        accounts.push(AccountMeta {
            pubkey: account_info.key(),
            is_signer: false,
            is_writable: true,
        });
        account_infos.push(account_info);
    }
    bench_sbf_end!("cpda_append_data_init");
    bench_sbf_start!("cpda_append_rest");

    for (j, account) in inputs.output_compressed_accounts.iter().enumerate() {
        // if mt index == current index Merkle tree account info has already been added.
        // if mt index != current index, Merkle tree account info is new, add it.
        #[allow(clippy::comparison_chain)]
        if account.merkle_tree_index == current_index {
            // Do nothing, but is the most common case assuming ordered Merkle trees.
        } else if account.merkle_tree_index != current_index {
            current_index = account.merkle_tree_index;
            let seq;
            (mt_next_index, _, seq) = check_program_owner_state_merkle_tree(
                &ctx.remaining_accounts[account.merkle_tree_index as usize],
                invoking_program,
            )?;
            let account_info =
                ctx.remaining_accounts[account.merkle_tree_index as usize].to_account_info();
            accounts.push(AccountMeta {
                pubkey: account_info.key(),
                is_signer: false,
                is_writable: true,
            });
            sequence_numbers.push(MerkleTreeSequenceNumber {
                pubkey: account_info.key(),
                seq,
            });
            hashed_merkle_tree = match hashed_pubkeys.iter().find(|x| x.0 == account_info.key()) {
                Some(hashed_merkle_tree) => hashed_merkle_tree.1,
                None => {
                    hash_to_bn254_field_size_be(&account_info.key().to_bytes())
                        .unwrap()
                        .0
                }
            };
            account_infos.push(account_info);
            num_leaves_in_tree = 0;
        }

        if let Some(address) = account.compressed_account.address {
            if let Some(position) = compressed_account_addresses
                .iter()
                .filter(|x| x.is_some())
                .position(|&x| x.unwrap() == address)
            {
                compressed_account_addresses.remove(position);
            } else {
                msg!("Address {:?}, is no new address and does not exist in input compressed accounts.", address);
                msg!(
                    "Remaining compressed_account_addresses: {:?}",
                    compressed_account_addresses
                );
                return Err(SystemProgramError::InvalidAddress.into());
            }
        }

        output_compressed_account_indices[j] = mt_next_index + num_leaves_in_tree;
        num_leaves_in_tree += 1;
        if account.compressed_account.data.is_some() && invoking_program.is_none() {
            msg!("Invoking program is not provided.");
            msg!("Only program owned compressed accounts can have data.");
            return err!(SystemProgramError::InvokingProgramNotProvided);
        }
        let hashed_owner = match hashed_pubkeys
            .iter()
            .find(|x| x.0 == account.compressed_account.owner)
        {
            Some(hashed_owner) => hashed_owner.1,
            None => {
                let hashed_owner =
                    hash_to_bn254_field_size_be(&account.compressed_account.owner.to_bytes())
                        .unwrap()
                        .0;
                hashed_pubkeys.push((account.compressed_account.owner, hashed_owner));
                hashed_owner
            }
        };
        // Compute output compressed account hash.
        output_compressed_account_hashes[j] = account
            .compressed_account
            .hash_with_hashed_values::<Poseidon>(
                &hashed_owner,
                &hashed_merkle_tree,
                &output_compressed_account_indices[j],
            )?;
        instruction_data.extend_from_slice(&[(account_infos.len() - 6) as u8]);
        instruction_data.extend_from_slice(&output_compressed_account_hashes[j]);
    }

    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let seeds = &[&[CPI_AUTHORITY_PDA_SEED, bump][..]];
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: account_compression::ID,
        accounts,
        data: instruction_data,
    };
    invoke_signed(&instruction, account_infos.as_slice(), seeds)?;
    bench_sbf_end!("cpda_append_rest");

    Ok(())
}

#[test]
fn test_instruction_data_borsh_compat() {
    let mut vec = Vec::<u8>::new();
    vec.extend_from_slice(&2u32.to_le_bytes());
    vec.push(1);
    vec.extend_from_slice(&[2u8; 32]);
    vec.push(3);
    vec.extend_from_slice(&[4u8; 32]);
    let refe = vec![(1, [2u8; 32]), (3, [4u8; 32])];
    let mut serialized = Vec::new();
    Vec::<(u8, [u8; 32])>::serialize(&refe, &mut serialized).unwrap();
    assert_eq!(serialized, vec);
    let res = Vec::<(u8, [u8; 32])>::deserialize(&mut vec.as_slice()).unwrap();
    assert_eq!(res, vec![(1, [2u8; 32]), (3, [4u8; 32])]);
}
