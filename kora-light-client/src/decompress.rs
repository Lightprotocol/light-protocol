//! Decompress instruction builder — builds Transfer2 instructions to decompress
//! compressed tokens into light-token or SPL token accounts.
//!
//! Uses the packed accounts scheme where instruction data references accounts
//! by u8 index rather than full pubkey.
//!
//! Ported from TypeScript `create-decompress-interface-instruction.ts`.

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
        Compression, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
        PackedMerkleContext, SplInterfaceInfo,
    },
};

/// Default max top-up (u16::MAX = no limit)
const DEFAULT_MAX_TOP_UP: u16 = u16::MAX;

/// Build a decompress instruction that moves compressed tokens to an on-chain
/// light-token or SPL token account.
///
/// # Arguments
/// * `payer` - Fee payer (signer)
/// * `owner` - Token account owner
/// * `mint` - Token mint
/// * `inputs` - Compressed token accounts to decompress
/// * `proof` - Validity proof from the RPC
/// * `destination` - Destination token account (light-token ATA or SPL ATA)
/// * `amount` - Total amount to decompress
/// * `decimals` - Token decimals
/// * `spl_interface` - If decompressing to SPL, provides pool info. None for light-token.
#[allow(clippy::too_many_arguments)]
pub fn create_decompress_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    inputs: &[CompressedTokenAccountInput],
    proof: &CompressedProof,
    destination: &Pubkey,
    amount: u64,
    decimals: u8,
    spl_interface: Option<&SplInterfaceInfo>,
) -> Result<Instruction, KoraLightError> {
    if inputs.is_empty() {
        return Err(KoraLightError::NoCompressedAccounts);
    }

    // Build packed accounts array — deduplicate pubkeys
    let mut packed_indices: HashMap<Pubkey, u8> = HashMap::new();
    let mut packed_accounts: Vec<(Pubkey, bool, bool)> = Vec::new(); // (pubkey, is_signer, is_writable)

    let mut insert_or_get = |pubkey: Pubkey, is_signer: bool, is_writable: bool| -> u8 {
        if let Some(&idx) = packed_indices.get(&pubkey) {
            // Update flags if needed
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

    // 1. Add all unique merkle trees (writable)
    for input in inputs {
        insert_or_get(input.tree, false, true);
    }

    // 2. Add all unique queues (writable)
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

    // 3. Add mint (readonly)
    let mint_index = insert_or_get(*mint, false, false);

    // 4. Add owner (signer)
    let owner_index = insert_or_get(*owner, true, false);

    // 5. Add destination (writable)
    let destination_index = insert_or_get(*destination, false, true);

    // 6. Add delegates if any
    for input in inputs {
        if let Some(delegate) = &input.delegate {
            insert_or_get(*delegate, false, false);
        }
    }

    // 7. For SPL destinations: add pool and token program
    let (pool_account_index, pool_index_val, bump_val) = if let Some(spl) = spl_interface {
        let pool_idx = insert_or_get(spl.spl_interface_pda, false, true);
        let _token_prog_idx = insert_or_get(spl.token_program, false, false);
        (pool_idx, spl.pool_index, spl.bump)
    } else {
        (0u8, 0u8, 0u8)
    };

    // Build input token data
    let in_token_data: Vec<MultiInputTokenDataWithContext> = inputs
        .iter()
        .map(|input| {
            let tree_idx = packed_indices[&input.tree];
            let queue_idx = packed_indices[&input.queue];
            let delegate_idx = input.delegate.map(|d| packed_indices[&d]).unwrap_or(0);

            MultiInputTokenDataWithContext {
                owner: owner_index,
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

    // Calculate change amount
    let input_total: u64 = inputs.iter().map(|i| i.amount).sum();
    let change_amount =
        input_total
            .checked_sub(amount)
            .ok_or(KoraLightError::InsufficientBalance {
                needed: amount,
                available: input_total,
            })?;

    // Build output data (change account if needed)
    let out_token_data: Vec<MultiTokenTransferOutputData> = if change_amount > 0 {
        vec![MultiTokenTransferOutputData {
            owner: owner_index,
            amount: change_amount,
            has_delegate: false,
            delegate: 0,
            mint: mint_index,
            version: inputs[0].version,
        }]
    } else {
        Vec::new()
    };

    // Build compression operation
    let compression = if spl_interface.is_some() {
        Compression::decompress_spl(
            amount,
            mint_index,
            destination_index,
            pool_account_index,
            pool_index_val,
            bump_val,
            decimals,
        )
    } else {
        Compression::decompress(amount, mint_index, destination_index)
    };

    // Build Transfer2 instruction data
    let transfer2_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: first_queue_index,
        max_top_up: DEFAULT_MAX_TOP_UP,
        cpi_context: None,
        compressions: Some(vec![compression]),
        proof: if inputs.iter().all(|i| i.prove_by_index) { None } else { Some(*proof) },
        in_token_data,
        out_token_data,
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    // Serialize: discriminator + borsh data
    let mut data = Vec::new();
    data.push(TRANSFER2_DISCRIMINATOR);
    transfer2_data.serialize(&mut data)?;

    // Build account metas: static accounts + packed accounts
    let mut accounts = vec![
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(CPI_AUTHORITY_PDA, false),
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA, false),
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA, false),
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    // Append packed accounts
    for (pubkey, is_signer, is_writable) in &packed_accounts {
        if *is_writable {
            if *is_signer {
                accounts.push(AccountMeta::new(*pubkey, true));
            } else {
                accounts.push(AccountMeta::new(*pubkey, false));
            }
        } else if *is_signer {
            accounts.push(AccountMeta::new_readonly(*pubkey, true));
        } else {
            accounts.push(AccountMeta::new_readonly(*pubkey, false));
        }
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

    fn make_input(
        amount: u64,
        tree: Pubkey,
        queue: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
    ) -> CompressedTokenAccountInput {
        CompressedTokenAccountInput {
            hash: [0u8; 32],
            tree,
            queue,
            amount,
            leaf_index: 42,
            prove_by_index: false,
            root_index: 0,
            version: 0,
            owner,
            mint,
            delegate: None,
        }
    }

    #[test]
    fn test_decompress_basic() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let inputs = vec![make_input(1000, tree, queue, owner, mint)];
        let proof = CompressedProof::default();

        let ix = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &inputs,
            &proof,
            &destination,
            1000,
            6,
            None,
        )
        .unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.data[0], TRANSFER2_DISCRIMINATOR);
        // 7 static accounts + packed accounts (tree, queue, mint, owner, destination)
        assert_eq!(ix.accounts.len(), 7 + 5);
        // Payer is signer
        assert!(ix.accounts[1].is_signer);
        assert_eq!(ix.accounts[1].pubkey, payer);
    }

    #[test]
    fn test_decompress_with_change() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let inputs = vec![make_input(1000, tree, queue, owner, mint)];
        let proof = CompressedProof::default();

        // Decompress only 500 of 1000
        let ix = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &inputs,
            &proof,
            &destination,
            500,
            6,
            None,
        )
        .unwrap();

        // Should succeed — change of 500 goes to output account
        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
    }

    #[test]
    fn test_decompress_insufficient_balance() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let inputs = vec![make_input(100, tree, queue, owner, mint)];
        let proof = CompressedProof::default();

        let result = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &inputs,
            &proof,
            &destination,
            200,
            6,
            None,
        );

        assert!(matches!(
            result,
            Err(KoraLightError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_decompress_deduplicates_trees() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique(); // same tree for both
        let queue = Pubkey::new_unique(); // same queue for both
        let destination = Pubkey::new_unique();

        let inputs = vec![
            make_input(500, tree, queue, owner, mint),
            make_input(500, tree, queue, owner, mint),
        ];
        let proof = CompressedProof::default();

        let ix = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &inputs,
            &proof,
            &destination,
            1000,
            6,
            None,
        )
        .unwrap();

        // tree and queue deduplicated: 7 static + 5 packed (tree, queue, mint, owner, dest)
        assert_eq!(ix.accounts.len(), 7 + 5);
    }

    #[test]
    fn test_decompress_with_spl_interface() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let pool_pda = Pubkey::new_unique();
        let token_program = Pubkey::new_unique();

        let inputs = vec![make_input(1000, tree, queue, owner, mint)];
        let proof = CompressedProof::default();

        let spl = SplInterfaceInfo {
            spl_interface_pda: pool_pda,
            bump: 255,
            pool_index: 0,
            token_program,
        };

        let ix = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &inputs,
            &proof,
            &destination,
            1000,
            6,
            Some(&spl),
        )
        .unwrap();

        // 7 static + 7 packed (tree, queue, mint, owner, dest, pool, token_program)
        assert_eq!(ix.accounts.len(), 7 + 7);
    }

    #[test]
    fn test_decompress_no_inputs() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let proof = CompressedProof::default();

        let result = create_decompress_instruction(
            &payer,
            &owner,
            &mint,
            &[],
            &proof,
            &destination,
            1000,
            6,
            None,
        );

        assert!(matches!(result, Err(KoraLightError::NoCompressedAccounts)));
    }

    #[test]
    fn test_decompress_prove_by_index_no_proof() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let inputs = vec![CompressedTokenAccountInput {
            prove_by_index: true,
            ..make_input(1000, tree, queue, owner, mint)
        }];
        let proof = CompressedProof::default();

        let ix = create_decompress_instruction(
            &payer, &owner, &mint, &inputs, &proof, &destination, 1000, 6, None,
        )
        .unwrap();

        // Deserialize and verify proof is None
        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(data.proof.is_none(), "proof must be None when all inputs use prove_by_index");
    }

    #[test]
    fn test_decompress_mixed_prove_by_index_has_proof() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        let inputs = vec![
            CompressedTokenAccountInput {
                prove_by_index: true,
                ..make_input(500, tree, queue, owner, mint)
            },
            CompressedTokenAccountInput {
                prove_by_index: false,
                ..make_input(500, tree, queue, owner, mint)
            },
        ];
        let proof = CompressedProof { a: [1; 32], b: [2; 64], c: [3; 32] };

        let ix = create_decompress_instruction(
            &payer, &owner, &mint, &inputs, &proof, &destination, 1000, 6, None,
        )
        .unwrap();

        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(data.proof.is_some(), "proof must be Some when any input does not use prove_by_index");
        assert_eq!(data.proof.unwrap(), proof);
    }
}
