use std::{fmt::Debug, str::FromStr};

use async_trait::async_trait;
use light_compressed_account::compressed_account::{
    CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext, MerkleContext,
};
use light_indexed_merkle_tree::array::IndexedElement;
use light_merkle_tree_metadata::QueueType;
use light_sdk::token::{AccountState, TokenData, TokenDataWithMerkleContext};
use num_bigint::BigUint;
use photon_api::models::{Account, TokenAccount, TokenAccountList, TokenBalanceList};
use solana_pubkey::Pubkey;

pub mod photon_indexer;

mod base58;
mod error;
mod types;

pub use base58::Base58Conversions;
pub use error::IndexerError;
//
pub use types::ProofRpcResultV2;
pub use types::{
    Address, AddressWithTree, Hash, MerkleProof, MerkleProofWithContext, ProofOfLeaf,
    ProofRpcResult,
};

#[derive(Debug, Clone)]
pub struct AddressQueueIndex {
    pub address: [u8; 32],
    pub queue_index: u64,
}

#[derive(Debug, Clone)]
pub struct BatchAddressUpdateIndexerResponse {
    pub batch_start_index: u64,
    pub addresses: Vec<AddressQueueIndex>,
    pub non_inclusion_proofs: Vec<NewAddressProofWithContext<40>>,
    pub subtrees: Vec<[u8; 32]>,
}

#[async_trait]
pub trait Indexer: std::marker::Send + std::marker::Sync {
    async fn get_indexer_slot(&self) -> Result<u64, IndexerError>;

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError>;

    async fn get_compressed_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError>;

    async fn get_compressed_token_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError>;

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<Account, IndexerError>;

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError>;

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError>;

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError>;

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
    ) -> Result<Vec<Account>, IndexerError>;

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<TokenBalanceList, IndexerError>;

    async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
    ) -> Result<Vec<String>, IndexerError>;

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError>;

    async fn get_multiple_new_address_proofs_h40(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError>;

    async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResult, IndexerError>;

    async fn get_validity_proof_v2(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<types::ProofRpcResultV2, IndexerError>;

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
    ) -> Result<BatchAddressUpdateIndexerResponse, IndexerError>;

    /// Returns queue elements from the queue with the given merkle tree pubkey. For input
    /// queues account compression program does not store queue elements in the
    /// account data but only emits these in the public transaction event. The
    /// indexer needs the queue elements to create batch update proofs.
    async fn get_queue_elements(
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        queue_type: QueueType,
        num_elements: u16,
        start_offset: Option<u64>,
    ) -> Result<Vec<MerkleProofWithContext>, IndexerError>;

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError>;
}

// For consistency with the Photon API.
#[derive(Clone, Debug, PartialEq)]
pub struct NewAddressProofWithContext<const NET_HEIGHT: usize> {
    pub merkle_tree: [u8; 32],
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; NET_HEIGHT],
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}

pub trait IntoPhotonAccount {
    fn into_photon_account(self) -> Account;
}

pub trait IntoPhotonTokenAccount {
    fn into_photon_token_account(self) -> TokenAccount;
}

impl IntoPhotonAccount for CompressedAccountWithMerkleContext {
    fn into_photon_account(self) -> Account {
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

        Account {
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

pub struct LocalPhotonAccount(Account);

impl TryFrom<LocalPhotonAccount> for CompressedAccountWithMerkleContext {
    type Error = Box<dyn std::error::Error>;

    fn try_from(local_account: LocalPhotonAccount) -> Result<Self, Self::Error> {
        let account = local_account.0;
        let merkle_context = MerkleContext {
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
