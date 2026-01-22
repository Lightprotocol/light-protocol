use std::collections::HashMap;

use borsh::BorshDeserialize;
use light_client::rpc::{Rpc, RpcError};
use light_compressible::{
    compression_info::CompressionInfo,
    config::CompressibleConfig as CtokenCompressibleConfig,
    rent::{RentConfig, SLOTS_PER_EPOCH},
};
use light_sdk::interface::LightConfig;
use light_token_interface::{
    state::{Mint, Token, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT},
    LIGHT_TOKEN_PROGRAM_ID,
};
use solana_pubkey::Pubkey;

use crate::{
    litesvm_extensions::LiteSvmExtensions, registry_sdk::REGISTRY_PROGRAM_ID, LightProgramTest,
};

/// Determines account type from account data.
/// - If account is exactly 165 bytes: Token (legacy size without extensions)
/// - If account is > 165 bytes: read byte 165 for discriminator
/// - If account is < 165 bytes: invalid (returns None)
fn determine_account_type(data: &[u8]) -> Option<u8> {
    const ACCOUNT_TYPE_OFFSET: usize = 165;

    match data.len().cmp(&ACCOUNT_TYPE_OFFSET) {
        std::cmp::Ordering::Less => None,
        std::cmp::Ordering::Equal => Some(ACCOUNT_TYPE_TOKEN_ACCOUNT), // 165 bytes = Token
        std::cmp::Ordering::Greater => Some(data[ACCOUNT_TYPE_OFFSET]),
    }
}

/// Extracts CompressionInfo, account type, and compression_only from account data.
/// Returns (CompressionInfo, account_type, compression_only) or None if parsing fails.
fn extract_compression_info(data: &[u8]) -> Option<(CompressionInfo, u8, bool)> {
    use light_zero_copy::traits::ZeroCopyAt;

    let account_type = determine_account_type(data)?;

    match account_type {
        ACCOUNT_TYPE_TOKEN_ACCOUNT => {
            let (ctoken, _) = Token::zero_copy_at(data).ok()?;
            let ext = ctoken.get_compressible_extension()?;

            let compression_info = CompressionInfo {
                config_account_version: ext.info.config_account_version.into(),
                compress_to_pubkey: ext.info.compress_to_pubkey,
                account_version: ext.info.account_version,
                lamports_per_write: ext.info.lamports_per_write.into(),
                compression_authority: ext.info.compression_authority,
                rent_sponsor: ext.info.rent_sponsor,
                last_claimed_slot: ext.info.last_claimed_slot.into(),
                rent_exemption_paid: ext.info.rent_exemption_paid.into(),
                _reserved: ext.info._reserved.into(),
                rent_config: RentConfig {
                    base_rent: ext.info.rent_config.base_rent.into(),
                    compression_cost: ext.info.rent_config.compression_cost.into(),
                    lamports_per_byte_per_epoch: ext.info.rent_config.lamports_per_byte_per_epoch,
                    max_funded_epochs: ext.info.rent_config.max_funded_epochs,
                    max_top_up: ext.info.rent_config.max_top_up.into(),
                },
            };
            let compression_only = ext.compression_only != 0;
            Some((compression_info, account_type, compression_only))
        }
        ACCOUNT_TYPE_MINT => {
            let mint = Mint::deserialize(&mut &data[..]).ok()?;
            // Mint accounts don't have compression_only, default to false
            Some((mint.compression, account_type, false))
        }
        _ => None,
    }
}

pub type CompressibleAccountStore = HashMap<Pubkey, StoredCompressibleAccount>;

