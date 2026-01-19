//! Transaction size estimation and instruction batching.

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Maximum transaction size in bytes (1280 MTU - 40 IPv6 header - 8 fragment header).
pub const PACKET_DATA_SIZE: usize = 1232;

/// Error when a single instruction exceeds the maximum transaction size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionTooLargeError {
    /// Index of the oversized instruction in the input vector.
    pub instruction_index: usize,
    /// Estimated size of a transaction containing only this instruction.
    pub estimated_size: usize,
    /// Maximum allowed transaction size.
    pub max_size: usize,
}

impl std::fmt::Display for InstructionTooLargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "instruction at index {} exceeds max transaction size: {} > {}",
            self.instruction_index, self.estimated_size, self.max_size
        )
    }
}

impl std::error::Error for InstructionTooLargeError {}

/// Split instructions into groups that fit within transaction size limits.
///
/// Signer count is derived from instruction AccountMeta.is_signer flags plus the payer.
///
/// # Arguments
/// * `instructions` - Instructions to split
/// * `payer` - Fee payer pubkey (always counted as a signer)
/// * `max_size` - Max tx size (defaults to PACKET_DATA_SIZE)
///
/// # Errors
/// Returns `InstructionTooLargeError` if any single instruction alone exceeds `max_size`.
pub fn split_by_tx_size(
    instructions: Vec<Instruction>,
    payer: &Pubkey,
    max_size: Option<usize>,
) -> Result<Vec<Vec<Instruction>>, InstructionTooLargeError> {
    let max_size = max_size.unwrap_or(PACKET_DATA_SIZE);

    if instructions.is_empty() {
        return Ok(vec![]);
    }

    let mut batches = Vec::new();
    let mut current_batch = Vec::new();

    for (idx, ix) in instructions.into_iter().enumerate() {
        let mut trial = current_batch.clone();
        trial.push(ix.clone());

        if estimate_tx_size(&trial, payer) > max_size {
            // Check if this single instruction alone exceeds max_size
            let single_ix_size = estimate_tx_size(std::slice::from_ref(&ix), payer);
            if single_ix_size > max_size {
                return Err(InstructionTooLargeError {
                    instruction_index: idx,
                    estimated_size: single_ix_size,
                    max_size,
                });
            }

            if !current_batch.is_empty() {
                batches.push(current_batch);
            }
            current_batch = vec![ix];
        } else {
            current_batch.push(ix);
        }
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    Ok(batches)
}

/// Count unique signers from instructions plus the payer.
fn count_signers(instructions: &[Instruction], payer: &Pubkey) -> usize {
    let mut signers = vec![*payer];
    for ix in instructions {
        for meta in &ix.accounts {
            if meta.is_signer && !signers.contains(&meta.pubkey) {
                signers.push(meta.pubkey);
            }
        }
    }
    signers.len()
}

/// Estimate transaction size including signatures.
///
/// Signer count is derived from instruction AccountMeta.is_signer flags plus the payer.
fn estimate_tx_size(instructions: &[Instruction], payer: &Pubkey) -> usize {
    let num_signers = count_signers(instructions, payer);

    // Collect unique accounts
    let mut accounts = vec![*payer];
    for ix in instructions {
        if !accounts.contains(&ix.program_id) {
            accounts.push(ix.program_id);
        }
        for meta in &ix.accounts {
            if !accounts.contains(&meta.pubkey) {
                accounts.push(meta.pubkey);
            }
        }
    }

    // Header: 3 bytes
    let mut size = 3;
    // Account keys: compact-u16 len + 32 bytes each
    size += compact_len(accounts.len()) + accounts.len() * 32;
    // Blockhash: 32 bytes
    size += 32;
    // Instructions
    size += compact_len(instructions.len());
    for ix in instructions {
        size += 1; // program_id index
        size += compact_len(ix.accounts.len()) + ix.accounts.len();
        size += compact_len(ix.data.len()) + ix.data.len();
    }
    // Signatures
    size += compact_len(num_signers) + num_signers * 64;

    size
}

#[inline]
fn compact_len(val: usize) -> usize {
    if val < 0x80 {
        1
    } else if val < 0x4000 {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use solana_instruction::AccountMeta;

    use super::*;

    #[test]
    fn test_split_by_tx_size() {
        let payer = Pubkey::new_unique();
        let instructions: Vec<Instruction> = (0..10)
            .map(|_| Instruction {
                program_id: Pubkey::new_unique(),
                accounts: (0..10)
                    .map(|_| AccountMeta::new(Pubkey::new_unique(), false))
                    .collect(),
                data: vec![0u8; 200],
            })
            .collect();

        let batches = split_by_tx_size(instructions, &payer, None).unwrap();
        assert!(batches.len() > 1);

        for batch in &batches {
            assert!(estimate_tx_size(batch, &payer) <= PACKET_DATA_SIZE);
        }
    }

    #[test]
    fn test_split_by_tx_size_oversized_instruction() {
        let payer = Pubkey::new_unique();

        // Create an instruction that exceeds PACKET_DATA_SIZE on its own
        let oversized_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: (0..5)
                .map(|_| AccountMeta::new(Pubkey::new_unique(), false))
                .collect(),
            data: vec![0u8; 2000], // Large data payload
        };

        let small_ix = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), false)],
            data: vec![0u8; 10],
        };

        // Oversized instruction at index 1
        let instructions = vec![small_ix.clone(), oversized_ix, small_ix];

        let result = split_by_tx_size(instructions, &payer, None);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.instruction_index, 1);
        assert!(err.estimated_size > err.max_size);
        assert_eq!(err.max_size, PACKET_DATA_SIZE);
    }

    #[test]
    fn test_signer_count_derived_from_metadata() {
        let payer = Pubkey::new_unique();
        let extra_signer = Pubkey::new_unique();

        // Instruction with an additional signer
        let ix_with_signer = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(extra_signer, true), // is_signer = true
            ],
            data: vec![0u8; 10],
        };

        // Instruction without additional signers
        let ix_no_signer = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), false)],
            data: vec![0u8; 10],
        };

        // Payer only
        assert_eq!(
            count_signers(std::slice::from_ref(&ix_no_signer), &payer),
            1
        );

        // Payer + extra signer
        assert_eq!(
            count_signers(std::slice::from_ref(&ix_with_signer), &payer),
            2
        );

        // Payer duplicated in instruction should still be 1
        let ix_payer_signer = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(payer, true)],
            data: vec![0u8; 10],
        };
        assert_eq!(count_signers(&[ix_payer_signer], &payer), 1);
    }
}
