//! Load ATA batch orchestration — decompress compressed tokens into an ATA.
//!
//! Ported from TypeScript `load-ata.ts` `_buildLoadBatches` function.
//! All RPC calls are Kora's responsibility — this function takes pre-fetched data.

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    account_select::MAX_INPUT_ACCOUNTS,
    create_ata::create_ata_idempotent_instruction,
    decompress::create_decompress_instruction,
    error::KoraLightError,
    types::{CompressedProof, CompressedTokenAccountInput, SplInterfaceInfo},
    wrap::create_wrap_instruction,
};

/// Compute unit constants for load operations
const CU_ATA_CREATION: u32 = 30_000;
const CU_WRAP: u32 = 50_000;
const CU_DECOMPRESS_BASE: u32 = 50_000;
const CU_FULL_PROOF: u32 = 100_000;
const CU_PER_ACCOUNT_PROVE_BY_INDEX: u32 = 10_000;
const CU_PER_ACCOUNT_FULL_PROOF: u32 = 30_000;
const CU_BUFFER_FACTOR: f32 = 1.3;
const CU_MIN: u32 = 50_000;
const CU_MAX: u32 = 1_400_000;

/// A batch of instructions representing one transaction in a load operation.
#[derive(Debug)]
pub struct LoadBatch {
    /// All instructions for this transaction
    pub instructions: Vec<Instruction>,
    /// Number of compressed accounts being decompressed in this batch
    pub num_compressed_accounts: usize,
    /// Whether this batch includes ATA creation
    pub has_ata_creation: bool,
    /// Number of wrap operations in this batch
    pub wrap_count: usize,
}

/// Input for building load ATA instructions.
///
/// All data must be pre-fetched by the caller (Kora).
#[derive(Debug)]
pub struct LoadAtaInput {
    /// Fee payer
    pub payer: Pubkey,
    /// Token owner
    pub owner: Pubkey,
    /// Token mint
    pub mint: Pubkey,
    /// Token decimals
    pub decimals: u8,
    /// Destination ATA address
    pub destination: Pubkey,
    /// Whether the destination ATA needs to be created
    pub needs_ata_creation: bool,
    /// Compressed accounts to decompress, in order
    pub compressed_accounts: Vec<CompressedTokenAccountInput>,
    /// One validity proof per chunk (chunks of MAX_INPUT_ACCOUNTS)
    pub proofs: Vec<CompressedProof>,
    /// SPL interface info if decompressing to SPL (None for light-token)
    pub spl_interface: Option<SplInterfaceInfo>,
    /// Optional: SPL balance to wrap (source ATA → light-token destination)
    pub spl_wrap: Option<WrapSource>,
}

/// SPL balance to wrap as part of load operation.
#[derive(Debug)]
pub struct WrapSource {
    /// SPL token account to wrap from
    pub source_ata: Pubkey,
    /// Amount to wrap
    pub amount: u64,
    /// SPL interface info for the wrap
    pub spl_interface: SplInterfaceInfo,
}

