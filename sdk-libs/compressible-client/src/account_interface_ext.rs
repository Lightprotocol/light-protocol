//! Extension trait for unified hot/cold account interfaces.
//!
//! Blanket-implemented for `Rpc + Indexer`.

use async_trait::async_trait;
use borsh::BorshDeserialize as _;
use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{Rpc, RpcError},
};
use light_compressed_account::address::derive_address;
use light_token_interface::{state::Mint, CMINT_ADDRESS_TREE};
use light_token_sdk::token::{derive_mint_compressed_address, derive_token_ata, find_mint_address};
use solana_pubkey::Pubkey;

use crate::{AccountInfoInterface, AtaInterface, MintInterface, MintState, TokenAccountInterface};

fn indexer_err(e: impl std::fmt::Display) -> RpcError {
    RpcError::CustomError(format!("IndexerError: {}", e))
}

/// Extension trait for fetching unified hot/cold account interfaces.
///
/// Blanket-implemented for all `Rpc + Indexer` types.
/// TODO: move to server endpoint.
#[async_trait]
pub trait AccountInterfaceExt: Rpc + Indexer {
    /// Fetch MintInterface for a mint signer.
    async fn get_mint_interface(&self, signer: &Pubkey) -> Result<MintInterface, RpcError>;

    /// Fetch AccountInfoInterface for a rent-free PDA.
    async fn get_account_info_interface(
        &self,
        address: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<AccountInfoInterface, RpcError>;

    /// Fetch TokenAccountInterface for a token account address.
    async fn get_token_account_interface(
        &self,
        address: &Pubkey,
    ) -> Result<TokenAccountInterface, RpcError>;

    /// Fetch AtaInterface for an (owner, mint) pair.
    async fn get_ata_interface(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<AtaInterface, RpcError>;
}

#[async_trait]
impl<T: Rpc + Indexer> AccountInterfaceExt for T {
    async fn get_mint_interface(&self, signer: &Pubkey) -> Result<MintInterface, RpcError> {
        let (cmint, _) = find_mint_address(signer);
        let address_tree = Pubkey::new_from_array(CMINT_ADDRESS_TREE);
        let compressed_address = derive_mint_compressed_address(signer, &address_tree);

        // On-chain first
        if let Some(account) = self.get_account(cmint).await? {
            return Ok(MintInterface {
                cmint,
                signer: *signer,
                address_tree,
                compressed_address,
                state: MintState::Hot { account },
            });
        }

        // Compressed state
        let result = self
            .get_compressed_account(compressed_address, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value {
            if let Some(data) = compressed.data.as_ref() {
                if !data.data.is_empty() {
                    if let Ok(mint_data) = Mint::try_from_slice(&data.data) {
                        return Ok(MintInterface {
                            cmint,
                            signer: *signer,
                            address_tree,
                            compressed_address,
                            state: MintState::Cold {
                                compressed,
                                mint_data,
                            },
                        });
                    }
                }
            }
        }

        Ok(MintInterface {
            cmint,
            signer: *signer,
            address_tree,
            compressed_address,
            state: MintState::None,
        })
    }

    async fn get_account_info_interface(
        &self,
        address: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<AccountInfoInterface, RpcError> {
        let address_tree = self.get_address_tree_v2().tree;
        let compressed_address = derive_address(
            &address.to_bytes(),
            &address_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // On-chain first
        if let Some(account) = self.get_account(*address).await? {
            return Ok(AccountInfoInterface::hot(*address, account));
        }

        // Compressed state
        let result = self
            .get_compressed_account(compressed_address, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value {
            if compressed.data.as_ref().is_some_and(|d| !d.data.is_empty()) {
                return Ok(AccountInfoInterface::cold(
                    *address,
                    compressed,
                    *program_id,
                ));
            }
        }

        // Doesn't exist
        let account = solana_account::Account {
            lamports: 0,
            data: vec![],
            owner: *program_id,
            executable: false,
            rent_epoch: 0,
        };
        Ok(AccountInfoInterface::hot(*address, account))
    }

    async fn get_token_account_interface(
        &self,
        address: &Pubkey,
    ) -> Result<TokenAccountInterface, RpcError> {
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;

        // On-chain first
        if let Some(account) = self.get_account(*address).await? {
            return TokenAccountInterface::hot(*address, account)
                .map_err(|e| RpcError::CustomError(format!("parse error: {}", e)));
        }

        // Compressed state
        let result = self
            .get_compressed_token_accounts_by_owner(address, None, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value.items.into_iter().next() {
            let mint = compressed.token.mint;
            return Ok(TokenAccountInterface::cold(
                *address,
                compressed,
                *address,
                mint,
                0,
                LIGHT_TOKEN_PROGRAM_ID.into(),
            ));
        }

        Err(RpcError::CustomError(format!(
            "token account not found: {}",
            address
        )))
    }

    async fn get_ata_interface(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<AtaInterface, RpcError> {
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;

        let (ata, bump) = derive_token_ata(owner, mint);

        // On-chain first
        if let Some(account) = self.get_account(ata).await? {
            let inner = TokenAccountInterface::hot(ata, account)
                .map_err(|e| RpcError::CustomError(format!("parse error: {}", e)))?;
            return Ok(AtaInterface::new(inner));
        }

        // Compressed state
        let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(
            Some(*mint),
        ));
        let result = self
            .get_compressed_token_accounts_by_owner(&ata, options, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value.items.into_iter().next() {
            let inner = TokenAccountInterface::cold(
                ata,
                compressed,
                *owner,
                *mint,
                bump,
                LIGHT_TOKEN_PROGRAM_ID.into(),
            );
            return Ok(AtaInterface::new(inner));
        }

        Err(RpcError::CustomError(format!(
            "ATA not found: owner={} mint={}",
            owner, mint
        )))
    }
}
