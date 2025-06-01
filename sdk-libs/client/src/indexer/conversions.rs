use std::str::FromStr;

use crate::indexer::base58::Base58Conversions;
use light_compressed_account::compressed_account::{
    CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
};
use light_sdk::token::{AccountState, TokenData, TokenDataWithMerkleContext};
use photon_api::models::{
    Account as PhotonAccount, TokenAccount, TokenAccountList,
};
use solana_pubkey::Pubkey;

pub trait IntoPhotonAccount {
    fn into_photon_account(self) -> PhotonAccount;
}

pub trait IntoPhotonTokenAccount {
    fn into_photon_token_account(self) -> TokenAccount;
}

impl IntoPhotonAccount for CompressedAccountWithMerkleContext {
    fn into_photon_account(self) -> PhotonAccount {
        let address = self.compressed_account.address.map(|a| a.to_base58());
        let hash = self.hash().unwrap().to_base58();

        let mut account_data = None;
        if let Some(data) = &self.compressed_account.data {
            let data_bs64 = base64::encode(&*data.data);
            let discriminator = u64::from_be_bytes(data.discriminator);
            account_data = Some(Box::new(photon_api::models::account_data::AccountData {
                data: data_bs64,
                discriminator,
                data_hash: data.data_hash.to_base58(),
            }));
        }

        PhotonAccount {
            address,
            hash: hash.to_string(),
            lamports: self.compressed_account.lamports,
            data: account_data,
            owner: self.compressed_account.owner.to_string(),
            seq: None,
            slot_created: 0,
            leaf_index: self.merkle_context.leaf_index,
            tree: self.merkle_context.merkle_tree_pubkey.to_string(),
        }
    }
}

impl IntoPhotonTokenAccount for TokenDataWithMerkleContext {
    fn into_photon_token_account(self) -> TokenAccount {
        let base_account = self.compressed_account.into_photon_account();

        let mut tlv = None;
        if let Some(tlv_vec) = &self.token_data.tlv {
            tlv = Some(base64::encode(tlv_vec.as_slice()));
        }

        let token_data = photon_api::models::token_data::TokenData {
            mint: self.token_data.mint.to_string(),
            owner: self.token_data.owner.to_string(),
            amount: self.token_data.amount,
            delegate: self.token_data.delegate.map(|d| d.to_string()),
            state: match self.token_data.state {
                AccountState::Initialized => {
                    photon_api::models::account_state::AccountState::Initialized
                }
                AccountState::Frozen => photon_api::models::account_state::AccountState::Frozen,
            },
            tlv,
        };

        TokenAccount {
            account: Box::new(base_account),
            token_data: Box::new(token_data),
        }
    }
}

pub struct LocalPhotonAccount(PhotonAccount);

impl TryFrom<LocalPhotonAccount> for CompressedAccountWithMerkleContext {
    type Error = Box<dyn std::error::Error>;

    fn try_from(local_account: LocalPhotonAccount) -> Result<Self, Self::Error> {
        let account = local_account.0;
        let merkle_context = light_compressed_account::compressed_account::MerkleContext {
            merkle_tree_pubkey: Pubkey::from_str(&account.tree)?,
            queue_pubkey: Default::default(),
            leaf_index: account.leaf_index,
            prove_by_index: false,
            tree_type: light_compressed_account::TreeType::StateV1,
        };

        let mut compressed_account = CompressedAccount {
            address: account
                .address
                .map(|a| <[u8; 32]>::from_base58(&a).unwrap()),
            lamports: account.lamports,
            owner: Pubkey::from_str(&account.owner)?,
            data: None,
        };

        if let Some(data) = account.data {
            let data_decoded = base64::decode(&data.data)?;
            compressed_account.data = Some(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: data_decoded,
                data_hash: <[u8; 32]>::from_base58(&data.data_hash)?,
            });
        }

        Ok(CompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context,
        })
    }
}

pub trait FromPhotonTokenAccountList {
    fn into_token_data_vec(self) -> Vec<TokenDataWithMerkleContext>;
}

impl FromPhotonTokenAccountList for TokenAccountList {
    fn into_token_data_vec(self) -> Vec<TokenDataWithMerkleContext> {
        self.items
            .into_iter()
            .map(|item| {
                let token_data = TokenData {
                    mint: Pubkey::from_str(&item.token_data.mint).unwrap(),
                    owner: Pubkey::from_str(&item.token_data.owner).unwrap(),
                    amount: item.token_data.amount,
                    delegate: item
                        .token_data
                        .delegate
                        .map(|d| Pubkey::from_str(&d).unwrap()),
                    state: match item.token_data.state {
                        photon_api::models::AccountState::Initialized => AccountState::Initialized,
                        photon_api::models::AccountState::Frozen => AccountState::Frozen,
                    },
                    tlv: item.token_data.tlv.map(|t| base64::decode(&t).unwrap()),
                };

                let compressed_account =
                    CompressedAccountWithMerkleContext::try_from(LocalPhotonAccount(*item.account))
                        .unwrap();

                TokenDataWithMerkleContext {
                    token_data,
                    compressed_account,
                }
            })
            .collect()
    }
}