/// Build load ATA instruction batches.
///
/// Returns one `Vec<Instruction>` per transaction. Each inner vec is a complete
/// set of instructions for one transaction (compute budget + setup + decompress).
///
/// # Arguments
/// * `input` - Pre-fetched data for the load operation
///
/// # Returns
/// * `Vec<LoadBatch>` — each batch is one transaction
pub fn create_load_ata_batches(input: LoadAtaInput) -> Result<Vec<LoadBatch>, KoraLightError> {
    let mut batches: Vec<LoadBatch> = Vec::new();

    // If nothing to do, return empty
    if input.compressed_accounts.is_empty() && input.spl_wrap.is_none() && !input.needs_ata_creation
    {
        return Ok(batches);
    }

    // Build setup instructions (ATA creation + wraps)
    let mut setup_instructions: Vec<Instruction> = Vec::new();
    let mut wrap_count = 0;

    if input.needs_ata_creation {
        setup_instructions.push(create_ata_idempotent_instruction(
            &input.payer,
            &input.owner,
            &input.mint,
        )?);
    }

    if let Some(wrap) = &input.spl_wrap {
        setup_instructions.push(create_wrap_instruction(
            &wrap.source_ata,
            &input.destination,
            &input.owner,
            &input.mint,
            wrap.amount,
            input.decimals,
            &input.payer,
            &wrap.spl_interface,
        )?);
        wrap_count += 1;
    }

    // If no compressed accounts to decompress, return setup-only batch
    if input.compressed_accounts.is_empty() {
        if !setup_instructions.is_empty() {
            let cu = calculate_compute_units(0, input.needs_ata_creation, wrap_count, false);
            let mut instructions = vec![compute_budget_instruction(cu)];
            instructions.extend(setup_instructions);
            batches.push(LoadBatch {
                instructions,
                num_compressed_accounts: 0,
                has_ata_creation: input.needs_ata_creation,
                wrap_count,
            });
        }
        return Ok(batches);
    }

    // Chunk compressed accounts into batches of MAX_INPUT_ACCOUNTS
    let chunks: Vec<&[CompressedTokenAccountInput]> = input
        .compressed_accounts
        .chunks(MAX_INPUT_ACCOUNTS)
        .collect();

    if chunks.len() != input.proofs.len() {
        return Err(KoraLightError::InvalidInput(format!(
            "Expected {} proofs for {} chunks, got {}",
            chunks.len(),
            chunks.len(),
            input.proofs.len(),
        )));
    }

    for (i, (chunk, proof)) in chunks.iter().zip(input.proofs.iter()).enumerate() {
        let mut batch_instructions: Vec<Instruction> = Vec::new();
        let mut batch_has_ata = false;
        let mut batch_wrap_count = 0;

        // First batch gets setup instructions
        if i == 0 {
            batch_instructions.append(&mut setup_instructions);
            batch_has_ata = input.needs_ata_creation;
            batch_wrap_count = wrap_count;
        } else if input.needs_ata_creation {
            // Subsequent batches: idempotent ATA creation (no-op if exists)
            batch_instructions.push(create_ata_idempotent_instruction(
                &input.payer,
                &input.owner,
                &input.mint,
            )?);
            batch_has_ata = true;
        }

        // Calculate chunk amount
        let chunk_amount: u64 = chunk.iter().map(|a| a.amount).sum();

        // Build decompress instruction for this chunk
        let decompress_ix = create_decompress_instruction(
            &input.payer,
            &input.owner,
            &input.mint,
            chunk,
            proof,
            &input.destination,
            chunk_amount,
            input.decimals,
            input.spl_interface.as_ref(),
        )?;
        batch_instructions.push(decompress_ix);

        // Check if any account needs full proof
        let needs_full_proof = chunk.iter().any(|a| !a.prove_by_index);

        // Calculate and prepend compute budget
        let cu = calculate_compute_units(
            chunk.len(),
            batch_has_ata,
            batch_wrap_count,
            needs_full_proof,
        );

        let mut final_instructions = vec![compute_budget_instruction(cu)];
        final_instructions.extend(batch_instructions);

        batches.push(LoadBatch {
            instructions: final_instructions,
            num_compressed_accounts: chunk.len(),
            has_ata_creation: batch_has_ata,
            wrap_count: batch_wrap_count,
        });
    }

    Ok(batches)
}

fn calculate_compute_units(
    num_accounts: usize,
    has_ata_creation: bool,
    wrap_count: usize,
    needs_full_proof: bool,
) -> u32 {
    let mut cu: u32 = 0;

    if has_ata_creation {
        cu += CU_ATA_CREATION;
    }
    cu += wrap_count as u32 * CU_WRAP;

    if num_accounts > 0 {
        cu += CU_DECOMPRESS_BASE;
        if needs_full_proof {
            cu += CU_FULL_PROOF;
        }
        for _ in 0..num_accounts {
            cu += if needs_full_proof {
                CU_PER_ACCOUNT_FULL_PROOF
            } else {
                CU_PER_ACCOUNT_PROVE_BY_INDEX
            };
        }
    }

    let cu_buffered = (cu as f32 * CU_BUFFER_FACTOR).ceil() as u32;
    cu_buffered.clamp(CU_MIN, CU_MAX)
}

