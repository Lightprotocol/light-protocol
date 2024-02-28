use std::collections::HashMap;

use account_compression::{state_merkle_tree_from_bytes, StateMerkleTreeAccount};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    instructions::{InstructionDataTransfer, TransferInstruction},
    utxo::{OutUtxo, Utxo},
    ErrorCode,
};
pub fn fetch_roots<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, (_, in_merkle_tree_index, _)) in inputs.in_utxos.iter().enumerate() {
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts[*in_merkle_tree_index as usize],
        )
        .unwrap();
        let merkle_tree_account = merkle_tree.load()?;
        let merkle_tree = state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);

        roots[j] = merkle_tree.roots[inputs.root_indices[j] as usize];
    }
    Ok(())
}
pub fn hash_in_utxos<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    leaves: &'a mut [[u8; 32]],
) -> anchor_lang::Result<()> {
    for (j, (utxo, _, _)) in inputs.in_utxos.iter().enumerate() {
        leaves[j] = utxo.hash();
    }
    Ok(())
}

pub fn out_utxos_to_utxos<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    utxos: &'a mut [Utxo],
    out_utxo_index: &'a mut [u32],
) -> anchor_lang::Result<()> {
    let mut merkle_tree_indices = HashMap::<Pubkey, usize>::new();
    let mut out_merkle_trees_account_infos = Vec::<AccountInfo>::new();
    for (j, (out_utxo, i)) in inputs.out_utxos.iter().enumerate() {
        let index = merkle_tree_indices.get_mut(&ctx.remaining_accounts[*i as usize].key());
        out_merkle_trees_account_infos.push(ctx.remaining_accounts[*i as usize].clone());
        match index {
            Some(index) => {
                out_utxo_index[j] = *index as u32;
            }
            None => {
                let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
                    &ctx.remaining_accounts[*i as usize],
                )
                .unwrap();
                let merkle_tree_account = merkle_tree.load()?;
                let merkle_tree =
                    state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);
                let index = merkle_tree.next_index as usize;
                merkle_tree_indices.insert(ctx.remaining_accounts[*i as usize].key(), index);

                out_utxo_index[j] = index as u32;
            }
        }
        let mut utxo = Utxo {
            owner: out_utxo.owner,
            blinding: [0u8; 32],
            lamports: out_utxo.lamports,
            data: out_utxo.data.clone(),
        };
        utxo.update_blinding(
            ctx.remaining_accounts[*i as usize].key(),
            out_utxo_index[j] as usize,
        )
        .unwrap();
        utxos[j] = utxo;
    }
    Ok(())
}

pub fn sum_check(
    in_utxos: &[(Utxo, u8, u8)],
    out_utxos: &[(OutUtxo, u8)],
    rpc_fee: &Option<u64>,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for utxo in in_utxos.iter() {
        sum = sum
            .checked_add(utxo.0.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    for utxo in out_utxos.iter() {
        sum = sum
            .checked_sub(utxo.0.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
    }

    if let Some(rpc_fee) = rpc_fee {
        sum = sum
            .checked_sub(*rpc_fee)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeRpcSumFailed)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

#[test]
fn test_sum_check_passes() {
    let in_utxos = vec![
        (
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 100,
                data: None,
            },
            0,
            0,
        ),
        (
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 50,
                data: None,
            },
            0,
            0,
        ),
    ];

    let out_utxos = vec![(
        OutUtxo {
            owner: Pubkey::new_unique(),
            lamports: 150,
            data: None,
        },
        0,
    )];

    let rpc_fee = None; // No RPC fee

    let result = sum_check(&in_utxos, &out_utxos, &rpc_fee);
    assert!(result.is_ok());
}

#[test]
fn test_sum_check_fails() {
    let in_utxos = vec![
        (
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 150,
                data: None,
            },
            0,
            0,
        ),
        (
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 50,
                data: None,
            },
            0,
            0,
        ),
    ];

    let out_utxos = vec![(
        OutUtxo {
            owner: Pubkey::new_unique(),
            lamports: 100,
            data: None,
        },
        0,
    )];

    let rpc_fee = Some(50); // Adding an RPC fee to ensure the sums don't match

    let result = sum_check(&in_utxos, &out_utxos, &rpc_fee);
    assert!(result.is_err());
}
