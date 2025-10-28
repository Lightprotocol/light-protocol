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
    let compressible_accounts = {
        stored_compressible_accounts
            .iter()
            .filter(|a| a.1.last_paid_slot < current_slot)
            .map(|e| e.1)
            .collect::<Vec<_>>()
    };

    let claim_able_accounts = stored_compressible_accounts
        .iter()
        .filter(|a| a.1.last_paid_slot >= current_slot)
        .map(|e| *e.0)
        .collect::<Vec<_>>();

    // Process claimable accounts in batches
    for token_accounts in claim_able_accounts.as_slice().chunks(20) {
        println!("Claim from : {:?}", token_accounts);
        // Use the new claim_forester function to claim via registry program
        claim_forester(rpc, token_accounts, &forester_keypair, &payer).await?;
    }

    // Process compressible accounts in batches
    const BATCH_SIZE: usize = 10; // Process up to 10 accounts at a time
    let mut pubkeys = Vec::with_capacity(compressible_accounts.len());
    for chunk in compressible_accounts.chunks(BATCH_SIZE) {
        let chunk_pubkeys: Vec<Pubkey> = chunk.iter().map(|e| e.pubkey).collect();
        println!("Compress and close: {:?}", chunk_pubkeys);

        // Use the new compress_and_close_forester function via registry program
        compress_and_close_forester(rpc, &chunk_pubkeys, &forester_keypair, &payer, None).await?;

        // Remove processed accounts from the HashMap
        for account_pubkey in chunk {
            pubkeys.push(account_pubkey.pubkey);
        }
    }
    for pubkey in pubkeys {
        stored_compressible_accounts.remove(&pubkey);
    }

    Ok(())
}