#[derive(Eq, Hash, PartialEq)]
pub struct StoredCompressibleAccount {
    pub pubkey: Pubkey,
    pub last_paid_slot: u64,
    pub compression: CompressionInfo,
    /// Account type: ACCOUNT_TYPE_TOKEN_ACCOUNT (2) or ACCOUNT_TYPE_MINT (1)
    pub account_type: u8,
    /// Whether this is a compression-only account (affects batching)
    pub compression_only: bool,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct FundingPoolConfig {
    pub compressible_config_pda: Pubkey,
    pub compression_authority_pda: Pubkey,
    pub compression_authority_pda_bump: u8,
    /// rent_sponsor == pool pda
    pub rent_sponsor_pda: Pubkey,
    pub rent_sponsor_pda_bump: u8,
}

impl FundingPoolConfig {
    pub fn new(version: u16) -> Self {
        let config = CtokenCompressibleConfig::new_light_token(
            version,
            true,
            Pubkey::default(),
            Pubkey::default(),
            RentConfig::default(),
        );
        let compressible_config =
            CtokenCompressibleConfig::derive_pda(&REGISTRY_PROGRAM_ID, version).0;
        Self {
            compressible_config_pda: compressible_config,
            rent_sponsor_pda: config.rent_sponsor,
            rent_sponsor_pda_bump: config.rent_sponsor_bump,
            compression_authority_pda: config.compression_authority,
            compression_authority_pda_bump: config.compression_authority_bump,
        }
    }

    pub fn get_v1() -> Self {
        Self::new(1)
    }
}

pub async fn claim_and_compress(
    rpc: &mut LightProgramTest,
    stored_compressible_accounts: &mut CompressibleAccountStore,
) -> Result<(), RpcError> {
    use crate::forester::{claim_forester, compress_and_close_forester};

    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();
    let payer = rpc.get_payer().insecure_clone();

    // Get all compressible token/mint accounts (both Token and Mint)
    let compressible_ctoken_accounts = rpc
        .context
        .get_program_accounts(&Pubkey::from(LIGHT_TOKEN_PROGRAM_ID));

    // CToken base accounts are 165 bytes, filter above that to exclude empty/minimal accounts
    for account in compressible_ctoken_accounts
        .iter()
        .filter(|e| e.1.data.len() >= 165 && e.1.lamports > 0)
    {
        // Extract compression info, account type, and compression_only
        let Some((compression, account_type, compression_only)) =
            extract_compression_info(&account.1.data)
        else {
            continue;
        };

        let base_lamports = rpc
            .get_minimum_balance_for_rent_exemption(account.1.data.len())
            .await
            .unwrap();
        let last_funded_epoch = compression
            .get_last_funded_epoch(
                account.1.data.len() as u64,
                account.1.lamports,
                base_lamports,
            )
            .unwrap();
        let last_funded_slot = last_funded_epoch * SLOTS_PER_EPOCH;
        stored_compressible_accounts.insert(
            account.0,
            StoredCompressibleAccount {
                pubkey: account.0,
                last_paid_slot: last_funded_slot,
                compression,
                account_type,
                compression_only,
            },
        );
    }

    let current_slot = rpc.get_slot().await?;

    // Separate accounts by type and compression_only setting
    let mut compress_accounts_compression_only = Vec::new();
    let mut compress_accounts_normal = Vec::new();
    let mut compress_mint_accounts = Vec::new();
    let mut claim_accounts = Vec::new();

    // For each stored account, determine action using AccountRentState
    for (pubkey, stored_account) in stored_compressible_accounts.iter() {
        let account = rpc.get_account(*pubkey).await?.unwrap();
        let rent_exemption = rpc
            .get_minimum_balance_for_rent_exemption(account.data.len())
            .await?;

        use light_compressible::rent::AccountRentState;

        let compression = &stored_account.compression;

        // Create state for rent calculation
        let state = AccountRentState {
            num_bytes: account.data.len() as u64,
            current_slot,
            current_lamports: account.lamports,
            last_claimed_slot: compression.last_claimed_slot,
        };

        // Check what action is needed
        match state.calculate_claimable_rent(&compression.rent_config, rent_exemption) {
            None => {
                // Account is compressible (has rent deficit)
                if stored_account.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT {
                    // CToken accounts - separate by compression_only
                    if stored_account.compression_only {
                        compress_accounts_compression_only.push(*pubkey);
                    } else {
                        compress_accounts_normal.push(*pubkey);
                    }
                } else if stored_account.account_type == ACCOUNT_TYPE_MINT {
                    // Mint accounts - use mint_action flow
                    compress_mint_accounts.push(*pubkey);
                }
            }
            Some(claimable_amount) if claimable_amount > 0 => {
                // Has rent to claim from completed epochs
                // Both Token and Mint can be claimed
                claim_accounts.push(*pubkey);
            }
            Some(_) => {
                // Well-funded, nothing to claim (0 completed epochs)
                // Do nothing - skip this account
            }
        }
    }

    // Process claimable accounts in batches
    for token_accounts in claim_accounts.as_slice().chunks(20) {
        claim_forester(rpc, token_accounts, &forester_keypair, &payer).await?;
    }

    // Process compressible accounts in batches, separated by compression_only setting
    // This prevents TlvExtensionLengthMismatch errors when batching accounts together
    const BATCH_SIZE: usize = 10;

    // Process compression_only=true CToken accounts
    for chunk in compress_accounts_compression_only.chunks(BATCH_SIZE) {
        compress_and_close_forester(rpc, chunk, &forester_keypair, &payer, None).await?;
        for account_pubkey in chunk {
            stored_compressible_accounts.remove(account_pubkey);
        }
    }

    // Process compression_only=false CToken accounts
    for chunk in compress_accounts_normal.chunks(BATCH_SIZE) {
        compress_and_close_forester(rpc, chunk, &forester_keypair, &payer, None).await?;
        for account_pubkey in chunk {
            stored_compressible_accounts.remove(account_pubkey);
        }
    }

    // Process Mint accounts via mint_action
    for mint_pubkey in compress_mint_accounts {
        compress_mint_forester(rpc, mint_pubkey, &payer).await?;
        stored_compressible_accounts.remove(&mint_pubkey);
    }

    Ok(())
}

pub async fn auto_compress_program_pdas(
    rpc: &mut LightProgramTest,
    program_id: Pubkey,
) -> Result<(), RpcError> {
    use solana_instruction::AccountMeta;
    use solana_sdk::signature::Signer;

    let payer = rpc.get_payer().insecure_clone();

    let config_pda = LightConfig::derive_pda(&program_id, 0).0;

    let cfg_acc_opt = rpc.get_account(config_pda).await?;
    let Some(cfg_acc) = cfg_acc_opt else {
        return Ok(());
    };
    let cfg = LightConfig::try_from_slice(&cfg_acc.data)
        .map_err(|e| RpcError::CustomError(format!("config deserialize: {e:?}")))?;
    let rent_sponsor = cfg.rent_sponsor;
    // compression_authority is the payer by default for auto-compress
    let compression_authority = payer.pubkey();
    let address_tree = cfg.address_space[0];

    let program_accounts = rpc.context.get_program_accounts(&program_id);

    if program_accounts.is_empty() {
        return Ok(());
    }

    // CompressAccountsIdempotent struct expects 4 accounts:
    // 1. fee_payer (signer, writable)
    // 2. config (read-only)
    // 3. rent_sponsor (writable)
    // 4. compression_authority (writable - per generated struct)
    let program_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new(compression_authority, false),
    ];

