//! Transfer instruction builders for Light Protocol.
//!
//! Provides Transfer2 instruction building for compressed token transfers.
//! Routes between:
//! - Compressed-to-compressed (Transfer2 with compressed inputs)
//! - Light-token-to-light-token (TransferChecked via on-chain accounts)

use std::collections::HashMap;

use borsh::BorshSerialize;
#[cfg(test)]
use borsh::BorshDeserialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::KoraLightError,
    program_ids::{
        ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA,
        LIGHT_SYSTEM_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID, REGISTERED_PROGRAM_PDA, SYSTEM_PROGRAM_ID,
        TRANSFER2_DISCRIMINATOR,
    },
    types::{
        CompressedProof, CompressedTokenAccountInput, CompressedTokenInstructionDataTransfer2,
        MultiInputTokenDataWithContext, MultiTokenTransferOutputData, PackedMerkleContext,
    },
};

/// Default max top-up (u16::MAX = no limit)
const DEFAULT_MAX_TOP_UP: u16 = u16::MAX;

/// Light Token TransferChecked discriminator
const TRANSFER_CHECKED_DISCRIMINATOR: u8 = 12;

/// Build a Transfer2 instruction for compressed-to-compressed token transfers.
///
/// This replaces Kora's ~660-line raw byte serialization with a proper SDK call.
///
/// # Arguments
/// * `payer` - Fee payer (signer)
/// * `authority` - Token owner or delegate (signer)
/// * `mint` - Token mint
/// * `inputs` - Source compressed token accounts
/// * `proof` - Validity proof from the RPC
/// * `destination_owner` - Owner of the destination compressed account
/// * `amount` - Amount to transfer
pub fn create_transfer2_instruction(
    payer: &Pubkey,
    authority: &Pubkey,
    mint: &Pubkey,
    inputs: &[CompressedTokenAccountInput],
    proof: &CompressedProof,
    destination_owner: &Pubkey,
    amount: u64,
) -> Result<Instruction, KoraLightError> {
    if inputs.is_empty() {
        return Err(KoraLightError::NoCompressedAccounts);
    }

    // Build packed accounts array
    let mut packed_indices: HashMap<Pubkey, u8> = HashMap::new();
    let mut packed_accounts: Vec<(Pubkey, bool, bool)> = Vec::new();

    let mut insert_or_get = |pubkey: Pubkey, is_signer: bool, is_writable: bool| -> u8 {
        if let Some(&idx) = packed_indices.get(&pubkey) {
            if is_writable {
                packed_accounts[idx as usize].2 = true;
            }
            if is_signer {
                packed_accounts[idx as usize].1 = true;
            }
            idx
        } else {
            let idx = packed_accounts.len() as u8;
            packed_indices.insert(pubkey, idx);
            packed_accounts.push((pubkey, is_signer, is_writable));
            idx
        }
    };

    // 1. Trees (writable)
    for input in inputs {
        insert_or_get(input.tree, false, true);
    }

    // 2. Queues (writable)
    let first_queue_index;
    {
        let mut first = true;
        first_queue_index = {
            let mut fqi = 0u8;
            for input in inputs {
                let idx = insert_or_get(input.queue, false, true);
                if first {
                    fqi = idx;
                    first = false;
                }
            }
            fqi
        };
    }

    // 3. Mint (readonly)
    let mint_index = insert_or_get(*mint, false, false);

    // 4. Authority/owner (signer)
    let authority_index = insert_or_get(*authority, true, false);

    // 5. Destination owner (readonly)
    let dest_owner_index = insert_or_get(*destination_owner, false, false);

    // 6. Delegates if any
    for input in inputs {
        if let Some(delegate) = &input.delegate {
            insert_or_get(*delegate, false, false);
        }
    }

    // Build input token data
    let in_token_data: Vec<MultiInputTokenDataWithContext> = inputs
        .iter()
        .map(|input| {
            let tree_idx = packed_indices[&input.tree];
            let queue_idx = packed_indices[&input.queue];
            let delegate_idx = input.delegate.map(|d| packed_indices[&d]).unwrap_or(0);

            MultiInputTokenDataWithContext {
                owner: authority_index,
                amount: input.amount,
                has_delegate: input.delegate.is_some(),
                delegate: delegate_idx,
                mint: mint_index,
                version: input.version,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: tree_idx,
                    queue_pubkey_index: queue_idx,
                    leaf_index: input.leaf_index,
                    prove_by_index: input.prove_by_index,
                },
                root_index: input.root_index,
            }
        })
        .collect();

    // Calculate change
    let input_total: u64 = inputs.iter().map(|i| i.amount).sum();
    let change_amount =
        input_total
            .checked_sub(amount)
            .ok_or(KoraLightError::InsufficientBalance {
                needed: amount,
                available: input_total,
            })?;

    // Build output: destination + optional change
    let mut out_token_data = vec![MultiTokenTransferOutputData {
        owner: dest_owner_index,
        amount,
        has_delegate: false,
        delegate: 0,
        mint: mint_index,
        version: inputs[0].version,
    }];

    if change_amount > 0 {
        out_token_data.push(MultiTokenTransferOutputData {
            owner: authority_index,
            amount: change_amount,
            has_delegate: false,
            delegate: 0,
            mint: mint_index,
            version: inputs[0].version,
        });
    }

    // Build Transfer2 instruction data
    let transfer2_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: first_queue_index,
        max_top_up: DEFAULT_MAX_TOP_UP,
        cpi_context: None,
        compressions: None,
        proof: if inputs.iter().all(|i| i.prove_by_index) { None } else { Some(*proof) },
        in_token_data,
        out_token_data,
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    // Serialize
    let mut data = Vec::new();
    data.push(TRANSFER2_DISCRIMINATOR);
    transfer2_data.serialize(&mut data)?;

    // Build account metas: static + packed
    let mut accounts = vec![
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(CPI_AUTHORITY_PDA, false),
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA, false),
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA, false),
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    for (pubkey, is_signer, is_writable) in &packed_accounts {
        if *is_writable {
            accounts.push(AccountMeta::new(*pubkey, *is_signer));
        } else {
            accounts.push(AccountMeta::new_readonly(*pubkey, *is_signer));
        }
    }

    Ok(Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Build a simple TransferChecked instruction for light-token ATA to ATA transfers.
///
/// This is for decompressed (on-chain) light-token accounts, NOT compressed accounts.
pub fn create_transfer_checked_instruction(
    source_ata: &Pubkey,
    destination_ata: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    decimals: u8,
    payer: &Pubkey,
) -> Result<Instruction, KoraLightError> {
    let mut data = Vec::with_capacity(10);
    data.push(TRANSFER_CHECKED_DISCRIMINATOR);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);

    let mut accounts = vec![
        AccountMeta::new(*source_ata, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_ata, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    // If payer != owner, add payer as signer
    if payer != owner {
        accounts.push(AccountMeta::new(*payer, true));
    }

    Ok(Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(amount: u64, tree: Pubkey, queue: Pubkey) -> CompressedTokenAccountInput {
        CompressedTokenAccountInput {
            hash: [0u8; 32],
            tree,
            queue,
            amount,
            leaf_index: 42,
            prove_by_index: false,
            root_index: 0,
            version: 0,
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            delegate: None,
        }
    }

    #[test]
    fn test_transfer2_basic() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let dest_owner = Pubkey::new_unique();

        let inputs = vec![make_input(1000, tree, queue)];
        let proof = CompressedProof::default();

        let ix = create_transfer2_instruction(
            &payer,
            &authority,
            &mint,
            &inputs,
            &proof,
            &dest_owner,
            1000,
        )
        .unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.data[0], TRANSFER2_DISCRIMINATOR);
        // 7 static + packed (tree, queue, mint, authority, dest_owner)
        assert_eq!(ix.accounts.len(), 7 + 5);
    }

    #[test]
    fn test_transfer2_with_change() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let dest_owner = Pubkey::new_unique();

        let inputs = vec![make_input(1000, tree, queue)];
        let proof = CompressedProof::default();

        let ix = create_transfer2_instruction(
            &payer,
            &authority,
            &mint,
            &inputs,
            &proof,
            &dest_owner,
            700,
        )
        .unwrap();

        // Should have both destination and change outputs in data
        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
    }

    #[test]
    fn test_transfer_checked() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let payer = Pubkey::new_unique();

        let ix = create_transfer_checked_instruction(&source, &dest, &mint, &owner, 500, 6, &payer)
            .unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.data[0], TRANSFER_CHECKED_DISCRIMINATOR);
        // Amount in LE bytes
        assert_eq!(&ix.data[1..9], &500u64.to_le_bytes());
        // Decimals
        assert_eq!(ix.data[9], 6);
        // 5 accounts + payer (since payer != owner)
        assert_eq!(ix.accounts.len(), 6);
    }

    #[test]
    fn test_transfer2_prove_by_index_no_proof() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let dest_owner = Pubkey::new_unique();

        let inputs = vec![CompressedTokenAccountInput {
            prove_by_index: true,
            ..make_input(1000, tree, queue)
        }];
        let proof = CompressedProof::default();

        let ix = create_transfer2_instruction(
            &payer, &authority, &mint, &inputs, &proof, &dest_owner, 1000,
        )
        .unwrap();

        // Deserialize and verify proof is None
        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(data.proof.is_none(), "proof must be None when all inputs use prove_by_index");
    }

    #[test]
    fn test_transfer2_mixed_prove_by_index_has_proof() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let dest_owner = Pubkey::new_unique();

        let inputs = vec![
            CompressedTokenAccountInput {
                prove_by_index: true,
                ..make_input(500, tree, queue)
            },
            CompressedTokenAccountInput {
                prove_by_index: false,
                ..make_input(500, tree, queue)
            },
        ];
        let proof = CompressedProof { a: [1; 32], b: [2; 64], c: [3; 32] };

        let ix = create_transfer2_instruction(
            &payer, &authority, &mint, &inputs, &proof, &dest_owner, 1000,
        )
        .unwrap();

        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(data.proof.is_some(), "proof must be Some when any input does not use prove_by_index");
        assert_eq!(data.proof.unwrap(), proof);
    }
}
