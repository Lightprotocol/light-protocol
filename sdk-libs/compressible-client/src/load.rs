use light_client::indexer::{Indexer, TreeInfo};
use light_client::rpc::Rpc;
use light_compressed_account::compressed_account::CompressedAccountData;
use light_sdk::compressible::Pack;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::get_compressible_account::get_account_info_interface;
use crate::{compressible_instruction, AnchorDeserialize};

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("RPC error: {0}")]
    Rpc(#[from] light_client::rpc::RpcError),

    #[error("Indexer error: {0}")]
    Indexer(#[from] light_client::indexer::IndexerError),

    #[error("Failed to build decompress instruction: {0}")]
    InstructionBuild(#[from] Box<dyn std::error::Error>),

    #[error("Account {0} not found")]
    AccountNotFound(Pubkey),

    #[error("Failed to deserialize account data")]
    Deserialization,

    #[error("Compressible account error: {0}")]
    CompressibleAccount(#[from] crate::get_compressible_account::CompressibleAccountError),
}

/// Input specification for a compressible account to be loaded.
///
/// Each account can be either a PDA or a token account, and will be checked
/// for compression state. If compressed, it will be included in the decompress
/// instruction.
#[derive(Debug, Clone)]
pub struct AccountToLoad {
    /// The address of the account (decompressed/target address)
    pub address: Pubkey,
    
    /// The owner program ID for deriving the compressed address
    pub program_id: Pubkey,
    
    /// Tree info for deriving the compressed address
    pub address_tree_info: TreeInfo,
}

/// Result of loading accounts, containing both the optional decompress instruction
/// and metadata about which accounts were compressed.
#[derive(Debug)]
pub struct LoadResult {
    /// The decompress instruction if any accounts need decompression.
    /// None if all accounts are already decompressed (hot state).
    pub instruction: Option<Instruction>,
    
    /// Metadata about which accounts were found to be compressed
    pub compressed_accounts: Vec<Pubkey>,
}

/// Loads compressible accounts by checking their compression state and building
/// a decompress instruction if needed.
///
/// This is the Rust equivalent of the TypeScript `decompressIfNeeded()` method.
/// It provides a simple interface where clients can pass their accounts without
/// needing to know the internals of compression/decompression.
///
/// # Design Goals
///
/// 1. **Simple Interface**: Clients pass account addresses and get back an instruction (or None)
/// 2. **Automatic Detection**: Automatically determines which accounts need decompression
/// 3. **Idempotent**: Returns None if all accounts are already decompressed
/// 4. **Composable**: Returns a standalone instruction that can be prepended to transactions
///
/// # Arguments
///
/// * `program_id` - The program ID that owns the compressible accounts
/// * `discriminator` - The instruction discriminator for decompress_accounts_idempotent
/// * `accounts_to_load` - List of accounts to check and potentially decompress
/// * `program_account_metas` - Additional account metas required by the program's decompress instruction
/// * `rpc` - RPC client for querying account state
///
/// # Returns
///
/// Returns `Ok(LoadResult)` where:
/// - `instruction` is `Some` if decompression is needed
/// - `instruction` is `None` if all accounts are already decompressed
///
/// # Example
///
/// ```ignore
/// use light_compressible_client::{load, AccountToLoad, LoadResult};
/// use light_client::indexer::get_default_address_tree_info;
///
/// // Define accounts to load
/// let accounts_to_load = vec![
///     AccountToLoad {
///         address: pool_state_address,
///         program_id: my_program_id,
///         address_tree_info: get_default_address_tree_info(),
///     },
///     AccountToLoad {
///         address: user_account_address,
///         program_id: my_program_id,
///         address_tree_info: get_default_address_tree_info(),
///     },
/// ];
///
/// // Program-specific accounts required for decompress instruction
/// let program_metas = vec![
///     AccountMeta::new_readonly(fee_payer, true),
///     AccountMeta::new_readonly(config, false),
///     // ... other required accounts
/// ];
///
/// // Load accounts (checks compression state and builds instruction if needed)
/// let result = load(
///     my_program_id,
///     &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
///     accounts_to_load,
///     &program_metas,
///     &mut rpc_client,
/// ).await?;
///
/// // If decompression is needed, prepend the instruction
/// if let Some(decompress_ix) = result.instruction {
///     transaction.add_instruction(decompress_ix);
/// }
/// ```
pub async fn load<T, R>(
    program_id: Pubkey,
    discriminator: &[u8],
    accounts_to_load: Vec<AccountToLoad>,
    program_account_metas: &[AccountMeta],
    rpc: &mut R,
) -> Result<LoadResult, LoadError>
where
    T: Pack + Clone + std::fmt::Debug + AnchorDeserialize,
    R: Rpc + Indexer,
{
    if accounts_to_load.is_empty() {
        return Ok(LoadResult {
            instruction: None,
            compressed_accounts: Vec::new(),
        });
    }

    // Step 1: Fetch account info for all accounts to check compression state
    let mut account_interfaces = Vec::with_capacity(accounts_to_load.len());
    
    for account_spec in &accounts_to_load {
        let account_info = get_account_info_interface(
            &account_spec.address,
            &account_spec.program_id,
            &account_spec.address_tree_info,
            rpc,
        )
        .await?;

        if let Some(info) = account_info {
            account_interfaces.push((account_spec.clone(), info));
        } else {
            return Err(LoadError::AccountNotFound(account_spec.address));
        }
    }

    // Step 2: Filter to only compressed accounts
    let compressed_accounts_data: Vec<_> = account_interfaces
        .iter()
        .filter(|(_, info)| info.is_compressed)
        .collect();

    if compressed_accounts_data.is_empty() {
        // All accounts are already decompressed (hot state) - no instruction needed
        return Ok(LoadResult {
            instruction: None,
            compressed_accounts: Vec::new(),
        });
    }

    // Step 3: Build validity proof inputs
    let hashes: Vec<[u8; 32]> = compressed_accounts_data
        .iter()
        .filter_map(|(_, info)| info.merkle_context.as_ref().map(|ctx| ctx.hash))
        .collect();

    // Get validity proof from indexer
    let validity_proof_response = rpc
        .get_validity_proof(hashes.clone(), Vec::new(), None)
        .await?;

    let validity_proof_with_context = validity_proof_response.value;

    // Step 4: Prepare data for decompress instruction builder
    let mut compressed_accounts_with_data = Vec::new();
    let mut decompressed_addresses = Vec::new();

    for (account_spec, info) in &account_interfaces {
        if !info.is_compressed {
            continue;
        }

        let merkle_context = info
            .merkle_context
            .as_ref()
            .ok_or_else(|| LoadError::Deserialization)?;

        // Build CompressedAccount from the account info
        let compressed_account = light_client::indexer::CompressedAccount {
            address: None, // PDAs don't have addresses in the compressed state
            data: if !info.account_info.data.is_empty() {
                Some(CompressedAccountData {
                    discriminator: info.account_info.data[..8]
                        .try_into()
                        .map_err(|_| LoadError::Deserialization)?,
                    data: info.account_info.data[8..].to_vec(),
                    data_hash: [0u8; 32], // Will be computed by the system
                })
            } else {
                None
            },
            hash: merkle_context.hash,
            lamports: info.account_info.lamports,
            leaf_index: merkle_context.leaf_index,
            owner: info.account_info.owner,
            prove_by_index: merkle_context.prove_by_index,
            seq: None,
            slot_created: 0,
            tree_info: merkle_context.tree_info.clone(),
        };

        // Deserialize the account data to get the typed data
        let typed_data = T::deserialize(&mut &info.account_info.data[8..])
            .map_err(|_| LoadError::Deserialization)?;

        compressed_accounts_with_data.push((compressed_account, typed_data));
        decompressed_addresses.push(account_spec.address);
    }

    // Step 5: Build the decompress instruction
    let decompress_instruction = compressible_instruction::decompress_accounts_idempotent(
        &program_id,
        discriminator,
        &decompressed_addresses,
        &compressed_accounts_with_data,
        program_account_metas,
        validity_proof_with_context,
    )?;

    Ok(LoadResult {
        instruction: Some(decompress_instruction),
        compressed_accounts: decompressed_addresses,
    })
}

/// Simplified version of `load` for cases where all accounts share the same
/// program_id and address_tree_info.
///
/// This is a convenience wrapper that constructs `AccountToLoad` specs from
/// simple addresses.
pub async fn load_simple<T, R>(
    program_id: Pubkey,
    discriminator: &[u8],
    account_addresses: Vec<Pubkey>,
    address_tree_info: TreeInfo,
    program_account_metas: &[AccountMeta],
    rpc: &mut R,
) -> Result<LoadResult, LoadError>
where
    T: Pack + Clone + std::fmt::Debug + AnchorDeserialize,
    R: Rpc + Indexer,
{
    let accounts_to_load = account_addresses
        .into_iter()
        .map(|address| AccountToLoad {
            address,
            program_id,
            address_tree_info: address_tree_info.clone(),
        })
        .collect();

    load::<T, R>(program_id, discriminator, accounts_to_load, program_account_metas, rpc).await
}

