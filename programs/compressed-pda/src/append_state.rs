use std::collections::HashMap;

use account_compression::StateMerkleTreeAccount;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    instructions::{InstructionDataTransfer, TransferInstruction},
    utxo::Utxo,
};

pub fn insert_out_utxos<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    out_utxos: &'a mut [Utxo],
    out_utxo_indices: &'a mut [u32],
) -> anchor_lang::Result<()> {
    let mut merkle_tree_indices = HashMap::<Pubkey, usize>::new();
    let mut leaves: Vec<[u8; 32]> = Vec::with_capacity(inputs.out_utxos.len());
    let mut out_merkle_trees_account_infos = Vec::<AccountInfo>::new();
    for (j, out_utxo_tuple) in inputs.out_utxos.iter().enumerate() {
        let index = merkle_tree_indices
            .get_mut(&ctx.remaining_accounts[out_utxo_tuple.index_mt_account as usize].key());
        out_merkle_trees_account_infos
            .push(ctx.remaining_accounts[out_utxo_tuple.index_mt_account as usize].clone());
        match index {
            Some(index) => {
                out_utxo_indices[j] = *index as u32;
                *index += 1;
            }
            None => {
                let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
                    &ctx.remaining_accounts[out_utxo_tuple.index_mt_account as usize],
                )
                .unwrap();
                let merkle_tree = merkle_tree.load()?;
                let index = merkle_tree.load_next_index()?;
                merkle_tree_indices.insert(
                    ctx.remaining_accounts[out_utxo_tuple.index_mt_account as usize].key(),
                    index + 1,
                );

                out_utxo_indices[j] = index as u32;
            }
        }
        let mut utxo = Utxo {
            owner: out_utxo_tuple.out_utxo.owner,
            blinding: [0u8; 32],
            lamports: out_utxo_tuple.out_utxo.lamports,
            address: None,
            data: out_utxo_tuple.out_utxo.data.clone(),
        };
        utxo.update_blinding(
            ctx.remaining_accounts[out_utxo_tuple.index_mt_account as usize].key(),
            out_utxo_indices[j] as usize,
        )
        .unwrap();
        leaves.push(utxo.hash());
        out_utxos[j] = utxo;
    }

    insert_two_leaves_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.psp_account_compression_authority,
        &ctx.accounts.registered_program_pda,
        &ctx.accounts.noop_program,
        out_merkle_trees_account_infos,
        leaves,
    )
}
#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
#[inline(never)]
pub fn insert_two_leaves_cpi<'a, 'b>(
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

    let accounts = account_compression::cpi::accounts::InsertTwoLeavesParallel {
        authority: authority.to_account_info(),
        registered_program_pda: Some(registered_program_pda.to_account_info()),
        log_wrapper: log_wrapper.to_account_info(),
    };

    let mut cpi_ctx =
        CpiContext::new_with_signer(account_compression_program_id.clone(), accounts, seeds);
    cpi_ctx.remaining_accounts = out_merkle_trees_account_infos;
    account_compression::cpi::insert_leaves_into_merkle_trees(cpi_ctx, leaves)?;
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
