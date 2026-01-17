//! Transaction size estimation and instruction batching.

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Maximum transaction size in bytes (1280 MTU - 40 IPv6 header - 8 fragment header).
pub const PACKET_DATA_SIZE: usize = 1232;

/// Split instructions into groups that fit within transaction size limits.
///
/// # Arguments
/// * `instructions` - Instructions to split
/// * `payer` - Fee payer pubkey  
/// * `num_signers` - Number of signers
/// * `max_size` - Max tx size (defaults to PACKET_DATA_SIZE)
pub fn split_by_tx_size(
    instructions: Vec<Instruction>,
    payer: &Pubkey,
    num_signers: usize,
    max_size: Option<usize>,
) -> Vec<Vec<Instruction>> {
    let max_size = max_size.unwrap_or(PACKET_DATA_SIZE);

    if instructions.is_empty() {
        return vec![];
    }

    let mut batches = Vec::new();
    let mut current_batch = Vec::new();

    for ix in instructions {
        let mut trial = current_batch.clone();
        trial.push(ix.clone());

        if estimate_tx_size(&trial, payer, num_signers) > max_size {
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

    batches
}

/// Estimate transaction size including signatures.
fn estimate_tx_size(instructions: &[Instruction], payer: &Pubkey, num_signers: usize) -> usize {
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
    use super::*;
    use solana_instruction::AccountMeta;

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

        let batches = split_by_tx_size(instructions, &payer, 1, None);
        assert!(batches.len() > 1);

        for batch in &batches {
            assert!(estimate_tx_size(batch, &payer, 1) <= PACKET_DATA_SIZE);
        }
    }
}
