use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use borsh::BorshDeserialize;
use utxo::{OutUtxo, SerializedUtxos, TlvDataElement, Utxo};
pub mod utxo;
declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

#[error_code]
pub enum ErrorCode {
    #[msg("Sum check failed")]
    SumCheckFailed,
}

#[program]
pub mod psp_compressed_pda {

    use super::*;

    /// This function can be used to transfer sol and execute any other compressed transaction.
    /// Instruction data is not optimized for space.
    /// This method can be called by cpi so that instruction data can be compressed with a custom algorithm.
    pub fn execute_compressed_transaction(
        _ctx: Context<TransferInstruction>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let _inputs: InstructionDataTransfer = InstructionDataTransfer::try_deserialize_unchecked(
            &mut [vec![0u8; 8], inputs].concat().as_slice(),
        )?;
        // let (merkle_tree_indices, root_indices) = fetch_out_utxo_index(
        //     inputs.out_utxos.len(),
        //     &ctx.remaining_accounts
        //         [inputs.in_utxos.len() * 2..inputs.in_utxos.len() * 2 + inputs.out_utxos.len()],
        // )?;
        // let out_utxos: Vec<Utxo> = merkle_tree_indices
        //     .iter()
        //     .map(|(pubkey, i)| {
        //         let mut utxo = Utxo {
        //             owner: inputs.out_utxos[*i].owner,
        //             blinding: [0u8; 32],
        //             lamports: inputs.out_utxos[*i].lamports,
        //             data: inputs.out_utxos[*i].data.clone(),
        //         };
        //         utxo.update_blinding(*pubkey, root_indices[*i] as usize)
        //             .unwrap();
        //         utxo
        //     })
        //     .collect();
        // // // sum check
        // sum_check(inputs.in_utxos, inputs.out_utxos, inputs.rpc_fee)?;
        // check cpi signatures if account is defined
        // verify proof of inclusion of in utxo hashes
        // insert nullifiers (in utxo hashes)
        // insert leaves (out utxo hashes)

        Ok(())
    }

    /// This function can be used to transfer sol and execute any other compressed transaction.
    /// Instruction data is optimized for space.
    pub fn execute_compressed_transaction2(
        _ctx: Context<TransferInstruction>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let _inputs: InstructionDataTransfer2 =
            InstructionDataTransfer2::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;
        // let in_utxos = inputs.utxos.in_utxos_from_serialized_utxos(
        //     &ctx.accounts
        //         .to_account_infos()
        //         .iter()
        //         .map(|a| a.key())
        //         .collect::<Vec<Pubkey>>(),
        //     &ctx.remaining_accounts[..inputs.utxos.in_utxos.len()]
        //         .iter()
        //         .map(|a| a.key())
        //         .collect::<Vec<Pubkey>>(),
        // );
        // let (_, root_indices) = fetch_out_utxo_index(
        //     inputs.utxos.out_utxos.len(),
        //     &ctx.remaining_accounts[inputs.utxos.in_utxos.len() * 2
        //         ..inputs.utxos.in_utxos.len() * 2 + inputs.utxos.out_utxos.len()],
        // )?;

        // let out_utxos = inputs.utxos.out_utxos_from_serialized_utxos(
        //     &ctx.accounts
        //         .to_account_infos()
        //         .iter()
        //         .map(|a| a.key())
        //         .collect::<Vec<Pubkey>>(),
        //     &ctx.remaining_accounts[inputs.utxos.in_utxos.len() * 2..]
        //         .iter()
        //         .map(|a| a.key())
        //         .collect::<Vec<Pubkey>>(),
        //     &root_indices,
        // );
        // sum_check(in_utxos, out_utxos, inputs.rpc_fee)?;
        // check cpi signatures if account is defined
        // verify proof of inclusion of in utxo hashes
        // insert nullifiers (in utxo hashes)
        // insert leaves (out utxo hashes)
        Ok(())
    }

    // TODO: add compress and decompress sol as a wrapper around process_execute_compressed_transaction

    // TODO: add create_pda as a wrapper around process_execute_compressed_transaction
}