    const BATCH_SIZE: usize = 5;
    let mut chunk = Vec::with_capacity(BATCH_SIZE);
    for (pubkey, account) in program_accounts
        .into_iter()
        .filter(|(_, acc)| acc.lamports > 0 && !acc.data.is_empty())
    {
        chunk.push((pubkey, account));
        if chunk.len() == BATCH_SIZE {
            try_compress_chunk(rpc, &program_id, &chunk, &program_metas, &address_tree).await;
            chunk.clear();
        }
    }

    if !chunk.is_empty() {
        try_compress_chunk(rpc, &program_id, &chunk, &program_metas, &address_tree).await;
    }

    Ok(())
}

async fn try_compress_chunk(
    rpc: &mut LightProgramTest,
    program_id: &Pubkey,
    chunk: &[(Pubkey, solana_sdk::account::Account)],
    program_metas: &[solana_instruction::AccountMeta],
    address_tree: &Pubkey,
) {
    use light_client::{indexer::Indexer, interface::instructions};
    use light_compressed_account::address::derive_address;
    use solana_sdk::signature::Signer;

    // Attempt compression per-account idempotently.
    for (pda, _acc) in chunk.iter() {
        // v2 address derive using PDA as seed
        let addr = derive_address(
            &pda.to_bytes(),
            &address_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // Only proceed if a compressed account exists
        let Ok(resp) = rpc.get_compressed_account(addr, None).await else {
            continue;
        };
        let Some(cacc) = resp.value else {
            continue;
        };

        // Fetch proof for this single account hash
        let Ok(proof_with_context) = rpc
            .get_validity_proof(vec![cacc.hash], vec![], None)
            .await
            .map(|r| r.value)
        else {
            continue;
        };

        // Build compress instruction
        let Ok(ix) = instructions::build_compress_accounts_idempotent(
            program_id,
            &instructions::COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*pda],
            program_metas,
            proof_with_context,
        )
        .map_err(|e| e.to_string()) else {
            continue;
        };

        let payer = rpc.get_payer().insecure_clone();
        let payer_pubkey = payer.pubkey();

        // Ignore errors to continue compressing other PDAs
        let _ = rpc
            .create_and_send_transaction(std::slice::from_ref(&ix), &payer_pubkey, &[&payer])
            .await;
    }
}

