//! Decompress ATA-owned compressed tokens.
//!
//! This module provides client-side functionality to decompress multiple
//! ATA-owned compressed token accounts in a single instruction with one proof.
//!
//! Two API patterns are provided:
//!
//! ## High-level async API
//! - `decompress_atas`: Async, fetches state + proof internally
//!
//! ## High-performance sync API (for apps that pre-fetch state)
//! ```ignore
//! // 1. Fetch raw account interfaces (async)
//! let account = rpc.get_ata_account_interface(&mint, &owner).await?;
//!
//! // 2. Parse into token account interface (sync)
//! let parsed = parse_token_account_interface(&account)?;
//!
//! // 3. If cold, get proof and build instructions (sync)
//! if parsed.is_cold {
//!     let proof = rpc.get_validity_proof(...).await?;
//!     let ixs = build_decompress_atas(&[parsed], fee_payer, Some(proof))?;
//! }
//! ```

use light_client::indexer::{
    CompressedTokenAccount, GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer,
    IndexerError, ValidityProofWithContext,
};
use light_ctoken_sdk::compat::TokenData;
use light_compressed_account::compressed_account::PackedMerkleContext;
use light_ctoken_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{ExtensionStruct, TokenDataVersion},
};
use light_ctoken_sdk::{
    compat::AccountState,
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    ctoken::{derive_ctoken_ata, CreateAssociatedCTokenAccount},
    error::CTokenSdkError,
};
use light_sdk::instruction::PackedAccounts;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use spl_token_2022::state::Account as SplTokenAccount;
use thiserror::Error;

/// Error type for decompress ATA operations.
#[derive(Debug, Error)]
pub enum DecompressAtaError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("CToken SDK error: {0}")]
    CTokenSdk(#[from] CTokenSdkError),

    #[error("No state trees in proof")]
    NoStateTreesInProof,

    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),

    #[error("Cold ATA missing compressed data at index {0}")]
    MissingCompressedData(usize),

    #[error("Proof required for cold ATAs")]
    ProofRequired,

    #[error("Invalid account data")]
    InvalidAccountData,
}

// ============================================================================
// Raw Account Interface
// ============================================================================

/// Context for decompressing a cold ATA.
/// Contains all data needed to build decompression instructions.
#[derive(Debug, Clone)]
pub struct AtaDecompressionContext {
    /// Full compressed token account from indexer.
    pub compressed: CompressedTokenAccount,
    /// Wallet owner (signer for decompression).
    pub wallet_owner: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// ATA derivation bump.
    pub bump: u8,
}

/// Raw ATA account interface - Account bytes are ALWAYS present.
///
/// For hot accounts: actual on-chain bytes.
/// For cold accounts: synthetic SPL Token Account format bytes.
///
/// Use `parse_token_account_interface()` to extract typed `TokenData`.
#[derive(Debug, Clone)]
pub struct AtaAccountInterface {
    /// The ATA pubkey.
    pub pubkey: Pubkey,
    /// Raw Solana Account - always present.
    /// Hot: actual on-chain bytes.
    /// Cold: synthetic bytes (TokenData packed as SPL Token Account format).
    pub account: Account,
    /// Whether this account is compressed (needs decompression).
    pub is_cold: bool,
    /// Decompression context (only if cold).
    pub decompression_context: Option<AtaDecompressionContext>,
}

/// Pack TokenData into SPL Token Account format bytes (165 bytes).
pub fn pack_token_data_to_spl_bytes(
    mint: &Pubkey,
    owner: &Pubkey,
    token_data: &TokenData,
) -> [u8; 165] {
    use solana_program::program_pack::Pack;
    let spl_account = SplTokenAccount {
        mint: *mint,
        owner: *owner,
        amount: token_data.amount,
        delegate: token_data.delegate.into(),
        state: match token_data.state {
            AccountState::Frozen => spl_token_2022::state::AccountState::Frozen,
            _ => spl_token_2022::state::AccountState::Initialized,
        },
        is_native: solana_program::program_option::COption::None,
        delegated_amount: 0,
        close_authority: solana_program::program_option::COption::None,
    };
    let mut buf = [0u8; 165];
    SplTokenAccount::pack(spl_account, &mut buf).expect("pack should never fail");
    buf
}