pub fn sum_check(
    in_utxos: Vec<Utxo>,
    out_utxos: Vec<Utxo>,
    rpc_fee: Option<u64>,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for utxo in in_utxos.iter() {
        sum = sum
            .checked_add(utxo.lamports)
            .ok_or(ProgramError::InvalidAccountData)?;
    }

    for utxo in out_utxos.iter() {
        println!("utxo.lamports {}", utxo.lamports);
        sum = sum
            .checked_sub(utxo.lamports)
            .ok_or(ProgramError::InvalidAccountData)?;
    }

    if let Some(rpc_fee) = rpc_fee {
        sum = sum
            .checked_sub(rpc_fee)
            .ok_or(ProgramError::InvalidAccountData)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

// TODO: pass the information in which Merkle tree which utxo is as instruction data
// #[inline(never)]
// pub fn fetch_out_utxo_index(
//     number_out_utxos: usize,
//     remaining_accounts: &[AccountInfo],
// ) -> Result<(HashMap<Pubkey, usize>, Vec<u32>)> {
//     let mut merkle_tree_indices = HashMap::<Pubkey, usize>::new();
//     let mut out_utxo_index: Vec<u32> = Vec::new();
//     for i in 0..number_out_utxos {
//         let index = merkle_tree_indices.get_mut(&remaining_accounts[i].key());
//         match index {
//             Some(index) => {
//                 out_utxo_index.push(*index as u32);
//             }
//             None => {
//                 let merkle_tree =
//                     AccountLoader::<ConcurrentMerkleTreeAccount>::try_from(&remaining_accounts[i])
//                         .unwrap();
//                 let merkle_tree_account = merkle_tree.load()?;
//                 let merkle_tree =
//                     state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);
//                 let index = merkle_tree.next_index as usize;
//                 merkle_tree_indices.insert(remaining_accounts[i].key(), index);

//                 out_utxo_index.push(index as u64);
//             }
//         }
//     }
//     Ok((merkle_tree_indices, out_utxo_index))
// }
/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each in utxo one queue and Merkle tree account each for each out utxo.
#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// Check that mint authority is derived from signer
    // #[account(mut, seeds = [b"authority", authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub authority_pda: UncheckedAccount<'info>,
    /// CHECK this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK this account
    pub noop_program: UncheckedAccount<'info>,
    pub compressed_pda_program: UncheckedAccount<'info>, // Program<'info, psp_compressed_pda::program::CompressedPda>,
    /// CHECK this account in psp account compression program
    #[account(mut)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK this account in psp account compression program
    pub account_compression_program: UncheckedAccount<'info>,
    pub cpi_signature_account: Option<Account<'info, CpiSignatureAccount>>,
}

#[account]
pub struct CpiSignatureAccount {
    pub signatures: Vec<CpiSignature>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiSignature {
    pub program: Pubkey,
    pub tlv_hash: [u8; 32],
    pub tlv_data: TlvDataElement,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    low_element_indices: Vec<u16>,
    root_indices: Vec<u64>,
    rpc_fee: Option<u64>,
    in_utxos: Vec<Utxo>,
    out_utxos: Vec<OutUtxo>,
    in_utxo_merkle_tree_remaining_account_index: Vec<u8>,
    in_utxo_nullifier_queue_remaining_account_index: Vec<u8>,
    out_utxo_merkle_tree_remaining_account_index: Vec<u8>,
}

// TODO: parse utxos a more efficient way, since owner is sent multiple times this way
#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer2 {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    low_element_indices: Vec<u16>,
    root_indices: Vec<u64>,
    rpc_fee: Option<u64>,
    utxos: SerializedUtxos,
    in_utxo_merkle_tree_remaining_account_index: Vec<u8>,
    in_utxo_nullifier_queue_remaining_account_index: Vec<u8>,
    out_utxo_merkle_tree_remaining_account_index: Vec<u8>,
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::utxo::Utxo;

    #[test]
    fn test_sum_check_passes() {
        let in_utxos = vec![
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 100,
                data: None,
            },
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 50,
                data: None,
            },
        ];

        let out_utxos = vec![Utxo {
            owner: Pubkey::new_unique(),
            lamports: 150,
            blinding: [0; 32],
            data: None,
        }];

        let rpc_fee = None; // No RPC fee

        let result = sum_check(in_utxos, out_utxos, rpc_fee);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sum_check_fails() {
        let in_utxos = vec![
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 200,
                data: None,
            },
            Utxo {
                owner: Pubkey::new_unique(),
                blinding: [0; 32],
                lamports: 50,
                data: None,
            },
        ];

        let out_utxos = vec![Utxo {
            owner: Pubkey::new_unique(),
            blinding: [0; 32],
            lamports: 100,
            data: None,
        }];

        let rpc_fee = Some(50); // Adding an RPC fee to ensure the sums don't match

        let result = sum_check(in_utxos, out_utxos, rpc_fee);
        assert!(result.is_err());
    }
}
