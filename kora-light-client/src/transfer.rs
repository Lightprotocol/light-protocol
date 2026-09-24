//! Transfer instruction builders for Light Protocol.
//!
//! Provides Transfer2 instruction building for compressed token transfers.
//! Routes between:
//! - Compressed-to-compressed (Transfer2 with compressed inputs)
//! - Light-token-to-light-token (TransferChecked via on-chain accounts)

#[cfg(test)]
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::KoraLightError,
    packed_accounts::PackedAccountsBuilder,
    program_ids::{
        ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA,
        DEFAULT_MAX_TOP_UP, LIGHT_SYSTEM_PROGRAM_ID, LIGHT_TOKEN_PROGRAM_ID,
        REGISTERED_PROGRAM_PDA, SYSTEM_PROGRAM_ID, TRANSFER2_DISCRIMINATOR,
    },
    types::{
        CompressedProof, CompressedTokenAccountInput, CompressedTokenInstructionDataTransfer2,
        MultiInputTokenDataWithContext, MultiTokenTransferOutputData, PackedMerkleContext,
    },
};

/// Light Token TransferChecked discriminator
const TRANSFER_CHECKED_DISCRIMINATOR: u8 = 12;

/// Compressed-to-compressed token transfer.
///
/// Builds a Transfer2 instruction that moves tokens between compressed accounts.
/// Automatically creates a change output if `amount < input_total`.
/// Omits the proof when all inputs use `prove_by_index`.
///
/// # Example
/// ```rust,ignore
/// use kora_light_client::Transfer2;
///
/// let ix = Transfer2 {
///     payer,
///     authority,
///     mint,
///     inputs: &accounts,
///     proof: &proof,
///     destination_owner,
///     amount: 1_000,
/// }.instruction()?;
/// ```
#[derive(Debug, Clone)]
pub struct Transfer2<'a> {
    /// Fee payer (signer).
    pub payer: Pubkey,
    /// Token owner or delegate (signer).
    pub authority: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// Source compressed token accounts.
    pub inputs: &'a [CompressedTokenAccountInput],
    /// Validity proof from the RPC.
    pub proof: &'a CompressedProof,
    /// Owner of the destination compressed account.
    pub destination_owner: Pubkey,
    /// Amount to transfer.
    pub amount: u64,
}

impl<'a> Transfer2<'a> {
    /// Build the Transfer2 instruction.
    pub fn instruction(&self) -> Result<Instruction, KoraLightError> {
        create_transfer2_instruction(
            &self.payer,
            &self.authority,
            &self.mint,
            self.inputs,
            self.proof,
            &self.destination_owner,
            self.amount,
        )
    }
}

/// Decompressed ATA-to-ATA token transfer.
///
/// Builds a TransferChecked instruction for on-chain light-token accounts.
/// Not for compressed accounts.
///
/// # Example
/// ```rust,ignore
/// use kora_light_client::TransferChecked;
///
/// let ix = TransferChecked {
///     source_ata,
///     destination_ata,
///     mint,
///     owner,
///     amount: 1_000,
///     decimals: 6,
///     payer,
/// }.instruction()?;
/// ```
#[derive(Debug, Clone)]
pub struct TransferChecked {
    /// Source token account (writable).
    pub source_ata: Pubkey,
    /// Destination token account (writable).
    pub destination_ata: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// Token owner (signer).
    pub owner: Pubkey,
    /// Amount to transfer.
    pub amount: u64,
    /// Token decimals.
    pub decimals: u8,
    /// Fee payer (signer). Only added if different from owner.
    pub payer: Pubkey,
}

impl TransferChecked {
    /// Build the TransferChecked instruction.
    pub fn instruction(&self) -> Result<Instruction, KoraLightError> {
        create_transfer_checked_instruction(
            &self.source_ata,
            &self.destination_ata,
            &self.mint,
            &self.owner,
            self.amount,
            self.decimals,
            &self.payer,
        )
    }
}

/// Build a Transfer2 instruction for compressed-to-compressed token transfers.
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
    let mut builder = PackedAccountsBuilder::new();

    // 1. Trees (writable)
    for input in inputs {
        builder.insert_or_get(input.tree, false, true);
    }

    // 2. Queues (writable)
    for input in inputs {
        builder.insert_or_get(input.queue, false, true);
    }
    let first_queue_index = builder.get_index(&inputs[0].queue);

    // 3. Mint (readonly)
    let mint_index = builder.insert_or_get(*mint, false, false);

    // 4. Authority/owner (signer)
    let authority_index = builder.insert_or_get(*authority, true, false);

    // 5. Destination owner (readonly)
    let dest_owner_index = builder.insert_or_get(*destination_owner, false, false);

    // 6. Delegates if any
    for input in inputs {
        if let Some(delegate) = &input.delegate {
            builder.insert_or_get(*delegate, false, false);
        }
    }

    // Build input token data
    let in_token_data: Vec<MultiInputTokenDataWithContext> = inputs
        .iter()
        .map(|input| {
            let tree_idx = builder.get_index(&input.tree);
            let queue_idx = builder.get_index(&input.queue);
            let delegate_idx = input.delegate.map(|d| builder.get_index(&d)).unwrap_or(0);

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
    let input_total: u64 = inputs
        .iter()
        .try_fold(0u64, |acc, i| acc.checked_add(i.amount))
        .ok_or(KoraLightError::ArithmeticOverflow)?;
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
        proof: if inputs.iter().all(|i| i.prove_by_index) {
            None
        } else {
            Some(*proof)
        },
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

    accounts.extend(builder.build_account_metas());

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
            &payer,
            &authority,
            &mint,
            &inputs,
            &proof,
            &dest_owner,
            1000,
        )
        .unwrap();

        // Deserialize and verify proof is None
        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(
            data.proof.is_none(),
            "proof must be None when all inputs use prove_by_index"
        );
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
        let proof = CompressedProof {
            a: [1; 32],
            b: [2; 64],
            c: [3; 32],
        };

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

        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(
            data.proof.is_some(),
            "proof must be Some when any input does not use prove_by_index"
        );
        assert_eq!(data.proof.unwrap(), proof);
    }

    #[test]
    fn test_transfer2_with_delegate() {
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let dest_owner = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        let inputs = vec![CompressedTokenAccountInput {
            delegate: Some(delegate),
            ..make_input(1000, tree, queue)
        }];
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

        // 7 static + packed (tree, queue, mint, authority, dest_owner, delegate)
        assert_eq!(ix.accounts.len(), 7 + 6);

        // Verify delegate is in packed accounts (readonly, not signer)
        let delegate_account = &ix.accounts[7 + 5]; // last packed account
        assert_eq!(delegate_account.pubkey, delegate);
        assert!(!delegate_account.is_signer);
        assert!(!delegate_account.is_writable);

        // Verify instruction data has delegate set
        let data = CompressedTokenInstructionDataTransfer2::try_from_slice(&ix.data[1..]).unwrap();
        assert!(data.in_token_data[0].has_delegate);
        assert_eq!(data.in_token_data[0].delegate, 5); // delegate is 6th packed account (index 5)
    }
}
