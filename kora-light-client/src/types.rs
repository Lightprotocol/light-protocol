//! Borsh-serializable types for Light Protocol instruction data.
//!
//! These types MUST be byte-identical to the on-chain program's Borsh layout.
//! Ported from:
//! - `program-libs/compressed-account/src/instruction_data/compressed_proof.rs`
//! - `program-libs/compressed-account/src/compressed_account.rs` (PackedMerkleContext)
//! - `program-libs/token-interface/src/instructions/transfer2/instruction_data.rs`
//! - `program-libs/token-interface/src/instructions/transfer2/compression.rs`
//! - `program-libs/token-interface/src/instructions/transfer2/cpi_context.rs`
//!
//! Source commit: HEAD of main branch at time of porting.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

// ---------------------------------------------------------------------------
// Compressed Proof (from program-libs/compressed-account)
// ---------------------------------------------------------------------------

/// ZK validity proof (a, b, c components = 128 bytes total).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

impl Default for CompressedProof {
    fn default() -> Self {
        Self {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        }
    }
}

// ---------------------------------------------------------------------------
// Packed Merkle Context (from program-libs/compressed-account)
// ---------------------------------------------------------------------------

/// Merkle tree context using packed indices into the accounts array.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct PackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

// ---------------------------------------------------------------------------
// CPI Context (from program-libs/token-interface/transfer2/cpi_context.rs)
// ---------------------------------------------------------------------------

/// Compressed CPI context for cross-program invocations.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CompressedCpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
}

// ---------------------------------------------------------------------------
// Compression (from program-libs/token-interface/transfer2/compression.rs)
// ---------------------------------------------------------------------------

/// Compression mode for token operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum CompressionMode {
    /// SPL/T22 → compressed (add tokens to pool)
    Compress,
    /// Compressed → SPL/T22 or light-token (remove tokens from pool)
    Decompress,
    /// Compress token account and close it
    CompressAndClose,
}

/// A single compression/decompression operation within a Transfer2 instruction.
///
/// All index fields (mint, source_or_recipient, authority, pool_account_index)
/// are u8 indices into the packed accounts array.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Compression {
    pub mode: CompressionMode,
    pub amount: u64,
    /// Index of mint in packed accounts
    pub mint: u8,
    /// Index of source (compress) or recipient (decompress) in packed accounts
    pub source_or_recipient: u8,
    /// Index of owner or delegate account in packed accounts
    pub authority: u8,
    /// Pool account index for SPL token compression/decompression
    pub pool_account_index: u8,
    /// Pool index for SPL token compression/decompression
    pub pool_index: u8,
    /// Bump seed for SPL token pool PDA
    pub bump: u8,
    /// Decimals for SPL token transfer_checked
    pub decimals: u8,
}

impl Compression {
    /// Create a decompress operation for light-token (no SPL pool involved).
    pub fn decompress(amount: u64, mint: u8, recipient: u8) -> Self {
        Self {
            mode: CompressionMode::Decompress,
            amount,
            mint,
            source_or_recipient: recipient,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
            decimals: 0,
        }
    }

    /// Create a decompress operation to SPL token account (uses pool).
    pub fn decompress_spl(
        amount: u64,
        mint: u8,
        recipient: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
        decimals: u8,
    ) -> Self {
        Self {
            mode: CompressionMode::Decompress,
            amount,
            mint,
            source_or_recipient: recipient,
            authority: 0,
            pool_account_index,
            pool_index,
            bump,
            decimals,
        }
    }

    /// Create a compress operation from SPL token account (uses pool).
    #[allow(clippy::too_many_arguments)]
    pub fn compress_spl(
        amount: u64,
        mint: u8,
        source: u8,
        authority: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
        decimals: u8,
    ) -> Self {
        Self {
            mode: CompressionMode::Compress,
            amount,
            mint,
            source_or_recipient: source,
            authority,
            pool_account_index,
            pool_index,
            bump,
            decimals,
        }
    }

