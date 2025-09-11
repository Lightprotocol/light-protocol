use std::collections::HashMap;

use borsh::BorshDeserialize;
use light_client::rpc::{Rpc, RpcError};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Inputs,
    },
};
use light_ctoken_types::{
    state::{CompressedToken, ExtensionStruct, SLOTS_PER_EPOCH},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_sdk::instruction::PackedAccounts;
use solana_pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

use crate::{accounts::test_keypairs::RENT_AUTHORITY_TEST_KEYPAIR, LightProgramTest};

pub type CompressibleAccountStore = HashMap<Pubkey, StoredCompressibleAccount>;

#[derive(Eq, Hash, PartialEq)]
pub struct StoredCompressibleAccount {
    pub pubkey: Pubkey,
    pub last_paid_slot: u64,
    pub account: CompressedToken,
}

#[derive(Debug, PartialEq)]
pub struct FundingPoolConfig {
    pub rent_authority_keypair: Keypair,
    pub rent_authority_pubkey: Pubkey,
    pub pool_pda: Pubkey,
    pub pool_pda_bump: u8,
}

impl FundingPoolConfig {
    pub fn new(rent_authority_keypair: Keypair) -> Self {
        // Derive pool PDA to fund it
        let (pool_pda, pool_pda_bump) = light_compressed_token_sdk::instructions::derive_pool_pda(
            &rent_authority_keypair.pubkey(),
        );

        Self {
            rent_authority_pubkey: rent_authority_keypair.pubkey(),
            rent_authority_keypair,
            pool_pda,
            pool_pda_bump,
        }
    }

    pub fn get_local_test_validator() -> Self {
        Self::new(Keypair::from_bytes(RENT_AUTHORITY_TEST_KEYPAIR.as_slice()).unwrap())
    }
}

impl Clone for FundingPoolConfig {
    fn clone(&self) -> Self {
        Self {
            rent_authority_keypair: Keypair::from_bytes(&self.rent_authority_keypair.to_bytes())
                .expect("Failed to clone Keypair"),
            rent_authority_pubkey: self.rent_authority_pubkey,
            pool_pda: self.pool_pda,
            pool_pda_bump: self.pool_pda_bump,
        }
    }
}

pub async fn claim_and_compress(
    rpc: &mut LightProgramTest,
    stored_compressible_accounts: &mut CompressibleAccountStore,
) -> Result<(), RpcError> {
    let funding_pool_config = rpc.test_accounts.funding_pool_config.clone();
    let payer_pubkey = rpc
        .test_accounts
        .funding_pool_config
        .rent_authority_keypair
        .pubkey();
    let compressible_ctoken_accounts = rpc
        .context
        .get_program_accounts(&light_compressed_token::ID);

    for account in compressible_ctoken_accounts
        .iter()
        .filter(|e| e.1.data.len() > 200 && e.1.lamports > 0)
    {
        let des_account = CompressedToken::deserialize(&mut account.1.data.as_slice())?;
        if let Some(extensions) = des_account.extensions.as_ref() {
            for extension in extensions.iter() {
                if let ExtensionStruct::Compressible(e) = extension {
                    let base_lamports = rpc
                        .get_minimum_balance_for_rent_exemption(
                            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize,
                        )
                        .await
                        .unwrap();
                    let last_paid_epoch = e
                        .get_last_paid_epoch(
                            account.1.data.len() as u64,
                            account.1.lamports,
                            base_lamports,
                        )
                        .unwrap();
                    let last_paid_slot = last_paid_epoch * SLOTS_PER_EPOCH;
                    stored_compressible_accounts.insert(
                        account.0,
                        StoredCompressibleAccount {
                            pubkey: account.0,
                            last_paid_slot,
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

    for token_accounts in claim_able_accounts.as_slice().chunks(20) {
        println!("Claim from : {:?}", token_accounts);
        // Build claim instruction for all 10 accounts
        let claim_instruction = light_compressed_token_sdk::instructions::claim(
            funding_pool_config.pool_pda,
            funding_pool_config.pool_pda_bump,
            funding_pool_config.rent_authority_pubkey,
            token_accounts,
        );
        let signers = [&funding_pool_config.rent_authority_keypair];
        // Execute claim transaction
        rpc.create_and_send_transaction(&[claim_instruction], &payer_pubkey, &signers)
            .await?;
    }

    // Process compressible accounts in batches
    const BATCH_SIZE: usize = 10; // Process up to 10 accounts at a time
    let mut pubkeys = Vec::with_capacity(compressible_accounts.len());
    for chunk in compressible_accounts.chunks(BATCH_SIZE) {
        println!(
            "Compress and close: {:?}",
            chunk.iter().map(|e| e.pubkey).collect::<Vec<_>>()
        );
        // Call manual implementation for batch processing
        compress_and_close_batch(
            rpc,
            chunk,
            &funding_pool_config.rent_authority_keypair,
            &funding_pool_config.rent_authority_keypair,
        )
        .await?;

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

/// Manual implementation of compress_and_close for a slice of token accounts
/// Uses the already deserialized account data from stored_compressible_accounts
async fn compress_and_close_batch(
    rpc: &mut LightProgramTest,
    compressible_token_accounts: &[&StoredCompressibleAccount],
    rent_authority_keypair: &Keypair,
    payer: &Keypair,
) -> Result<(), RpcError> {
    if compressible_token_accounts.is_empty() {
        return Ok(());
    }

    // Get output queue for compression
    let output_queue = rpc.get_random_state_tree_info()?.get_output_pubkey()?;

    // Build packed accounts and token accounts for all accounts to compress
    let mut packed_accounts = PackedAccounts::default();
    let mut token_accounts = Vec::with_capacity(compressible_token_accounts.len());

    // Add output queue first
    let output_queue_index = packed_accounts.insert_or_get(output_queue);

    for StoredCompressibleAccount {
        pubkey,
        last_paid_slot: _,
        account,
    } in compressible_token_accounts
    {
        let compressed_token = account;

        // Extract necessary data (convert from light_compressed_account::Pubkey to solana_sdk::Pubkey)
        let mint = Pubkey::from(compressed_token.mint.to_bytes());
        let owner = Pubkey::from(compressed_token.owner.to_bytes());
        let amount = compressed_token.amount;

        // Get rent authority and recipient from compressible extension
        let (rent_authority, rent_recipient) =
            if let Some(extensions) = &compressed_token.extensions {
                let mut authority = owner;
                let mut recipient = owner;

                for extension in extensions {
                    if let ExtensionStruct::Compressible(ext) = extension {
                        if ext.rent_authority != [0u8; 32] {
                            authority = Pubkey::from(ext.rent_authority);
                        }
                        if ext.rent_recipient != [0u8; 32] {
                            recipient = Pubkey::from(ext.rent_recipient);
                        }
                        break;
                    }
                }
                (authority, recipient)
            } else {
                (owner, owner)
            };

        // Verify the rent authority matches the expected signer
        if rent_authority != rent_authority_keypair.pubkey() {
            return Err(RpcError::CustomError(format!(
                "Rent authority mismatch for account {}: expected {}, got {}",
                pubkey,
                rent_authority_keypair.pubkey(),
                rent_authority
            )));
        }

        // Add accounts to packed_accounts
        let source_index = packed_accounts.insert_or_get(*pubkey);
        let mint_index = packed_accounts.insert_or_get(mint);
        let owner_index = packed_accounts.insert_or_get(owner);
        let authority_index = packed_accounts.insert_or_get_config(rent_authority, true, false);
        let rent_recipient_index = packed_accounts.insert_or_get(rent_recipient);

        // Create CTokenAccount2 for CompressAndClose operation
        let mut token_account =
            CTokenAccount2::new_empty(owner_index, mint_index, output_queue_index);

        // Set up compress_and_close
        token_account
            .compress_and_close(
                amount,
                source_index,
                authority_index,
                rent_recipient_index,
                token_accounts.len() as u8, // Index in the output array
            )
            .map_err(|e| {
                RpcError::CustomError(format!("Failed to setup compress_and_close: {:?}", e))
            })?;

        token_accounts.push(token_account);
    }

    // Create the transfer2 instruction
    let meta_config =
        Transfer2AccountsMetaConfig::new(payer.pubkey(), packed_accounts.to_account_metas().0);

    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        ..Default::default()
    };

    let instruction = create_transfer2_instruction(inputs)
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    // Prepare signers
    let mut signers = vec![payer];
    if rent_authority_keypair.pubkey() != payer.pubkey() {
        signers.push(rent_authority_keypair);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await?;

    Ok(())
}