/// Compress and close a Mint account via mint_action instruction.
/// Mint uses MintAction::CompressAndCloseMint flow instead of registry compress_and_close.
async fn compress_mint_forester(
    rpc: &mut LightProgramTest,
    mint_pubkey: Pubkey,
    payer: &solana_sdk::signature::Keypair,
) -> Result<(), RpcError> {
    use light_client::indexer::Indexer;
    use light_compressed_account::instruction_data::traits::LightInstructionData;
    use light_compressible::config::CompressibleConfig;
    use light_token::compressed_token::mint_action::MintActionMetaConfig;
    use light_token_interface::instructions::mint_action::{
        CompressAndCloseMintAction, MintActionCompressedInstructionData, MintWithContext,
    };
    use solana_sdk::signature::Signer;

    // Get Mint account data
    let mint_account = rpc
        .get_account(mint_pubkey)
        .await?
        .ok_or_else(|| RpcError::CustomError(format!("Mint account {} not found", mint_pubkey)))?;

    // Deserialize Mint to get compressed_address and rent_sponsor
    let mint: Mint = BorshDeserialize::deserialize(&mut mint_account.data.as_slice())
        .map_err(|e| RpcError::CustomError(format!("Failed to deserialize Mint: {:?}", e)))?;

    let compressed_mint_address = mint.metadata.compressed_address();
    let rent_sponsor = Pubkey::from(mint.compression.rent_sponsor);

    // Get the compressed mint account from indexer
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "Compressed mint {:?}",
            compressed_mint_address
        )))?;

    // Get validity proof
    let rpc_proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await?
        .value;

    // Build compressed mint inputs
    // IMPORTANT: Set mint to None when Mint is decompressed
    // This tells on-chain code to read mint data from Mint Solana account
    // (not from instruction data which would have stale compression_info)
    let compressed_mint_inputs = MintWithContext {
        prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
        leaf_index: compressed_mint_account.leaf_index,
        root_index: rpc_proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: None, // Mint is decompressed, data lives in Mint account
    };

    // Build instruction data with CompressAndCloseMint action
    let instruction_data = MintActionCompressedInstructionData::new(
        compressed_mint_inputs,
        rpc_proof_result.proof.into(),
    )
    .with_compress_and_close_mint(CompressAndCloseMintAction { idempotent: 1 });

    // Get state tree info
    let state_tree_info = rpc_proof_result.accounts[0].tree_info;

    // Build account metas - authority can be anyone for permissionless CompressAndCloseMint
    let config_address = CompressibleConfig::light_token_v1_config_pda();
    let meta_config = MintActionMetaConfig::new(
        payer.pubkey(),
        payer.pubkey(), // authority doesn't matter for CompressAndCloseMint
        state_tree_info.tree,
        state_tree_info.queue,
        state_tree_info.queue,
    )
    .with_compressible_mint(mint_pubkey, config_address, rent_sponsor);

    let account_metas = meta_config.to_account_metas();

    // Serialize instruction data
    let data = instruction_data
        .data()
        .map_err(|e| RpcError::CustomError(format!("Failed to serialize instruction: {:?}", e)))?;

    // Build instruction
    let instruction = solana_instruction::Instruction {
        program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    };

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(())
}