// ============================================================================
// Parsed Token Account Interface
// ============================================================================

/// Parsed token account with decompression metadata.
///
/// Returned by `parse_token_account_interface()`.
/// If `is_cold` is true (or `decompression_context` is Some), the account
/// needs decompression before it can be used on-chain.
#[derive(Debug, Clone)]
pub struct TokenAccountInterface {
    /// Parsed token data (standard SPL-compatible type).
    pub token_data: TokenData,
    /// Whether this account is compressed.
    pub is_cold: bool,
    /// Decompression context if cold (contains all data for instruction building).
    pub decompression_context: Option<AtaDecompressionContext>,
}

impl TokenAccountInterface {
    /// Convenience: get amount.
    #[inline]
    pub fn amount(&self) -> u64 {
        self.token_data.amount
    }

    /// Convenience: get delegate.
    #[inline]
    pub fn delegate(&self) -> Option<Pubkey> {
        self.token_data.delegate
    }

    /// Convenience: get state.
    #[inline]
    pub fn state(&self) -> AccountState {
        self.token_data.state.clone()
    }

    /// Returns the compressed account hash if cold (for validity proof).
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.decompression_context
            .as_ref()
            .map(|d| d.compressed.account.hash)
    }
}

/// Parse raw account interface into typed TokenAccountInterface.
///
/// For hot accounts: unpacks SPL Token Account bytes.
/// For cold accounts: uses TokenData from decompression context.
pub fn parse_token_account_interface(
    interface: &AtaAccountInterface,
) -> Result<TokenAccountInterface, DecompressAtaError> {
    use solana_program::program_pack::Pack;

    if interface.is_cold {
        // Cold: use TokenData from decompression context
        let ctx = interface
            .decompression_context
            .as_ref()
            .ok_or(DecompressAtaError::InvalidAccountData)?;

        Ok(TokenAccountInterface {
            token_data: ctx.compressed.token.clone(),
            is_cold: true,
            decompression_context: Some(ctx.clone()),
        })
    } else {
        // Hot: unpack SPL Token Account from raw bytes
        let data = &interface.account.data;
        if data.len() < 165 {
            return Err(DecompressAtaError::InvalidAccountData);
        }

        let spl_account = SplTokenAccount::unpack(&data[..165])
            .map_err(|_| DecompressAtaError::InvalidAccountData)?;

        let token_data = TokenData {
            mint: spl_account.mint,
            owner: spl_account.owner,
            amount: spl_account.amount,
            delegate: spl_account.delegate.into(),
            state: match spl_account.state {
                spl_token_2022::state::AccountState::Frozen => AccountState::Frozen,
                _ => AccountState::Initialized,
            },
            tlv: None,
        };

        Ok(TokenAccountInterface {
            token_data,
            is_cold: false,
            decompression_context: None,
        })
    }
}

// ============================================================================
// Legacy AtaInterface (for backward compatibility)
// ============================================================================

/// Legacy decompression context.
#[derive(Debug, Clone)]
pub struct DecompressionContext {
    pub compressed: CompressedTokenAccount,
}

/// Legacy ATA interface.
/// Prefer `AtaAccountInterface` + `parse_token_account_interface()` for new code.
#[derive(Debug, Clone)]
pub struct AtaInterface {
    pub ata: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    pub is_cold: bool,
    pub token_data: TokenData,
    pub raw_account: Option<Account>,
    pub decompression: Option<DecompressionContext>,
}

impl AtaInterface {
    #[inline]
    pub fn is_cold(&self) -> bool {
        self.is_cold
    }

    #[inline]
    pub fn is_hot(&self) -> bool {
        self.raw_account.is_some()
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        !self.is_cold && self.raw_account.is_none()
    }

