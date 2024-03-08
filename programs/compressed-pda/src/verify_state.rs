use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;

#[cfg(test)]
use crate::utxo::OutUtxo;
use crate::{
    instructions::{InstructionDataTransfer, TransferInstruction},
    utxo::{InUtxoTuple, OutUtxoTuple},
    ErrorCode,
};

pub fn fetch_roots<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (j, in_utxo_tuple) in inputs.in_utxos.iter().enumerate() {
        let merkle_tree = AccountLoader::<StateMerkleTreeAccount>::try_from(
            &ctx.remaining_accounts[in_utxo_tuple.index_mt_account as usize],
        )
        .unwrap();
        let merkle_tree = merkle_tree.load()?;
        let fetched_roots = merkle_tree.load_roots()?;

        roots[j] = fetched_roots[inputs.root_indices[j] as usize];
    }
    Ok(())
}
pub fn hash_in_utxos<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    leaves: &'a mut [[u8; 32]],
) -> anchor_lang::Result<()> {
    for (j, in_utxo_tuple) in inputs.in_utxos.iter().enumerate() {
        leaves[j] = in_utxo_tuple.in_utxo.hash();
    }
    Ok(())
}

pub fn sum_check(
    in_utxos: &[InUtxoTuple],
    out_utxos: &[OutUtxoTuple],
    relay_fee: &Option<u64>,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for utxo_tuple in in_utxos.iter() {
        sum = sum
            .checked_add(utxo_tuple.in_utxo.lamports)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    for utxo_tuple in out_utxos.iter() {
        sum = sum
            .checked_sub(utxo_tuple.out_utxo.lamports)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::utxo::{InUtxoTuple, Utxo};

    #[test]
    fn test_sum_check_passes() {
        let in_utxos: Vec<InUtxoTuple> = vec![
            InUtxoTuple {
                in_utxo: Utxo {
                    owner: Pubkey::new_unique(),
                    blinding: [0; 32],
                    lamports: 100,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
            },
            InUtxoTuple {
                in_utxo: Utxo {
                    owner: Pubkey::new_unique(),
                    blinding: [0; 32],
                    lamports: 50,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
            },
        ];

        let out_utxos: Vec<OutUtxoTuple> = vec![OutUtxoTuple {
            out_utxo: OutUtxo {
                owner: Pubkey::new_unique(),
                lamports: 150,
                data: None,
            },
            index_mt_account: 0,
        }];

        let relay_fee = None; // No RPC fee

        let result = sum_check(&in_utxos, &out_utxos, &relay_fee);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sum_check_fails() {
        let in_utxos: Vec<InUtxoTuple> = vec![
            InUtxoTuple {
                in_utxo: Utxo {
                    owner: Pubkey::new_unique(),
                    blinding: [0; 32],
                    lamports: 150,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
            },
            InUtxoTuple {
                in_utxo: Utxo {
                    owner: Pubkey::new_unique(),
                    blinding: [0; 32],
                    lamports: 50,
                    data: None,
                },
                index_mt_account: 0,
                index_nullifier_array_account: 0,
            },
        ];

        let out_utxos: [OutUtxoTuple; 1] = [OutUtxoTuple {
            out_utxo: OutUtxo {
                owner: Pubkey::new_unique(),
                lamports: 100,
                data: None,
            },
            index_mt_account: 0,
        }];

        let relay_fee = Some(50); // Adding an RPC fee to ensure the sums don't match

        let result = sum_check(&in_utxos, &out_utxos, &relay_fee);
        assert!(result.is_err());
    }
}
