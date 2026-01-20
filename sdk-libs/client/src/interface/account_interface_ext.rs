use async_trait::async_trait;
use borsh::BorshDeserialize as _;
use light_compressed_account::address::derive_address;
use light_token::instruction::derive_token_ata;
use light_token_interface::{state::Mint, MINT_ADDRESS_TREE};
use solana_pubkey::Pubkey;

use super::{AccountInterface, AccountToFetch, MintInterface, MintState, TokenAccountInterface};
use crate::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{Rpc, RpcError},
};

fn indexer_err(e: impl std::fmt::Display) -> RpcError {
    RpcError::CustomError(format!("IndexerError: {}", e))
}

/// Extension trait for fetching account interfaces (unified hot/cold handling).
#[async_trait]
pub trait AccountInterfaceExt: Rpc + Indexer {
    /// Fetch MintInterface for a mint account.
    ///
    /// Use this instead of get_account + unpack_mint.
    async fn get_mint_interface(&self, address: &Pubkey) -> Result<MintInterface, RpcError>;

    /// Fetch AccountInterface for an account.
    ///
    /// Use this instead of get_account.
    async fn get_account_interface(
        &self,
        address: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<AccountInterface, RpcError>;

    /// Fetch TokenAccountInterface for a token account.
    ///
    /// Use this instead of get_token_account.
    async fn get_token_account_interface(
        &self,
        address: &Pubkey,
    ) -> Result<TokenAccountInterface, RpcError>;

    /// Fetch TokenAccountInterface for an associated token account.
    ///
    /// Use this for all ATAs.
    async fn get_ata_interface(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<TokenAccountInterface, RpcError>;

    /// Fetch multiple accounts with automatic type dispatch.
    ///
    /// Use this instead of get_multiple_accounts.
    async fn get_multiple_account_interfaces(
        &self,
        accounts: &[AccountToFetch],
    ) -> Result<Vec<AccountInterface>, RpcError>;
}

// TODO: move all these to native RPC methods with single roundtrip.
#[async_trait]
impl<T: Rpc + Indexer> AccountInterfaceExt for T {
    async fn get_mint_interface(&self, address: &Pubkey) -> Result<MintInterface, RpcError> {
        let address_tree = Pubkey::new_from_array(MINT_ADDRESS_TREE);
        let compressed_address = derive_address(
            &address.to_bytes(),
            &address_tree.to_bytes(),
            &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
        );

        // Hot
        if let Some(account) = self.get_account(*address).await? {
            if account.lamports > 0 {
                return Ok(MintInterface {
                    mint: *address,
                    address_tree,
                    compressed_address,
                    state: MintState::Hot { account },
                });
            }
        }

        // Cold
        let result = self
            .get_compressed_account(compressed_address, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value {
            if let Some(data) = compressed.data.as_ref() {
                if !data.data.is_empty() {
                    let mint_data = Mint::try_from_slice(&data.data)
                        .map_err(|e| RpcError::CustomError(format!("mint parse error: {}", e)))?;
                    return Ok(MintInterface {
                        mint: *address,
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

        Ok(MintInterface {
            mint: *address,
            address_tree,
            compressed_address,
            state: MintState::None,
        })
    }

    async fn get_account_interface(
        &self,
        address: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<AccountInterface, RpcError> {
        let address_tree = self.get_address_tree_v2().tree;
        let compressed_address = derive_address(
            &address.to_bytes(),
            &address_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // Hot
        if let Some(account) = self.get_account(*address).await? {
            if account.lamports > 0 {
                return Ok(AccountInterface::hot(*address, account));
            }
        }

        // Cold
        let result = self
            .get_compressed_account(compressed_address, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value {
            if compressed.data.as_ref().is_some_and(|d| !d.data.is_empty()) {
                return Ok(AccountInterface::cold(*address, compressed, *program_id));
            }
        }

        // Doesn't exist.
        let account = solana_account::Account {
            lamports: 0,
            data: vec![],
            owner: *program_id,
            executable: false,
            rent_epoch: 0,
        };
        Ok(AccountInterface::hot(*address, account))
    }

    async fn get_token_account_interface(
        &self,
        address: &Pubkey,
    ) -> Result<TokenAccountInterface, RpcError> {
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;

        // Hot
        if let Some(account) = self.get_account(*address).await? {
            if account.lamports > 0 {
                return TokenAccountInterface::hot(*address, account)
                    .map_err(|e| RpcError::CustomError(format!("parse error: {}", e)));
            }
        }

        // Cold (program-owned tokens: address = owner)
        let result = self
            .get_compressed_token_accounts_by_owner(address, None, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value.items.into_iter().next() {
            return Ok(TokenAccountInterface::cold(
                *address,
                compressed,
                *address, // owner = hot address
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
    ) -> Result<TokenAccountInterface, RpcError> {
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;

        let (ata, _bump) = derive_token_ata(owner, mint);

        // Hot
        if let Some(account) = self.get_account(ata).await? {
            if account.lamports > 0 {
                return TokenAccountInterface::hot(ata, account)
                    .map_err(|e| RpcError::CustomError(format!("parse error: {}", e)));
            }
        }

        // Cold (ATA query by address)
        let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(
            Some(*mint),
        ));
        let result = self
            .get_compressed_token_accounts_by_owner(&ata, options, None)
            .await
            .map_err(indexer_err)?;

        if let Some(compressed) = result.value.items.into_iter().next() {
            return Ok(TokenAccountInterface::cold(
                ata,
                compressed,
                *owner, // owner_override = wallet owner
                LIGHT_TOKEN_PROGRAM_ID.into(),
            ));
        }

        Err(RpcError::CustomError(format!(
            "ATA not found: owner={} mint={}",
            owner, mint
        )))
    }

    async fn get_multiple_account_interfaces(
        &self,
        accounts: &[AccountToFetch],
    ) -> Result<Vec<AccountInterface>, RpcError> {
        // TODO: concurrent with futures
        let mut result = Vec::with_capacity(accounts.len());

        for account in accounts {
            let iface = match account {
                AccountToFetch::Pda {
                    address,
                    program_id,
                } => self.get_account_interface(address, program_id).await?,
                AccountToFetch::Token { address } => {
                    let token_iface = self.get_token_account_interface(address).await?;
                    AccountInterface {
                        key: token_iface.key,
                        account: token_iface.account,
                        cold: token_iface.cold,
                    }
                }
                AccountToFetch::Ata { wallet_owner, mint } => {
                    let token_iface = self.get_ata_interface(wallet_owner, mint).await?;
                    AccountInterface {
                        key: token_iface.key,
                        account: token_iface.account,
                        cold: token_iface.cold,
                    }
                }
                AccountToFetch::Mint { address } => {
                    let mint_iface = self.get_mint_interface(address).await?;
                    match mint_iface.state {
                        MintState::Hot { account } => AccountInterface {
                            key: mint_iface.mint,
                            account,
                            cold: None,
                        },
                        MintState::Cold { compressed, .. } => {
                            let owner = compressed.owner;
                            AccountInterface::cold(mint_iface.mint, compressed, owner)
                        }
                        MintState::None => AccountInterface {
                            key: mint_iface.mint,
                            account: Default::default(),
                            cold: None,
                        },
                    }
                }
            };
            result.push(iface);
        }

        Ok(result)
    }
}