fn compute_budget_instruction(units: u32) -> Instruction {
    solana_compute_budget_interface::ComputeBudgetInstruction::set_compute_unit_limit(units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account(amount: u64) -> CompressedTokenAccountInput {
        CompressedTokenAccountInput {
            hash: [0u8; 32],
            tree: Pubkey::new_unique(),
            queue: Pubkey::new_unique(),
            amount,
            leaf_index: 0,
            prove_by_index: false,
            root_index: 0,
            version: 0,
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            delegate: None,
        }
    }

    #[test]
    fn test_empty_load() {
        let input = LoadAtaInput {
            payer: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            decimals: 6,
            destination: Pubkey::new_unique(),
            needs_ata_creation: false,
            compressed_accounts: Vec::new(),
            proofs: Vec::new(),
            spl_interface: None,
            spl_wrap: None,
        };

        let batches = create_load_ata_batches(input).unwrap();
        assert!(batches.is_empty());
    }

    #[test]
    fn test_single_account_load() {
        let input = LoadAtaInput {
            payer: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            decimals: 6,
            destination: Pubkey::new_unique(),
            needs_ata_creation: true,
            compressed_accounts: vec![make_account(1000)],
            proofs: vec![CompressedProof::default()],
            spl_interface: None,
            spl_wrap: None,
        };

        let batches = create_load_ata_batches(input).unwrap();
        assert_eq!(batches.len(), 1);
        assert!(batches[0].has_ata_creation);
        assert_eq!(batches[0].num_compressed_accounts, 1);
        // compute_budget + ata_create + decompress = 3 instructions
        assert_eq!(batches[0].instructions.len(), 3);
    }

    #[test]
    fn test_multi_batch_load() {
        // 12 accounts = 2 batches (8 + 4)
        let accounts: Vec<_> = (0..12).map(|_| make_account(100)).collect();
        let proofs = vec![CompressedProof::default(), CompressedProof::default()];

        let input = LoadAtaInput {
            payer: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            decimals: 6,
            destination: Pubkey::new_unique(),
            needs_ata_creation: true,
            compressed_accounts: accounts,
            proofs,
            spl_interface: None,
            spl_wrap: None,
        };

        let batches = create_load_ata_batches(input).unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].num_compressed_accounts, 8);
        assert_eq!(batches[1].num_compressed_accounts, 4);
        // Second batch also gets idempotent ATA creation
        assert!(batches[1].has_ata_creation);
    }

    #[test]
    fn test_proof_count_mismatch() {
        let input = LoadAtaInput {
            payer: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            decimals: 6,
            destination: Pubkey::new_unique(),
            needs_ata_creation: false,
            compressed_accounts: vec![make_account(1000)],
            proofs: Vec::new(), // Mismatch: 1 chunk but 0 proofs
            spl_interface: None,
            spl_wrap: None,
        };

        let result = create_load_ata_batches(input);
        assert!(matches!(result, Err(KoraLightError::InvalidInput(_))));
    }

    #[test]
    fn test_compute_units_calculation() {
        // Basic: 1 account, no ATA, no wrap, full proof
        let cu = calculate_compute_units(1, false, 0, true);
        let expected = ((CU_DECOMPRESS_BASE + CU_FULL_PROOF + CU_PER_ACCOUNT_FULL_PROOF) as f32
            * CU_BUFFER_FACTOR)
            .ceil() as u32;
        assert_eq!(cu, expected.max(CU_MIN).min(CU_MAX));

        // With ATA creation
        let cu_with_ata = calculate_compute_units(1, true, 0, true);
        assert!(cu_with_ata > cu);
    }
}