    pub fn hash(&self) -> Option<[u8; 32]> {
        self.decompression
            .as_ref()
            .map(|d| d.compressed.account.hash)
    }

    pub fn account(&self) -> Option<&Account> {
        self.raw_account.as_ref()
    }

    pub fn compressed(&self) -> Option<&CompressedTokenAccount> {
        self.decompression.as_ref().map(|d| &d.compressed)
    }

    #[inline]
    pub fn amount(&self) -> u64 {
        self.token_data.amount
    }

    #[inline]
    pub fn delegate(&self) -> Option<Pubkey> {
        self.token_data.delegate
    }

    #[inline]
    pub fn state(&self) -> AccountState {
        self.token_data.state.clone()
    }
}

/// Internal context for each ATA to decompress.
struct InternalAtaDecompressContext {
    token_account: CompressedTokenAccount,
    ata_pubkey: Pubkey,
    wallet_owner: Pubkey,
    ata_bump: u8,
}

// ============================================================================
// New API: TokenAccountInterface-based
// ============================================================================

/// Builds decompress instructions from parsed TokenAccountInterfaces (sync).
///
/// High-performance API pattern:
/// 1. Fetch raw accounts: `get_ata_account_interface()`
/// 2. Parse: `parse_token_account_interface()`
/// 3. Get proof for cold accounts (async)
/// 4. Build instructions (this function, sync)
///
/// Returns empty vec if all accounts are hot - fast exit.
///
/// # Example
/// ```ignore
/// // 1. Fetch raw account interfaces (async)
/// let account = rpc.get_ata_account_interface(&mint, &owner).await?;
///
/// // 2. Parse into token account interface (sync)
/// let parsed = parse_token_account_interface(&account)?;
///
/// // 3. Collect cold hashes for proof
/// let cold_hashes: Vec<_> = [&parsed].iter()
///     .filter_map(|p| p.hash())
///     .collect();
///
/// // 4. If any cold, get proof (async)
/// let proof = if cold_hashes.is_empty() {
///     None
/// } else {
///     Some(rpc.get_validity_proof(cold_hashes, vec![], None).await?.value)
/// };
///
/// // 5. Build instructions (sync)
/// let instructions = build_decompress_token_accounts(&[parsed], fee_payer, proof)?;
/// ```
pub fn build_decompress_token_accounts(
    token_accounts: &[TokenAccountInterface],
    fee_payer: Pubkey,
    validity_proof: Option<ValidityProofWithContext>,
) -> Result<Vec<Instruction>, DecompressAtaError> {
    let mut cold_contexts: Vec<InternalAtaDecompressContext> = Vec::new();
    let mut create_ata_instructions = Vec::new();

    for token_account in token_accounts.iter() {
        if let Some(ctx) = &token_account.decompression_context {
            // Derive ATA for destination
            let (ata_pubkey, _) = derive_ctoken_ata(&ctx.wallet_owner, &ctx.mint);

            // Create ATA idempotently
            let create_ata =
                CreateAssociatedCTokenAccount::new(fee_payer, ctx.wallet_owner, ctx.mint)
                    .idempotent()
                    .instruction()?;
            create_ata_instructions.push(create_ata);

            cold_contexts.push(InternalAtaDecompressContext {
                token_account: ctx.compressed.clone(),
                ata_pubkey,
                wallet_owner: ctx.wallet_owner,
                ata_bump: ctx.bump,
            });
        }
    }

    // Fast exit if all hot
    if cold_contexts.is_empty() {
        return Ok(vec![]);
    }

    // Proof required for cold accounts
    let proof = validity_proof.ok_or(DecompressAtaError::ProofRequired)?;

    // Build decompress instruction
    let decompress_ix = build_batch_decompress_instruction(fee_payer, &cold_contexts, proof)?;

    let mut instructions = create_ata_instructions;
    instructions.push(decompress_ix);
    Ok(instructions)
}

