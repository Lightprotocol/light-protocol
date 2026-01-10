#[cfg(feature = "devenv")]
use std::collections::HashMap;

#[cfg(feature = "devenv")]
use anchor_lang::pubkey;
#[cfg(feature = "devenv")]
use borsh::BorshDeserialize;
#[cfg(feature = "devenv")]
use light_client::rpc::{Rpc, RpcError};
#[cfg(feature = "devenv")]
use light_compressible::compression_info::CompressionInfo;
#[cfg(feature = "devenv")]
use light_compressible::config::CompressibleConfig as CtokenCompressibleConfig;
#[cfg(feature = "devenv")]
use light_compressible::rent::RentConfig;
#[cfg(feature = "devenv")]
use light_compressible::rent::SLOTS_PER_EPOCH;
#[cfg(feature = "devenv")]
use light_ctoken_interface::state::{
    CToken, CompressedMint, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
#[cfg(feature = "devenv")]
use light_sdk::compressible::CompressibleConfig as CpdaCompressibleConfig;
#[cfg(feature = "devenv")]
use solana_pubkey::Pubkey;

#[cfg(feature = "devenv")]
use crate::{litesvm_extensions::LiteSvmExtensions, LightProgramTest};

/// Determines account type from account data.
/// - If account is exactly 165 bytes: CToken (legacy size without extensions)
/// - If account is > 165 bytes: read byte 165 for discriminator
/// - If account is < 165 bytes: invalid (returns None)
#[cfg(feature = "devenv")]
fn determine_account_type(data: &[u8]) -> Option<u8> {
    const ACCOUNT_TYPE_OFFSET: usize = 165;

    match data.len().cmp(&ACCOUNT_TYPE_OFFSET) {
        std::cmp::Ordering::Less => None,
        std::cmp::Ordering::Equal => Some(ACCOUNT_TYPE_TOKEN_ACCOUNT), // 165 bytes = CToken
        std::cmp::Ordering::Greater => Some(data[ACCOUNT_TYPE_OFFSET]),
    }
}

/// Extracts CompressionInfo, account type, and compression_only from account data.
/// Returns (CompressionInfo, account_type, compression_only) or None if parsing fails.
#[cfg(feature = "devenv")]
fn extract_compression_info(data: &[u8]) -> Option<(CompressionInfo, u8, bool)> {
    use light_zero_copy::traits::ZeroCopyAt;

    let account_type = determine_account_type(data)?;

    match account_type {
        ACCOUNT_TYPE_TOKEN_ACCOUNT => {
            let (ctoken, _) = CToken::zero_copy_at(data).ok()?;
            let ext = ctoken.get_compressible_extension()?;

            let compression_info = CompressionInfo {
                config_account_version: ext.info.config_account_version.into(),
                compress_to_pubkey: ext.info.compress_to_pubkey,
                account_version: ext.info.account_version,
                lamports_per_write: ext.info.lamports_per_write.into(),
                compression_authority: ext.info.compression_authority,
                rent_sponsor: ext.info.rent_sponsor,
                last_claimed_slot: ext.info.last_claimed_slot.into(),
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
            let cmint = CompressedMint::deserialize(&mut &data[..]).ok()?;
            // CMint accounts don't have compression_only, default to false
            Some((cmint.compression, account_type, false))
        }
        _ => None,
    }
}

#[cfg(feature = "devenv")]
pub type CompressibleAccountStore = HashMap<Pubkey, StoredCompressibleAccount>;

#[cfg(feature = "devenv")]
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

#[cfg(feature = "devenv")]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct FundingPoolConfig {
    pub compressible_config_pda: Pubkey,
    pub compression_authority_pda: Pubkey,
    pub compression_authority_pda_bump: u8,
    /// rent_sponsor == pool pda
    pub rent_sponsor_pda: Pubkey,
    pub rent_sponsor_pda_bump: u8,
}

#[cfg(feature = "devenv")]
impl FundingPoolConfig {
    pub fn new(version: u16) -> Self {
        let config = CtokenCompressibleConfig::new_ctoken(
            version,
            true,
            Pubkey::default(),
            Pubkey::default(),
            RentConfig::default(),
        );
        let compressible_config = CtokenCompressibleConfig::derive_pda(
            &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"),
            version,
        )
        .0;
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

#[cfg(feature = "devenv")]
pub async fn claim_and_compress(
    rpc: &mut LightProgramTest,
    stored_compressible_accounts: &mut CompressibleAccountStore,
) -> Result<(), RpcError> {
    use crate::forester::{claim_forester, compress_and_close_forester};

    let forester_keypair = rpc.test_accounts.protocol.forester.insecure_clone();
    let payer = rpc.get_payer().insecure_clone();

    // Get all compressible token/mint accounts (both CToken and CMint)
    let compressible_ctoken_accounts = rpc
        .context
        .get_program_accounts(&light_compressed_token::ID);

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

    // Separate accounts by compression_only setting to avoid TLV extension mismatch
    let mut compress_accounts_compression_only = Vec::new();
    let mut compress_accounts_normal = Vec::new();
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
                // Only CToken accounts can be compressed via compress_and_close_forester
                // CMint accounts have a different compression flow
                if stored_account.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT {
                    // Separate by compression_only to avoid batching incompatible accounts
                    if stored_account.compression_only {
                        compress_accounts_compression_only.push(*pubkey);
                    } else {
                        compress_accounts_normal.push(*pubkey);
                    }
                }
            }
            Some(claimable_amount) if claimable_amount > 0 => {
                // Has rent to claim from completed epochs
                // Both CToken and CMint can be claimed
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

    // Process compression_only=true accounts
    for chunk in compress_accounts_compression_only.chunks(BATCH_SIZE) {
        compress_and_close_forester(rpc, chunk, &forester_keypair, &payer, None).await?;
        for account_pubkey in chunk {
            stored_compressible_accounts.remove(account_pubkey);
        }
    }

    // Process compression_only=false accounts
    for chunk in compress_accounts_normal.chunks(BATCH_SIZE) {
        compress_and_close_forester(rpc, chunk, &forester_keypair, &payer, None).await?;
        for account_pubkey in chunk {
            stored_compressible_accounts.remove(account_pubkey);
        }
    }

    Ok(())
}

#[cfg(feature = "devenv")]
pub async fn auto_compress_program_pdas(
    rpc: &mut LightProgramTest,
    program_id: Pubkey,
) -> Result<(), RpcError> {
    use solana_instruction::AccountMeta;
    use solana_sdk::signature::Signer;

    let payer = rpc.get_payer().insecure_clone();

    let config_pda = CpdaCompressibleConfig::derive_pda(&program_id, 0).0;

    let cfg_acc_opt = rpc.get_account(config_pda).await?;
    let Some(cfg_acc) = cfg_acc_opt else {
        return Ok(());
    };
    let cfg = CpdaCompressibleConfig::try_from_slice(&cfg_acc.data)
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
    // 4. compression_authority (read-only)
    let program_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(rent_sponsor, false),
        AccountMeta::new_readonly(compression_authority, false),
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

#[cfg(feature = "devenv")]
async fn try_compress_chunk(
    rpc: &mut LightProgramTest,
    program_id: &Pubkey,
    chunk: &[(Pubkey, solana_sdk::account::Account)],
    program_metas: &[solana_instruction::AccountMeta],
    address_tree: &Pubkey,
) {
    use light_client::indexer::Indexer;
    use light_compressed_account::address::derive_address;
    use light_compressible_client::compressible_instruction;
    use solana_sdk::signature::Signer;

    // Attempt compression per-account idempotently.
    for (pda, acc) in chunk.iter() {
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

        // Build single-PDA compress instruction
        let Ok(ix) = compressible_instruction::compress_accounts_idempotent(
            program_id,
            &compressible_instruction::COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
            &[*pda],
            std::slice::from_ref(acc),
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
