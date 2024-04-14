use std::collections::HashMap;

use account_compression::StateMerkleTreeAccount;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::instructions::{InstructionDataTransfer, TransferInstruction};

#[heap_neutral]
pub fn insert_output_compressed_accounts_into_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    output_compressed_account_indices: &'a mut [u32],
    output_compressed_account_hashes: &'a mut [[u8; 32]],
    addresses: &'a mut Vec<Option<[u8; 32]>>,
) -> anchor_lang::Result<()> {
    let mut merkle_tree_indices = HashMap::<Pubkey, usize>::new();
    let mut out_merkle_trees_account_infos = Vec::<AccountInfo>::new();
    for (j, mt_index) in inputs
        .output_state_merkle_tree_account_indices
        .iter()
        .enumerate()
    {
        let index = merkle_tree_indices.get_mut(&ctx.remaining_accounts[*mt_index as usize].key());
        out_merkle_trees_account_infos.push(ctx.remaining_accounts[*mt_index as usize].clone());
        match index {
            Some(index) => {
                output_compressed_account_indices[j] = *index as u32;
                *index += 1;
            }
            None => {
                let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
                    &ctx.remaining_accounts[*mt_index as usize],
                )
                .unwrap();
                let merkle_tree = merkle_tree.load()?;
                let index = merkle_tree.load_next_index()?;
                merkle_tree_indices
                    .insert(ctx.remaining_accounts[*mt_index as usize].key(), index + 1);

                output_compressed_account_indices[j] = index as u32;
            }
        }
        // Address has to be created or a compressed account with this address has to be provided as transaction input.
        if let Some(address) = inputs.output_compressed_accounts[j].address {
            if let Some(position) = addresses.iter().position(|&x| x.unwrap() == address) {
                addresses.remove(position);
            } else {
                msg!("Address {:?}, has not been created and no compressed account with this address was provided as transaction input", address);
                msg!("Remaining addresses: {:?}", addresses);
                return Err(crate::ErrorCode::InvalidAddress.into());
            }
        }
        output_compressed_account_hashes[j] = inputs.output_compressed_accounts[j].hash(
            &ctx.remaining_accounts[*mt_index as usize].key(),
            &output_compressed_account_indices[j],
        )?;
    }

    append_leaves_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.psp_account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
        &ctx.accounts.noop_program,
        out_merkle_trees_account_infos,
        output_compressed_account_hashes.to_vec(),
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn append_leaves_cpi<'a, 'b>(
    program_id: &Pubkey,
    account_compression_program_id: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    registered_program_pda: &'b AccountInfo<'a>,
    log_wrapper: &'b AccountInfo<'a>,
    out_merkle_trees_account_infos: Vec<AccountInfo<'a>>,
    leaves: Vec<[u8; 32]>,
) -> Result<()> {
    let (seed, bump) = get_seeds(program_id, &authority.key())?;
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority", seed.as_slice(), bump][..]];

    let accounts = account_compression::cpi::accounts::AppendLeaves {
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
        log_wrapper: log_wrapper.to_account_info(),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts = out_merkle_trees_account_infos;
    account_compression::cpi::append_leaves_to_merkle_trees(cpi_ctx, leaves)?;
    Ok(())
}

#[inline(never)]
pub fn get_seeds<'a>(program_id: &'a Pubkey, cpi_signer: &'a Pubkey) -> Result<([u8; 32], u8)> {
    let seed = account_compression::ID.key().to_bytes();
    let (key, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[b"cpi_authority", seed.as_slice()],
        program_id,
    );
    assert_eq!(key, *cpi_signer);
    Ok((seed, bump))
}