    /// Create a compress operation for light-token (no SPL pool).
    pub fn compress(amount: u64, mint: u8, source: u8, authority: u8) -> Self {
        Self {
            mode: CompressionMode::Compress,
            amount,
            mint,
            source_or_recipient: source,
            authority,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
            decimals: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Transfer2 instruction data types
// (from program-libs/token-interface/transfer2/instruction_data.rs)
// ---------------------------------------------------------------------------

/// Input token data with merkle context for Transfer2.
///
/// All pubkey fields are u8 indices into the packed accounts array.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MultiInputTokenDataWithContext {
    /// Index of owner in packed accounts
    pub owner: u8,
    pub amount: u64,
    /// Whether a delegate is set
    pub has_delegate: bool,
    /// Index of delegate in packed accounts (only valid if has_delegate)
    pub delegate: u8,
    /// Index of mint in packed accounts
    pub mint: u8,
    /// Token data version
    pub version: u8,
    /// Merkle tree context with packed indices
    pub merkle_context: PackedMerkleContext,
    /// Index of the root used in inclusion validity proof
    pub root_index: u16,
}

/// Output token data for Transfer2.
///
/// All pubkey fields are u8 indices into the packed accounts array.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MultiTokenTransferOutputData {
    /// Index of owner in packed accounts
    pub owner: u8,
    pub amount: u64,
    /// Whether a delegate is set
    pub has_delegate: bool,
    /// Index of delegate in packed accounts (only valid if has_delegate)
    pub delegate: u8,
    /// Index of mint in packed accounts
    pub mint: u8,
    /// Token data version
    pub version: u8,
}

/// Full Transfer2 instruction data (Borsh-serialized).
///
/// The discriminator byte (101) is prepended BEFORE this struct when building
/// the instruction — it is NOT part of this struct.
#[repr(C)]
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedTokenInstructionDataTransfer2 {
    pub with_transaction_hash: bool,
    pub with_lamports_change_account_merkle_tree_index: bool,
    pub lamports_change_account_merkle_tree_index: u8,
    pub lamports_change_account_owner_index: u8,
    /// Index of output queue in packed accounts
    pub output_queue: u8,
    /// Maximum lamports for rent and top-up combined (u16::MAX = no limit, 0 = no top-ups)
    pub max_top_up: u16,
    pub cpi_context: Option<CompressedCpiContext>,
    pub compressions: Option<Vec<Compression>>,
    pub proof: Option<CompressedProof>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub out_token_data: Vec<MultiTokenTransferOutputData>,
    pub in_lamports: Option<Vec<u64>>,
    pub out_lamports: Option<Vec<u64>>,
    /// Extensions for input compressed token accounts (one Vec per input account)
    pub in_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
    /// Extensions for output compressed token accounts (one Vec per output account)
    pub out_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
}

// ---------------------------------------------------------------------------
// Extension instruction data
// (from program-libs/token-interface/instructions/extensions/)
//
// This enum must match the on-chain variant ordering exactly for Borsh compat.
// ---------------------------------------------------------------------------

/// Extension data for compressed token accounts.
///
/// Variant ordering MUST match the on-chain enum exactly (33 variants, indices 0-32).
/// Only variants 19, 31, and 32 carry data; the rest are reserved placeholders.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum ExtensionInstructionData {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18,
    /// Token metadata extension (index 19)
    TokenMetadata(TokenMetadataInstructionData),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    Placeholder26,
    /// Reserved for PausableAccount extension (index 27)
    Placeholder27,
    /// Reserved for PermanentDelegateAccount extension (index 28)
    Placeholder28,
    Placeholder29,
    Placeholder30,
    /// CompressedOnly extension (index 31) — marks account as decompress-only
    CompressedOnly(CompressedOnlyExtensionInstructionData),
    /// Compressible extension (index 32) — compression info from light-compressible
    Compressible(CompressionInfo),
}

