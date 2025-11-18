#[cfg(feature = "devenv")]
use std::collections::HashMap;

#[cfg(feature = "devenv")]
use anchor_lang::pubkey;
#[cfg(feature = "devenv")]
use borsh::BorshDeserialize;
#[cfg(feature = "devenv")]
use light_client::rpc::{Rpc, RpcError};
#[cfg(feature = "devenv")]
use light_compressible::rent::SLOTS_PER_EPOCH;
#[cfg(feature = "devenv")]
use light_compressible::{config::CompressibleConfig, rent::RentConfig};
#[cfg(feature = "devenv")]
use light_ctoken_types::{
    state::{CToken, ExtensionStruct},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
#[cfg(feature = "devenv")]
use solana_pubkey::Pubkey;

#[cfg(feature = "devenv")]
use crate::{litesvm_extensions::LiteSvmExtensions, LightProgramTest};

#[cfg(feature = "devenv")]
pub type CompressibleAccountStore = HashMap<Pubkey, StoredCompressibleAccount>;

#[cfg(feature = "devenv")]
#[derive(Eq, Hash, PartialEq)]
pub struct StoredCompressibleAccount {
    pub pubkey: Pubkey,
    pub last_paid_slot: u64,
    pub account: CToken,
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
        let config = CompressibleConfig::new_ctoken(
            version,
            true,
            Pubkey::default(),
            Pubkey::default(),
            RentConfig::default(),
        );
        let compressible_config = CompressibleConfig::derive_pda(
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

    // Get all compressible token accounts
    let compressible_ctoken_accounts = rpc
        .context
        .get_program_accounts(&light_compressed_token::ID);

    for account in compressible_ctoken_accounts
        .iter()
        .filter(|e| e.1.data.len() > 200 && e.1.lamports > 0)
    {
        let des_account = CToken::deserialize(&mut account.1.data.as_slice())?;
        if let Some(extensions) = des_account.extensions.as_ref() {
            for extension in extensions.iter() {
                if let ExtensionStruct::Compressible(e) = extension {
                    let base_lamports = rpc
                        .get_minimum_balance_for_rent_exemption(
                            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize,
                        )
                        .await
                        .unwrap();
                    let last_funded_epoch = e
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
                            account: des_account.clone(),
                        },
                    );
                }
            }
        }
    }

    let current_slot = rpc.get_slot().await?;
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await?;

    let mut compress_accounts = Vec::new();
    let mut claim_accounts = Vec::new();

    // For each stored account, determine action using AccountRentState
    for (pubkey, stored_account) in stored_compressible_accounts.iter() {
        let account = rpc.get_account(*pubkey).await?.unwrap();

        // Get compressible extension
        if let Some(extensions) = stored_account.account.extensions.as_ref() {
            for extension in extensions.iter() {
                if let ExtensionStruct::Compressible(comp_ext) = extension {
                    use light_compressible::rent::AccountRentState;

                    // Create state for rent calculation
                    let state = AccountRentState {
                        num_bytes: account.data.len() as u64,
                        current_slot,
                        current_lamports: account.lamports,
                        last_claimed_slot: comp_ext.last_claimed_slot,
                    };

                    // Check what action is needed
                    match state.calculate_claimable_rent(&comp_ext.rent_config, rent_exemption) {
                        None => {
                            // Account is compressible (has rent deficit)
                            compress_accounts.push(*pubkey);
                        }
                        Some(claimable_amount) if claimable_amount > 0 => {
                            // Has rent to claim from completed epochs
                            claim_accounts.push(*pubkey);
                        }
                        Some(_) => {
                            // Well-funded, nothing to claim (0 completed epochs)
                            // Do nothing - skip this account
                        }
                    }
                }
            }
        }
    }

    // Process claimable accounts in batches
    for token_accounts in claim_accounts.as_slice().chunks(20) {
        println!(
            "Claim from {} accounts: {:?}",
            token_accounts.len(),
            token_accounts
        );
        claim_forester(rpc, token_accounts, &forester_keypair, &payer).await?;
    }

    // Process compressible accounts in batches
    const BATCH_SIZE: usize = 10;
    for chunk in compress_accounts.chunks(BATCH_SIZE) {
        println!("Compress and close {} accounts: {:?}", chunk.len(), chunk);
        compress_and_close_forester(rpc, chunk, &forester_keypair, &payer, None).await?;

        // Remove compressed accounts from HashMap
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

    let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;
    let cfg_acc = rpc
        .get_account(config_pda)
        .await?
        .ok_or_else(|| RpcError::CustomError("compressible config not found".into()))?;
    let cfg = CompressibleConfig::deserialize(&mut &cfg_acc.data[..])
        .map_err(|e| RpcError::CustomError(format!("config deserialize: {e:?}")))?;
    let rent_sponsor = cfg.rent_sponsor;
    let address_tree = cfg.address_space[0];

    let program_accounts = rpc.context.get_program_accounts(&program_id);
    if program_accounts.is_empty() {
        return Ok(());
    }

    let output_state_tree_info = rpc
        .get_random_state_tree_info()
        .map_err(|e| RpcError::CustomError(format!("no state tree: {e:?}")))?;

    let program_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(rent_sponsor, false),
    ];

    const BATCH_SIZE: usize = 5;
    let mut chunk = Vec::with_capacity(BATCH_SIZE);
    for (pubkey, account) in program_accounts
        .into_iter()
        .filter(|(_, acc)| acc.lamports > 0 && !acc.data.is_empty())
    {
        chunk.push((pubkey, account));
        if chunk.len() == BATCH_SIZE {
            try_compress_chunk(
                rpc,
                &program_id,
                &chunk,
                &program_metas,
                &address_tree,
                output_state_tree_info,
            )
            .await;
            chunk.clear();
        }
    }

    if !chunk.is_empty() {
        try_compress_chunk(
            rpc,
            &program_id,
            &chunk,
            &program_metas,
            &address_tree,
            output_state_tree_info,
        )
        .await;
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
    output_state_tree_info: light_client::indexer::TreeInfo,
) {
    use light_client::indexer::Indexer;
    use light_compressed_account::address::derive_address;
    use light_compressible_client::CompressibleInstruction;
    use solana_sdk::signature::Signer;

    let mut pdas = Vec::with_capacity(chunk.len());
    let mut accounts_to_compress = Vec::with_capacity(chunk.len());
    let mut hashes = Vec::with_capacity(chunk.len());
    for (pda, acc) in chunk.iter() {
        let addr = derive_address(
            &pda.to_bytes(),
            &address_tree.to_bytes(),
            &program_id.to_bytes(),
        );
        if let Ok(resp) = rpc.get_compressed_account(addr, None).await {
            if let Some(cacc) = resp.value {
                pdas.push(*pda);
                accounts_to_compress.push(acc.clone());
                hashes.push(cacc.hash);
            }
        }
    }
    if pdas.is_empty() {
        return;
    }

    let proof_with_context = match rpc.get_validity_proof(hashes, vec![], None).await {
        Ok(r) => r.value,
        Err(_) => return,
    };

    let signer_seeds: Vec<Vec<Vec<u8>>> = (0..pdas.len()).map(|_| Vec::new()).collect();

    let ix_res = CompressibleInstruction::compress_accounts_idempotent(
        program_id,
        &CompressibleInstruction::COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &pdas,
        &accounts_to_compress,
        program_metas,
        signer_seeds,
        proof_with_context,
        output_state_tree_info,
    )
    .map_err(|e| e.to_string());
    if let Ok(ix) = ix_res {
        let payer = rpc.get_payer().insecure_clone();
        let payer_pubkey = payer.pubkey();
        let _ = rpc
            .create_and_send_transaction(&[ix], &payer_pubkey, &[&payer])
            .await;
    }
}
