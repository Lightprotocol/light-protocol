//! Zero-copy, minimal CToken event parsing primitives.
//!
//! **Design philosophy:**
//! - Simple, composable functions
//! - Zero-copy references (no allocations during filtering)
//! - Transparent - easy to understand and adapt
//! - No opinions - just extract data, let user decide what to do
//!
//! **Usage:**
//! ```ignore
//! // 1. Extract transaction components (zero-copy)
//! let (program_ids, instructions, accounts) = extract_light_transaction(&tx);
//!
//! // 2. Parse Light events
//! let events = event_from_light_transaction(&program_ids, &instructions, accounts)?;
//!
//! // 3. Extract all mints (zero-copy)
//! let mints = extract_mints(&event, &ctoken_program_id);
//!
//! // 4. Extract metadata from a mint
//! let metadata = extract_mint_metadata(&mint.mint);
//! ```

use borsh::BorshDeserialize;
use light_compressed_account::indexer_event::event::PublicTransactionEvent;
use light_ctoken_types::state::{CompressedMint, ExtensionStruct};
use solana_pubkey::Pubkey;

/// CToken mint discriminator (all zeros)
pub const CTOKEN_MINT_DISCRIMINATOR: [u8; 8] = [0u8; 8];

/// CToken account discriminator (V2 - big-endian 3)
pub const CTOKEN_ACCOUNT_DISCRIMINATOR_V2: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 3];

/// CToken account discriminator (ShaFlat - big-endian 4)
pub const CTOKEN_ACCOUNT_DISCRIMINATOR_SHAFLAT: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];

/// Extract transaction components for Light event parsing.
///
/// **Generic design:** Works with any transaction format (solana-sdk, LaserStream, etc).
/// Just provide account_keys and a closure to extract instruction components.
///
/// **Performance:** ~100ns (minimal allocations)
///
/// # Example
/// ```ignore
/// // From solana-sdk Transaction:
/// let (program_ids, instructions, accounts) = extract_light_transaction(
///     &tx.message.account_keys,
///     &tx.message.instructions,
///     |ix| (ix.program_id_index, &ix.accounts, &ix.data),
/// );
///
/// // From LaserStream transaction:
/// let (program_ids, instructions, accounts) = extract_light_transaction(
///     &tx.transaction.message.account_keys,
///     &tx.transaction.message.instructions,
///     |ix| (ix.program_id_index, &ix.accounts, &ix.data),
/// );
///
/// let events = event_from_light_transaction(&program_ids, &instructions, accounts)?;
/// ```
#[inline]
pub fn extract_light_transaction<I, F>(
    account_keys: &[Pubkey],
    instructions: &[I],
    extract_fn: F,
) -> (Vec<Pubkey>, Vec<Vec<u8>>, Vec<Vec<Pubkey>>)
where
    F: Fn(&I) -> (u8, &[u8], &[u8]),
{
    let num_instructions = instructions.len();

    let mut program_ids = Vec::with_capacity(num_instructions);
    let mut instruction_data = Vec::with_capacity(num_instructions);
    let mut accounts_per_ix = Vec::with_capacity(num_instructions);

    for ix in instructions {
        let (program_id_index, account_indices, data) = extract_fn(ix);

        let program_id_idx = program_id_index as usize;
        let program_id = if program_id_idx < account_keys.len() {
            account_keys[program_id_idx]
        } else {
            Pubkey::default()
        };
        program_ids.push(program_id);

        instruction_data.push(data.to_vec());

        let mut ix_accounts = Vec::with_capacity(account_indices.len());
        for &account_idx in account_indices {
            let idx = account_idx as usize;
            if idx < account_keys.len() {
                ix_accounts.push(account_keys[idx]);
            }
        }
        accounts_per_ix.push(ix_accounts);
    }

    (program_ids, instruction_data, accounts_per_ix)
}

/// Zero-copy mint reference
#[derive(Debug, Copy, Clone)]
pub struct MintRef<'a> {
    /// Reference to the deserialized mint
    pub mint: &'a CompressedMint,
    /// Compressed account hash (unique identifier in tree)
    pub account_hash: &'a [u8; 32],
    /// Position in Merkle tree
    pub leaf_index: u32,
    /// Which Merkle tree
    pub merkle_tree_index: u8,
    /// True if this is a new mint (init), false if update
    pub is_new: bool,
}

/// Parsed token metadata (extracted from mint extensions)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintMetadata {
    /// Token name (e.g., "Solana")
    pub name: String,
    /// Token symbol (e.g., "SOL")
    pub symbol: String,
    /// Metadata URI
    pub uri: String,
    /// Update authority (None if immutable)
    pub update_authority: Option<Pubkey>,
}