/// Token metadata for compressed token accounts (index 19).
/// Uses [u8; 32] for pubkey instead of Pubkey for version-agnosticism.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<[u8; 32]>,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
}

/// Key-value metadata pair.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AdditionalMetadata {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

/// CompressedOnly extension data (index 31).
/// Marks a compressed account as decompress-only (cannot be transferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CompressedOnlyExtensionInstructionData {
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
    pub is_frozen: bool,
    pub compression_index: u8,
    pub is_ata: bool,
    pub bump: u8,
    pub owner_index: u8,
}

/// Compressible extension data (index 32) — compression info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CompressionInfo {
    pub config_account_version: u16,
    pub compress_to_pubkey: u8,
    pub account_version: u8,
    pub lamports_per_write: u32,
    pub compression_authority: [u8; 32],
    pub rent_sponsor: [u8; 32],
    pub last_claimed_slot: u64,
}

// ---------------------------------------------------------------------------
// Create ATA instruction data types
// (from program-libs/token-interface/instructions/create_associated_token_account.rs)
// (from program-libs/token-interface/instructions/extensions/compressible.rs)
// ---------------------------------------------------------------------------

/// Instruction data for CreateAssociatedTokenAccount.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateAssociatedTokenAccountInstructionData {
    pub compressible_config: Option<CompressibleExtensionInstructionData>,
}

/// Compressible extension data for token accounts.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressibleExtensionInstructionData {
    pub token_account_version: u8,
    pub rent_payment: u8,
    pub compression_only: u8,
    pub write_top_up: u32,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
}

/// Destination pubkey specification for compress operations.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressToPubkey {
    pub bump: u8,
    pub program_id: [u8; 32],
    pub seeds: Vec<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Input types for instruction builders (not on-chain — crate-specific)
// ---------------------------------------------------------------------------

/// A compressed token account as returned by the RPC, ready for instruction building.
///
/// Kora implements `TryFrom<CompressedTokenAccountRpc>` for this type.
#[derive(Debug, Clone)]
pub struct CompressedTokenAccountInput {
    /// The hash of this compressed account
    pub hash: [u8; 32],
    /// The Merkle tree this account lives in
    pub tree: Pubkey,
    /// The nullifier queue for this tree
    pub queue: Pubkey,
    /// Token amount
    pub amount: u64,
    /// Leaf index in the Merkle tree
    pub leaf_index: u32,
    /// Whether this account can use prove-by-index optimization
    pub prove_by_index: bool,
    /// Root index for the validity proof
    pub root_index: u16,
    /// Token data version
    pub version: u8,
    /// Owner of this token account
    pub owner: Pubkey,
    /// Mint of this token account
    pub mint: Pubkey,
    /// Optional delegate
    pub delegate: Option<Pubkey>,
}

/// SPL interface info for compress/decompress operations involving SPL token pools.
#[derive(Debug, Clone)]
pub struct SplInterfaceInfo {
    /// SPL interface PDA (the token pool account)
    pub spl_interface_pda: Pubkey,
    /// Bump for the PDA
    pub bump: u8,
    /// Pool index (typically 0)
    pub pool_index: u8,
    /// The SPL token program (Token or Token-2022)
    pub token_program: Pubkey,
}

/// Validity proof with root indices from the RPC.
#[derive(Debug, Clone)]
pub struct ValidityProofWithContext {
    pub compressed_proof: CompressedProof,
    /// One root index per input account, in the same order
    pub root_indices: Vec<u16>,
}