/// Async wrapper: decompress parsed TokenAccountInterfaces.
///
/// Takes parsed interfaces, fetches proof internally, builds instructions.
/// Returns empty vec if all accounts are hot - fast exit.
///
/// # Example
/// ```ignore
/// // Fetch and parse
/// let account = rpc.get_ata_account_interface(&mint, &owner).await?;
/// let parsed = parse_token_account_interface(&account)?;
///
/// // Decompress (fetches proof internally if needed)
/// let instructions = decompress_token_accounts(&[parsed], fee_payer, &rpc).await?;
/// ```
pub async fn decompress_token_accounts<I: Indexer>(
    token_accounts: &[TokenAccountInterface],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressAtaError> {
    let cold_hashes: Vec<[u8; 32]> = token_accounts.iter().filter_map(|a| a.hash()).collect();

    if cold_hashes.is_empty() {
        return Ok(vec![]);
    }

    let proof = indexer
        .get_validity_proof(cold_hashes, vec![], None)
        .await?
        .value;

    build_decompress_token_accounts(token_accounts, fee_payer, Some(proof))
}

// ============================================================================
// Legacy API: AtaInterface-based (backward compatibility)
// ============================================================================

/// Builds decompress instructions for ATAs synchronously (legacy API).
///
/// Prefer `build_decompress_token_accounts` with `TokenAccountInterface` for new code.
pub fn build_decompress_atas(
    atas: &[AtaInterface],
    fee_payer: Pubkey,
    validity_proof: Option<ValidityProofWithContext>,
) -> Result<Vec<Instruction>, DecompressAtaError> {
    let mut cold_contexts: Vec<InternalAtaDecompressContext> = Vec::new();
    let mut create_ata_instructions = Vec::new();

    for ata in atas.iter() {
        if ata.is_cold {
            if let Some(decompression) = &ata.decompression {
                let create_ata =
                    CreateAssociatedCTokenAccount::new(fee_payer, ata.owner, ata.mint)
                        .idempotent()
                        .instruction()?;
                create_ata_instructions.push(create_ata);

                cold_contexts.push(InternalAtaDecompressContext {
                    token_account: decompression.compressed.clone(),
                    ata_pubkey: ata.ata,
                    wallet_owner: ata.owner,
                    ata_bump: ata.bump,
                });
            }
        }
    }

    if cold_contexts.is_empty() {
        return Ok(vec![]);
    }

    let proof = validity_proof.ok_or(DecompressAtaError::ProofRequired)?;
    let decompress_ix = build_batch_decompress_instruction(fee_payer, &cold_contexts, proof)?;

    let mut instructions = create_ata_instructions;
    instructions.push(decompress_ix);
    Ok(instructions)
}

/// Async wrapper for legacy AtaInterface API.
pub async fn decompress_atas<I: Indexer>(
    atas: &[AtaInterface],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressAtaError> {
    let cold_hashes: Vec<[u8; 32]> = atas.iter().filter_map(|a| a.hash()).collect();

    if cold_hashes.is_empty() {
        return Ok(vec![]);
    }

    let proof = indexer
        .get_validity_proof(cold_hashes, vec![], None)
        .await?
        .value;

    build_decompress_atas(atas, fee_payer, Some(proof))
}

/// Decompresses ATA-owned compressed tokens for multiple (mint, owner) pairs.
///
/// This is a convenience async API that fetches state and proof internally.
/// For high-performance apps, use `build_decompress_atas` with pre-fetched state.
///
/// For each (mint, wallet_owner) pair:
/// 1. Derives the ATA address
/// 2. Fetches compressed token accounts owned by that ATA
/// 3. Gets a single validity proof for all accounts
/// 4. Creates destination ATAs if needed (idempotent)
/// 5. Builds single decompress instruction
///
/// # Arguments
/// * `mint_owner_pairs` - List of (mint, wallet_owner) pairs to decompress
/// * `fee_payer` - Fee payer pubkey
/// * `indexer` - Indexer for fetching accounts and proofs
///
/// # Returns
/// * Vec of instructions: [create_ata_idempotent..., decompress_all]
/// * Returns empty vec if no compressed tokens found
pub async fn decompress_atas_idempotent<I: Indexer>(
    mint_owner_pairs: &[(Pubkey, Pubkey)],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressAtaError> {
    let mut create_ata_instructions = Vec::new();
    let mut all_accounts: Vec<InternalAtaDecompressContext> = Vec::new();

    // Phase 1: Gather compressed token accounts and prepare ATA creation
    for (mint, wallet_owner) in mint_owner_pairs {
        let (ata_pubkey, ata_bump) = derive_ctoken_ata(wallet_owner, mint);

        // Query compressed tokens owned by this ATA
        let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(Some(
            *mint,
        )));
        let result = indexer
            .get_compressed_token_accounts_by_owner(&ata_pubkey, options, None)
            .await?;

        let accounts = result.value.items;
        if accounts.is_empty() {
            continue;
        }

        // Create ATA idempotently
        let create_ata = CreateAssociatedCTokenAccount::new(fee_payer, *wallet_owner, *mint)
            .idempotent()
            .instruction()?;
        create_ata_instructions.push(create_ata);

        // Collect context for each account
        for acc in accounts {
            all_accounts.push(InternalAtaDecompressContext {
                token_account: acc,
                ata_pubkey,
                wallet_owner: *wallet_owner,
                ata_bump,
            });
        }
    }

    if all_accounts.is_empty() {
        return Ok(create_ata_instructions);
    }

    // Phase 2: Get validity proof for all accounts
    let hashes: Vec<[u8; 32]> = all_accounts
        .iter()
        .map(|ctx| ctx.token_account.account.hash)
        .collect();

    let proof_result = indexer.get_validity_proof(hashes, vec![], None).await?.value;

    // Phase 3: Build decompress instruction
    let decompress_ix = build_batch_decompress_instruction(fee_payer, &all_accounts, proof_result)?;

    let mut instructions = create_ata_instructions;
    instructions.push(decompress_ix);
    Ok(instructions)
}

fn build_batch_decompress_instruction(
    fee_payer: Pubkey,
    accounts: &[InternalAtaDecompressContext],
    proof: ValidityProofWithContext,
) -> Result<Instruction, DecompressAtaError> {
    let mut packed_accounts = PackedAccounts::default();

    // Pack tree infos first (inserts trees and queues)
    let packed_tree_infos = proof.pack_tree_infos(&mut packed_accounts);
    let tree_infos = packed_tree_infos
        .state_trees
        .as_ref()
        .ok_or(DecompressAtaError::NoStateTreesInProof)?;

    let mut token_accounts_vec = Vec::with_capacity(accounts.len());
    let mut in_tlv_data: Vec<Vec<ExtensionInstructionData>> = Vec::with_capacity(accounts.len());
    let mut has_any_tlv = false;

    for (i, ctx) in accounts.iter().enumerate() {
        let token = &ctx.token_account.token;
        let tree_info = &tree_infos.packed_tree_infos[i];

        // Insert wallet_owner as signer (for ATA, wallet signs, not ATA pubkey)
        let owner_index = packed_accounts.insert_or_get_config(ctx.wallet_owner, true, false);

        // Insert ATA pubkey (as the token owner in TokenData - not a signer!)
        let ata_index = packed_accounts.insert_or_get(ctx.ata_pubkey);

        // Insert mint
        let mint_index = packed_accounts.insert_or_get(token.mint);

        // Insert delegate if present
        let delegate_index = token
            .delegate
            .map(|d| packed_accounts.insert_or_get(d))
            .unwrap_or(0);

        // Insert destination ATA (same as ata_index since we decompress to the same ATA)
        let destination_index = ata_index;

        // Build MultiInputTokenDataWithContext
        // NOTE: prove_by_index comes from tree_info (the proof), not account (the query)
        // The query may have stale prove_by_index values, but the proof is authoritative.
        let source = MultiInputTokenDataWithContext {
            owner: ata_index, // Token owner is ATA pubkey (not wallet!)
            amount: token.amount,
            has_delegate: token.delegate.is_some(),
            delegate: delegate_index,
            mint: mint_index,
            version: TokenDataVersion::ShaFlat as u8,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: tree_info.queue_pubkey_index,
                prove_by_index: tree_info.prove_by_index,
                leaf_index: tree_info.leaf_index,
            },
            root_index: tree_info.root_index,
        };

        // Build CTokenAccount2 for decompress
        let mut ctoken_account = CTokenAccount2::new(vec![source])?;
        ctoken_account.decompress_ctoken(token.amount, destination_index)?;
        token_accounts_vec.push(ctoken_account);

        // Build TLV for this input (CompressedOnly extension for ATAs)
        let is_frozen = token.state == AccountState::Frozen;
        let tlv_vec: Vec<ExtensionInstructionData> = token
            .tlv
            .as_ref()
            .map(|exts| {
                exts.iter()
                    .filter_map(|ext| match ext {
                        ExtensionStruct::CompressedOnly(co) => {
                            Some(ExtensionInstructionData::CompressedOnly(
                                CompressedOnlyExtensionInstructionData {
                                    delegated_amount: co.delegated_amount,
                                    withheld_transfer_fee: co.withheld_transfer_fee,
                                    is_frozen,
                                    compression_index: 0,
                                    is_ata: true,
                                    bump: ctx.ata_bump,
                                    owner_index, // Wallet owner who signs
                                },
                            ))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !tlv_vec.is_empty() {
            has_any_tlv = true;
        }
        in_tlv_data.push(tlv_vec);
    }

    // Convert packed_accounts to AccountMetas
    let (packed_account_metas, _, _) = packed_accounts.to_account_metas();

    // Build Transfer2 instruction
    let meta_config = Transfer2AccountsMetaConfig::new(fee_payer, packed_account_metas);
    let transfer_config = Transfer2Config::default().filter_zero_amount_outputs();

    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts: token_accounts_vec,
        transfer_config,
        validity_proof: proof.proof,
        in_tlv: if has_any_tlv { Some(in_tlv_data) } else { None },
        ..Default::default()
    };

    create_transfer2_instruction(inputs).map_err(DecompressAtaError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_ata() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let (ata, bump) = derive_ctoken_ata(&wallet, &mint);
        assert_ne!(ata, wallet);
        assert_ne!(ata, mint);
        let _ = bump;
    }

    #[test]
    fn test_ata_interface_is_cold() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let (ata, bump) = derive_ctoken_ata(&wallet, &mint);

        let hot_ata = AtaInterface {
            ata,
            owner: wallet,
            mint,
            bump,
            is_cold: false,
            token_data: TokenData {
                mint,
                owner: ata,
                amount: 100,
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            },
            raw_account: Some(Account::default()),
            decompression: None,
        };
        assert!(!hot_ata.is_cold());
        assert!(hot_ata.is_hot());
        assert_eq!(hot_ata.amount(), 100);

        let none_ata = AtaInterface {
            ata,
            owner: wallet,
            mint,
            bump,
            is_cold: false,
            token_data: TokenData::default(),
            raw_account: None,
            decompression: None,
        };
        assert!(!none_ata.is_cold());
        assert!(!none_ata.is_hot());
        assert!(none_ata.is_none());
    }

    #[test]
    fn test_build_decompress_atas_fast_exit() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let (ata, bump) = derive_ctoken_ata(&wallet, &mint);

        // All hot - should return empty vec
        let hot_atas = vec![AtaInterface {
            ata,
            owner: wallet,
            mint,
            bump,
            is_cold: false,
            token_data: TokenData {
                mint,
                owner: ata,
                amount: 50,
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            },
            raw_account: Some(Account::default()),
            decompression: None,
        }];

        let result = build_decompress_atas(&hot_atas, wallet, None).unwrap();
        assert!(result.is_empty());
    }
}