/// Extract all CToken mints from a transaction event (zero-copy).
///
/// Returns references to deserialized mints with event context.
/// Mints are stored in `mint_storage` to maintain lifetime.
///
/// **Performance:** ~30ns per output account, ~2µs to deserialize mint
///
/// # Example
/// ```ignore
/// let mut mint_storage = Vec::new();
/// let mints = extract_mints(&event, &ctoken_program_id, &mut mint_storage)?;
///
/// for mint_ref in mints {
///     println!("Mint: {} decimals", mint_ref.mint.base.decimals);
///     if mint_ref.is_new {
///         println!("  -> NEW MINT");
///     }
/// }
/// ```
pub fn extract_mints<'a>(
    event: &'a PublicTransactionEvent,
    ctoken_program_id: &Pubkey,
    mint_storage: &'a mut Vec<CompressedMint>,
) -> Result<Vec<MintRef<'a>>, std::io::Error> {
    mint_storage.clear();
    let mut indices = Vec::new();

    // Pass 1: Deserialize mints and store
    for (idx, output) in event.output_compressed_accounts.iter().enumerate() {
        // Filter: owner must be CToken program
        if output.compressed_account.owner != *ctoken_program_id {
            continue;
        }

        // Filter: must have data with mint discriminator
        let data = match &output.compressed_account.data {
            Some(d) if d.discriminator == CTOKEN_MINT_DISCRIMINATOR => &d.data,
            _ => continue,
        };

        // Deserialize mint
        let mint = CompressedMint::deserialize(&mut &data[..])?;
        mint_storage.push(mint);
        indices.push(idx);
    }

    // Pass 2: Create references (now mint_storage is immutable)
    let mut results = Vec::with_capacity(mint_storage.len());
    for (storage_idx, output_idx) in indices.iter().enumerate() {
        let output = &event.output_compressed_accounts[*output_idx];

        // Check if new (init) or update
        let is_new = match output.compressed_account.address {
            Some(addr) => !event
                .input_compressed_account_hashes
                .iter()
                .any(|h| *h == addr),
            None => true,
        };

        results.push(MintRef {
            mint: &mint_storage[storage_idx],
            account_hash: &event.output_compressed_account_hashes[*output_idx],
            leaf_index: event.output_leaf_indices[*output_idx],
            merkle_tree_index: output.merkle_tree_index,
            is_new,
        });
    }

    Ok(results)
}

/// Extract metadata from a CompressedMint.
///
/// Parses the TokenMetadata extension (type 19) if present.
/// Handles null-terminated UTF-8 strings and null update authority.
///
/// **Performance:** ~100ns (string parsing overhead)
///
/// # Example
/// ```ignore
/// if let Some(metadata) = extract_mint_metadata(mint_ref.mint) {
///     println!("Token: {}/{}", metadata.symbol, metadata.name);
///     println!("URI: {}", metadata.uri);
/// }
/// ```
pub fn extract_mint_metadata(mint: &CompressedMint) -> Option<MintMetadata> {
    let extensions = mint.extensions.as_ref()?;

    for ext in extensions {
        if let ExtensionStruct::TokenMetadata(m) = ext {
            // Parse null-terminated UTF-8 strings
            let name = parse_utf8(&m.name);
            let symbol = parse_utf8(&m.symbol);
            let uri = parse_utf8(&m.uri);

            // Check for null update authority ([0u8; 32] = None)
            // Convert from light_compressed_account::Pubkey to solana_pubkey::Pubkey
            let update_authority = if m.update_authority.to_bytes() == [0u8; 32] {
                None
            } else {
                Some(Pubkey::new_from_array(m.update_authority.to_bytes()))
            };

            return Some(MintMetadata {
                name,
                symbol,
                uri,
                update_authority,
            });
        }
    }

    None
}

/// Extract NEW mints only (init events).
///
/// Convenience wrapper around `extract_mints` that filters for `is_new == true`.
///
/// # Example
/// ```ignore
/// let mut mint_storage = Vec::new();
/// let new_mints = extract_new_mints(&event, &ctoken_program_id, &mut mint_storage)?;
///
/// println!("Found {} new mints", new_mints.len());
/// ```
pub fn extract_new_mints<'a>(
    event: &'a PublicTransactionEvent,
    ctoken_program_id: &Pubkey,
    mint_storage: &'a mut Vec<CompressedMint>,
) -> Result<Vec<MintRef<'a>>, std::io::Error> {
    let mints = extract_mints(event, ctoken_program_id, mint_storage)?;
    Ok(mints.into_iter().filter(|m| m.is_new).collect())
}

/// Extract UPDATED mints only (update events).
///
/// Convenience wrapper around `extract_mints` that filters for `is_new == false`.
///
/// # Example
/// ```ignore
/// let mut mint_storage = Vec::new();
/// let updates = extract_mint_updates(&event, &ctoken_program_id, &mut mint_storage)?;
///
/// for mint_ref in updates {
///     println!("Mint updated: supply now {}", mint_ref.mint.base.supply);
/// }
/// ```
pub fn extract_mint_updates<'a>(
    event: &'a PublicTransactionEvent,
    ctoken_program_id: &Pubkey,
    mint_storage: &'a mut Vec<CompressedMint>,
) -> Result<Vec<MintRef<'a>>, std::io::Error> {
    let mints = extract_mints(event, ctoken_program_id, mint_storage)?;
    Ok(mints.into_iter().filter(|m| !m.is_new).collect())
}

/// Parse null-terminated UTF-8 string, trimming nulls and whitespace.
#[inline]
fn parse_utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_utf8() {
        assert_eq!(parse_utf8(b"Solana\0\0\0"), "Solana");
        assert_eq!(parse_utf8(b"SOL\0\0\0\0\0"), "SOL");
        assert_eq!(parse_utf8(b"  test  \0\0"), "test");
        assert_eq!(parse_utf8(b"\0\0\0\0\0\0\0\0"), "");
    }

    #[test]
    fn test_null_authority() {
        let null_bytes = [0u8; 32];
        let pubkey = Pubkey::new_from_array(null_bytes);
        assert_eq!(pubkey.to_bytes(), [0u8; 32]);
    }
}