// ---------------------------------------------------------------------------
// Borsh verification tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// BORSH VERIFICATION GATE
    ///
    /// Verify that borsh 1.5 produces the same bytes as the on-chain program
    /// (which uses borsh 0.10 via AnchorSerialize). The binary format is the
    /// same across borsh versions for these primitive types.
    #[test]
    fn borsh_gate_compressed_proof() {
        let proof = CompressedProof {
            a: [1u8; 32],
            b: [2u8; 64],
            c: [3u8; 32],
        };

        let bytes = borsh::to_vec(&proof).expect("serialize");
        assert_eq!(
            bytes.len(),
            128,
            "CompressedProof should be exactly 128 bytes"
        );
        assert_eq!(&bytes[0..32], &[1u8; 32], "a field");
        assert_eq!(&bytes[32..96], &[2u8; 64], "b field");
        assert_eq!(&bytes[96..128], &[3u8; 32], "c field");
    }

    #[test]
    fn borsh_gate_packed_merkle_context() {
        let ctx = PackedMerkleContext {
            merkle_tree_pubkey_index: 7,
            queue_pubkey_index: 8,
            leaf_index: 42,
            prove_by_index: true,
        };

        let bytes = borsh::to_vec(&ctx).expect("serialize");
        // u8 + u8 + u32(LE) + bool = 1 + 1 + 4 + 1 = 7 bytes
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[0], 7); // merkle_tree_pubkey_index
        assert_eq!(bytes[1], 8); // queue_pubkey_index
        assert_eq!(&bytes[2..6], &42u32.to_le_bytes()); // leaf_index
        assert_eq!(bytes[6], 1); // prove_by_index = true
    }

    #[test]
    fn borsh_gate_compression_mode() {
        // CompressionMode::Compress = variant 0
        let bytes = borsh::to_vec(&CompressionMode::Compress).unwrap();
        assert_eq!(bytes, vec![0]);

        // CompressionMode::Decompress = variant 1
        let bytes = borsh::to_vec(&CompressionMode::Decompress).unwrap();
        assert_eq!(bytes, vec![1]);

        // CompressionMode::CompressAndClose = variant 2
        let bytes = borsh::to_vec(&CompressionMode::CompressAndClose).unwrap();
        assert_eq!(bytes, vec![2]);
    }

    #[test]
    fn borsh_gate_compression_struct() {
        let c = Compression::decompress(1000, 3, 5);
        let bytes = borsh::to_vec(&c).expect("serialize");

        // CompressionMode(1 byte) + amount(8) + mint(1) + source_or_recipient(1) +
        // authority(1) + pool_account_index(1) + pool_index(1) + bump(1) + decimals(1) = 16
        assert_eq!(bytes.len(), 16);
        assert_eq!(bytes[0], 1); // Decompress
        assert_eq!(&bytes[1..9], &1000u64.to_le_bytes()); // amount
        assert_eq!(bytes[9], 3); // mint index
        assert_eq!(bytes[10], 5); // recipient index
    }

    #[test]
    fn borsh_gate_multi_input_token_data() {
        let data = MultiInputTokenDataWithContext {
            owner: 1,
            amount: 500,
            has_delegate: false,
            delegate: 0,
            mint: 2,
            version: 0,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 3,
                queue_pubkey_index: 4,
                leaf_index: 10,
                prove_by_index: false,
            },
            root_index: 5,
        };

        let bytes = borsh::to_vec(&data).expect("serialize");
        // u8 + u64 + bool + u8 + u8 + u8 + PMC(7) + u16 = 1+8+1+1+1+1+7+2 = 22
        assert_eq!(bytes.len(), 22);
        assert_eq!(bytes[0], 1); // owner index
    }

    #[test]
    fn borsh_gate_output_data() {
        let data = MultiTokenTransferOutputData {
            owner: 1,
            amount: 1000,
            has_delegate: false,
            delegate: 0,
            mint: 2,
            version: 0,
        };

        let bytes = borsh::to_vec(&data).expect("serialize");
        // u8 + u64 + bool + u8 + u8 + u8 = 1+8+1+1+1+1 = 13
        assert_eq!(bytes.len(), 13);
    }
}
