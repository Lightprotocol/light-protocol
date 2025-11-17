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
