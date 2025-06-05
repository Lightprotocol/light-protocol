use std::{fmt::Debug, time::Duration};

use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_batched_merkle_tree::{
    constants::{
        DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_ROOT_HISTORY_LEN,
        DEFAULT_BATCH_STATE_TREE_HEIGHT,
    },
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    fee::FeeConfig,
    indexer::{
        Account, AccountProofInputs, Address, AddressMerkleTreeAccounts, AddressProofInputs,
        AddressWithTree, BatchAddressUpdateIndexerResponse, Context,
        GetCompressedAccountsByOwnerConfig, GetCompressedTokenAccountsByOwnerOrDelegateOptions,
        Indexer, IndexerError, IndexerRpcConfig, Items, ItemsWithCursor, MerkleProof,
        MerkleProofWithContext, NewAddressProofWithContext, OwnerBalance, PaginatedOptions,
        Response, RetryConfig, SignatureWithMetadata, StateMerkleTreeAccounts, TokenAccount,
        TokenBalance, ValidityProofWithContext,
    },
    rpc::{Rpc, RpcError},
};
use light_compressed_account::{
    compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
    hash_chain::create_hash_chain_from_slice,
    indexer_event::event::PublicTransactionEvent,
    instruction_data::compressed_proof::CompressedProof,
    tx_hash::create_tx_hash,
    TreeType,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_merkle_tree_metadata::QueueType;
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    constants::{PROVE_PATH, SERVER_ADDRESS},
    helpers::{big_int_to_string, bigint_to_u8_32, string_to_big_int},
    proof::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    proof_type::ProofType,
    proof_types::{
        combined::{v1::CombinedJsonStruct as CombinedJsonStructLegacy, v2::CombinedJsonStruct},
        inclusion::{
            v1::{
                BatchInclusionJsonStruct as BatchInclusionJsonStructLegacy,
                InclusionProofInputs as InclusionProofInputsLegacy,
            },
            v2::{BatchInclusionJsonStruct, InclusionMerkleProofInputs, InclusionProofInputs},
        },
        non_inclusion::{
            v1::{
                BatchNonInclusionJsonStruct as BatchNonInclusionJsonStructLegacy,
                NonInclusionProofInputs as NonInclusionProofInputsLegacy,
            },
            v2::{BatchNonInclusionJsonStruct, NonInclusionProofInputs},
        },
    },
};
use light_sdk::{
    token::{TokenData, TokenDataWithMerkleContext},
    Hash,
};
use log::info;
use num_bigint::{BigInt, BigUint};
use num_traits::FromBytes;
use reqwest::Client;
use solana_sdk::{
    bs58,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use super::{
    address_tree::{AddressMerkleTreeBundle, IndexedMerkleTreeVersion},
    state_tree::{LeafIndexInfo, StateMerkleTreeBundle},
};
#[cfg(feature = "devenv")]
use crate::accounts::{
    address_tree_v2::create_batch_address_merkle_tree,
    state_tree_v2::create_batched_state_merkle_tree,
};
use crate::{
    accounts::{
        address_tree::create_address_merkle_tree_and_queue_account,
        state_tree::create_state_merkle_tree_and_queue_account, test_accounts::TestAccounts,
        test_keypairs::BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR,
    },
    indexer::TestIndexerExtensions,
};

#[derive(Debug)]
pub struct TestIndexer {
    pub state_merkle_trees: Vec<StateMerkleTreeBundle>,
    pub address_merkle_trees: Vec<AddressMerkleTreeBundle>,
    pub payer: Keypair,
    pub group_pda: Pubkey,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub events: Vec<PublicTransactionEvent>,
}

impl Clone for TestIndexer {
    fn clone(&self) -> Self {
        Self {
            state_merkle_trees: self.state_merkle_trees.clone(),
            address_merkle_trees: self.address_merkle_trees.clone(),
            payer: self.payer.insecure_clone(),
            group_pda: self.group_pda,
            compressed_accounts: self.compressed_accounts.clone(),
            nullified_compressed_accounts: self.nullified_compressed_accounts.clone(),
            token_compressed_accounts: self.token_compressed_accounts.clone(),
            token_nullified_compressed_accounts: self.token_nullified_compressed_accounts.clone(),
            events: self.events.clone(),
        }
    }
}

#[async_trait]
impl Indexer for TestIndexer {
    // TODO: add slot to test indexer struct
    async fn get_indexer_slot(&self, _config: Option<RetryConfig>) -> Result<u64, IndexerError> {
        // test indexer is always up to date
        Ok(u64::MAX)
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<[u8; 32]>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProof>>, IndexerError> {
        info!("Getting proofs for {:?}", hashes);
        let mut proofs: Vec<MerkleProof> = Vec::new();
        hashes.iter().for_each(|hash| {
            self.state_merkle_trees.iter().for_each(|tree| {
                if let Some(leaf_index) = tree.merkle_tree.get_leaf_index(hash) {
                    let proof = tree
                        .merkle_tree
                        .get_proof_of_leaf(leaf_index, false)
                        .unwrap();
                    proofs.push(MerkleProof {
                        hash: *hash,
                        leaf_index: leaf_index as u64,
                        merkle_tree: tree.accounts.merkle_tree,
                        proof: proof.to_vec(),
                        root_seq: tree.merkle_tree.sequence_number as u64,
                        root: *tree.merkle_tree.roots.last().unwrap(),
                    });
                }
            })
        });
        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: Items { items: proofs },
        })
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
        _options: Option<GetCompressedAccountsByOwnerConfig>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<Account>>, IndexerError> {
        let accounts_with_context = <TestIndexer as TestIndexerExtensions>::get_compressed_accounts_with_merkle_context_by_owner(self, owner);
        let accounts: Result<Vec<Account>, IndexerError> = accounts_with_context
            .into_iter()
            .map(|acc| acc.try_into())
            .collect();

        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: ItemsWithCursor {
                items: accounts?,
                cursor: None,
            },
        })
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Account>, IndexerError> {
        let account = match (address, hash) {
            (Some(address), _) => self
                .compressed_accounts
                .iter()
                .find(|acc| acc.compressed_account.address == Some(address)),
            (_, Some(hash)) => {
                let res = self
                    .compressed_accounts
                    .iter()
                    .find(|acc| acc.hash() == Ok(hash));
                // TODO: unify token accounts with compressed accounts.
                if res.is_none() {
                    let res = self
                        .token_compressed_accounts
                        .iter()
                        .find(|acc| acc.compressed_account.hash() == Ok(hash));
                    res.map(|x| &x.compressed_account)
                } else {
                    res
                }
            }
            (None, None) => {
                return Err(IndexerError::InvalidParameters(
                    "Either address or hash must be provided".to_string(),
                ))
            }
        };

        let account_data = account
            .ok_or(IndexerError::AccountNotFound)?
            .clone()
            .try_into()?;

        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: account_data,
        })
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        _options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenAccount>>, IndexerError> {
        let mint = _options.as_ref().and_then(|opts| opts.mint);
        let token_accounts: Result<Vec<TokenAccount>, IndexerError> = self
            .token_compressed_accounts
            .iter()
            .filter(|acc| {
                acc.token_data.owner == *owner && mint.is_none_or(|m| acc.token_data.mint == m)
            })
            .map(|acc| TokenAccount::try_from(acc.clone()))
            .collect();

        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: ItemsWithCursor {
                items: token_accounts?,
                cursor: None,
            },
        })
    }

    async fn get_compressed_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let account_response = self.get_compressed_account(address, hash, None).await?;
        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: account_response.value.lamports,
        })
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        let account = match (address, hash) {
            (Some(address), _) => self
                .token_compressed_accounts
                .iter()
                .find(|acc| acc.compressed_account.compressed_account.address == Some(address)),
            (_, Some(hash)) => self
                .token_compressed_accounts
                .iter()
                .find(|acc| acc.compressed_account.hash() == Ok(hash)),
            (None, None) => {
                return Err(IndexerError::InvalidParameters(
                    "Either address or hash must be provided".to_string(),
                ))
            }
        };

        let amount = account
            .map(|acc| acc.token_data.amount)
            .ok_or(IndexerError::AccountNotFound)?;

        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: amount,
        })
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<Account>>, IndexerError> {
        match (addresses, hashes) {
            (Some(addresses), _) => {
                let accounts = self
                    .compressed_accounts
                    .iter()
                    .filter(|acc| {
                        acc.compressed_account
                            .address
                            .is_some_and(|addr| addresses.contains(&addr))
                    })
                    .map(|acc| acc.clone().try_into())
                    .collect::<Result<Vec<Account>, IndexerError>>()?;
                Ok(Response {
                    context: Context {
                        slot: self.get_current_slot(),
                    },
                    value: Items { items: accounts },
                })
            }
            (_, Some(hashes)) => {
                let accounts = self
                    .compressed_accounts
                    .iter()
                    .filter(|acc| acc.hash().is_ok_and(|hash| hashes.contains(&hash)))
                    .map(|acc| acc.clone().try_into())
                    .collect::<Result<Vec<Account>, IndexerError>>()?;
                Ok(Response {
                    context: Context {
                        slot: self.get_current_slot(),
                    },
                    value: Items { items: accounts },
                })
            }
            (None, None) => Err(IndexerError::InvalidParameters(
                "Either addresses or hashes must be provided".to_string(),
            )),
        }
    }

    async fn get_compressed_token_balances_by_owner_v2(
        &self,
        owner: &Pubkey,
        _options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenBalance>>, IndexerError> {
        let mint = _options.as_ref().and_then(|opts| opts.mint);
        let balances: Vec<TokenBalance> = self
            .token_compressed_accounts
            .iter()
            .filter(|acc| {
                acc.token_data.owner == *owner && mint.is_none_or(|m| acc.token_data.mint == m)
            })
            .map(|acc| TokenBalance {
                balance: acc.token_data.amount,
                mint: acc.token_data.mint,
            })
            .collect();

        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: ItemsWithCursor {
                items: balances,
                cursor: None,
            },
        })
    }

    async fn get_compression_signatures_for_account(
        &self,
        _hash: Hash,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<SignatureWithMetadata>>, IndexerError> {
        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: Items { items: vec![] },
        })
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<NewAddressProofWithContext>>, IndexerError> {
        let proofs = self
            ._get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, false)
            .await?;
        Ok(Response {
            context: Context {
                slot: self.get_current_slot(),
            },
            value: Items { items: proofs },
        })
    }

    async fn get_validity_proof(
        &self,
        hashes: Vec<[u8; 32]>,
        new_addresses_with_trees: Vec<AddressWithTree>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ValidityProofWithContext>, IndexerError> {
        println!("hashes: {:?}", hashes);
        println!("new_addresses_with_trees: {:?}", new_addresses_with_trees);
        #[cfg(feature = "v2")]
        {
            // V2 implementation with queue handling
            let mut state_merkle_tree_pubkeys = Vec::new();

            for hash in hashes.iter() {
                let account = self.get_compressed_account(None, Some(*hash), None).await?;
                state_merkle_tree_pubkeys.push(account.value.merkle_context.tree);
            }
            let mut proof_inputs = vec![];

            let mut indices_to_remove = Vec::new();
            // for all accounts in batched trees, check whether values are in tree or queue
            let compressed_accounts = if !hashes.is_empty() && !state_merkle_tree_pubkeys.is_empty()
            {
                let zipped_accounts = hashes.iter().zip(state_merkle_tree_pubkeys.iter());

                for (i, (compressed_account, state_merkle_tree_pubkey)) in
                    zipped_accounts.enumerate()
                {
                    let accounts = self.state_merkle_trees.iter().find(|x| {
                        x.accounts.merkle_tree == *state_merkle_tree_pubkey
                            && x.tree_type == TreeType::StateV2
                    });

                    if let Some(accounts) = accounts {
                        let queue_element = accounts
                            .output_queue_elements
                            .iter()
                            .find(|(hash, _)| hash == compressed_account);
                        if let Some((_, index)) = queue_element {
                            if accounts.output_queue_batch_size.is_some()
                                && accounts.leaf_index_in_queue_range(*index as usize)?
                            {
                                indices_to_remove.push(i);
                                proof_inputs.push(AccountProofInputs {
                                    hash: *compressed_account,
                                    root: [0u8; 32],
                                    root_index: None,
                                    leaf_index: accounts
                                        .output_queue_elements
                                        .iter()
                                        .position(|(x, _)| x == compressed_account)
                                        .unwrap()
                                        as u64,
                                    tree_info: light_client::indexer::TreeInfo {
                                        cpi_context: Some(accounts.accounts.cpi_context),
                                        tree: accounts.accounts.merkle_tree,
                                        queue: accounts.accounts.nullifier_queue,
                                        next_tree_info: None,
                                        tree_type: accounts.tree_type,
                                    },
                                })
                            }
                        }
                    }
                }

                let compress_accounts = hashes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !indices_to_remove.contains(i))
                    .map(|(_, x)| *x)
                    .collect::<Vec<[u8; 32]>>();

                if compress_accounts.is_empty() {
                    None
                } else {
                    Some(compress_accounts)
                }
            } else {
                None
            };

            // Get the basic validity proof if needed
            let rpc_result: Option<ValidityProofWithContext> = if (compressed_accounts.is_some()
                && !compressed_accounts.as_ref().unwrap().is_empty())
                || !new_addresses_with_trees.is_empty()
            {
                Some(
                    self._get_validity_proof_v1_implementation(
                        compressed_accounts.unwrap_or_default(),
                        new_addresses_with_trees,
                    )
                    .await?,
                )
            } else {
                None
            };

            // Handle root indices with queue considerations
            let addresses = if let Some(rpc_result) = rpc_result.as_ref() {
                rpc_result.addresses.to_vec()
            } else {
                Vec::new()
            };
            let accounts = {
                let mut root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
                    rpc_result.accounts.to_vec()
                } else {
                    Vec::new()
                };
                println!("rpc_result {:?}", rpc_result);
                println!("root_indices: {:?}", root_indices);
                // reverse so that we can pop elements.
                proof_inputs.reverse();
                // Reinsert.
                for index in indices_to_remove.iter().rev() {
                    if root_indices.len() <= *index {
                        root_indices.push(proof_inputs.pop().unwrap());
                    } else {
                        root_indices.insert(*index, proof_inputs.pop().unwrap());
                    }
                }
                root_indices
                // root_indices.into_iter().map(|x| x.unwrap_or(0)).collect()
            };

            Ok(Response {
                context: Context {
                    slot: self.get_current_slot(),
                },
                value: ValidityProofWithContext {
                    accounts,
                    addresses,
                    proof: rpc_result
                        .map(|rpc_result| rpc_result.proof.0.unwrap())
                        .into(),
                },
            })
        }

        #[cfg(not(feature = "v2"))]
        {
            // V1 implementation - direct call to V1 logic
            let result = self
                ._get_validity_proof_v1_implementation(hashes, new_addresses_with_trees)
                .await?;
            Ok(Response {
                context: Context {
                    slot: self.get_current_slot(),
                },
                value: result,
            })
        }
    }

    async fn get_queue_elements(
        &mut self,
        _merkle_tree_pubkey: [u8; 32],
        _queue_type: QueueType,
        _num_elements: u16,
        _start_offset: Option<u64>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<MerkleProofWithContext>>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!("get_queue_elements");
        #[cfg(feature = "v2")]
        {
            let merkle_tree_pubkey = _merkle_tree_pubkey;
            let queue_type = _queue_type;
            let num_elements = _num_elements;
            let pubkey = Pubkey::new_from_array(merkle_tree_pubkey);
            let address_tree_bundle = self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == pubkey);
            if let Some(address_tree_bundle) = address_tree_bundle {
                let end_offset = std::cmp::min(
                    num_elements as usize,
                    address_tree_bundle.queue_elements.len(),
                );
                let queue_elements = address_tree_bundle.queue_elements[0..end_offset].to_vec();

                let merkle_proofs_with_context = queue_elements
                    .iter()
                    .map(|element| MerkleProofWithContext {
                        proof: Vec::new(),
                        leaf: [0u8; 32],
                        leaf_index: 0,
                        merkle_tree: address_tree_bundle.accounts.merkle_tree.to_bytes(),
                        root: address_tree_bundle.root(),
                        tx_hash: None,
                        root_seq: 0,
                        account_hash: *element,
                    })
                    .collect();
                return Ok(Response {
                    context: Context {
                        slot: self.get_current_slot(),
                    },
                    value: Items {
                        items: merkle_proofs_with_context,
                    },
                });
            }

            let state_tree_bundle = self
                .state_merkle_trees
                .iter_mut()
                .find(|x| x.accounts.merkle_tree == pubkey);
            if queue_type == QueueType::InputStateV2 {
                if let Some(state_tree_bundle) = state_tree_bundle {
                    let end_offset = std::cmp::min(
                        num_elements as usize,
                        state_tree_bundle.input_leaf_indices.len(),
                    );
                    let queue_elements =
                        state_tree_bundle.input_leaf_indices[0..end_offset].to_vec();
                    let merkle_proofs = queue_elements
                        .iter()
                        .map(|leaf_info| {
                            match state_tree_bundle
                                .merkle_tree
                                .get_proof_of_leaf(leaf_info.leaf_index as usize, true)
                            {
                                Ok(proof) => proof.to_vec(),
                                Err(_) => {
                                    let mut next_index =
                                        state_tree_bundle.merkle_tree.get_next_index() as u64;
                                    while next_index < leaf_info.leaf_index as u64 {
                                        state_tree_bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                                        next_index =
                                            state_tree_bundle.merkle_tree.get_next_index() as u64;
                                    }
                                    state_tree_bundle
                                        .merkle_tree
                                        .get_proof_of_leaf(leaf_info.leaf_index as usize, true)
                                        .unwrap()
                                        .to_vec();
                                    Vec::new()
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                    let leaves = queue_elements
                        .iter()
                        .map(|leaf_info| {
                            state_tree_bundle
                                .merkle_tree
                                .get_leaf(leaf_info.leaf_index as usize)
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>();
                    let merkle_proofs_with_context = merkle_proofs
                        .iter()
                        .zip(queue_elements.iter())
                        .zip(leaves.iter())
                        .map(|((proof, element), leaf)| MerkleProofWithContext {
                            proof: proof.clone(),
                            leaf: *leaf,
                            leaf_index: element.leaf_index as u64,
                            merkle_tree: state_tree_bundle.accounts.merkle_tree.to_bytes(),
                            root: state_tree_bundle.merkle_tree.root(),
                            tx_hash: Some(element.tx_hash),
                            root_seq: 0,
                            account_hash: element.leaf,
                        })
                        .collect();

                    return Ok(Response {
                        context: Context {
                            slot: self.get_current_slot(),
                        },
                        value: Items {
                            items: merkle_proofs_with_context,
                        },
                    });
                }
            }

            if queue_type == QueueType::OutputStateV2 {
                if let Some(state_tree_bundle) = state_tree_bundle {
                    let end_offset = std::cmp::min(
                        num_elements as usize,
                        state_tree_bundle.output_queue_elements.len(),
                    );
                    let queue_elements =
                        state_tree_bundle.output_queue_elements[0..end_offset].to_vec();
                    let indices = queue_elements
                        .iter()
                        .map(|(_, index)| index)
                        .collect::<Vec<_>>();
                    let merkle_proofs = indices
                        .iter()
                        .map(|index| {
                            match state_tree_bundle
                                .merkle_tree
                                .get_proof_of_leaf(**index as usize, true)
                            {
                                Ok(proof) => proof.to_vec(),
                                Err(_) => {
                                    let mut next_index =
                                        state_tree_bundle.merkle_tree.get_next_index() as u64;
                                    while next_index < **index {
                                        state_tree_bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                                        next_index =
                                            state_tree_bundle.merkle_tree.get_next_index() as u64;
                                    }
                                    state_tree_bundle
                                        .merkle_tree
                                        .get_proof_of_leaf(**index as usize, true)
                                        .unwrap()
                                        .to_vec();
                                    Vec::new()
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                    let leaves = indices
                        .iter()
                        .map(|index| {
                            state_tree_bundle
                                .merkle_tree
                                .get_leaf(**index as usize)
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>();
                    let merkle_proofs_with_context = merkle_proofs
                        .iter()
                        .zip(queue_elements.iter())
                        .zip(leaves.iter())
                        .map(|((proof, (element, index)), leaf)| MerkleProofWithContext {
                            proof: proof.clone(),
                            leaf: *leaf,
                            leaf_index: *index,
                            merkle_tree: state_tree_bundle.accounts.merkle_tree.to_bytes(),
                            root: state_tree_bundle.merkle_tree.root(),
                            tx_hash: None,
                            root_seq: 0,
                            account_hash: *element,
                        })
                        .collect();
                    return Ok(Response {
                        context: Context {
                            slot: self.get_current_slot(),
                        },
                        value: Items {
                            items: merkle_proofs_with_context,
                        },
                    });
                }
            }

            Err(IndexerError::InvalidParameters(
                "Merkle tree not found".to_string(),
            ))
        }
    }

    async fn get_subtrees(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<Items<[u8; 32]>>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!("get_subtrees");
        #[cfg(feature = "v2")]
        {
            let merkle_tree_pubkey = Pubkey::new_from_array(_merkle_tree_pubkey);
            let address_tree_bundle = self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
            if let Some(address_tree_bundle) = address_tree_bundle {
                Ok(Response {
                    context: Context {
                        slot: self.get_current_slot(),
                    },
                    value: Items {
                        items: address_tree_bundle.get_subtrees(),
                    },
                })
            } else {
                let state_tree_bundle = self
                    .state_merkle_trees
                    .iter()
                    .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
                if let Some(state_tree_bundle) = state_tree_bundle {
                    Ok(Response {
                        context: Context {
                            slot: self.get_current_slot(),
                        },
                        value: Items {
                            items: state_tree_bundle.merkle_tree.get_subtrees(),
                        },
                    })
                } else {
                    Err(IndexerError::InvalidParameters(
                        "Merkle tree not found".to_string(),
                    ))
                }
            }
        }
    }

    async fn get_address_queue_with_proofs(
        &mut self,
        _merkle_tree_pubkey: &Pubkey,
        _zkp_batch_size: u16,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<BatchAddressUpdateIndexerResponse>, IndexerError> {
        #[cfg(not(feature = "v2"))]
        unimplemented!("get_address_queue_with_proofs");
        #[cfg(feature = "v2")]
        {
            use light_client::indexer::AddressQueueIndex;
            let merkle_tree_pubkey = _merkle_tree_pubkey;
            let zkp_batch_size = _zkp_batch_size;

            let batch_start_index = self
                .get_address_merkle_trees()
                .iter()
                .find(|x| x.accounts.merkle_tree == *merkle_tree_pubkey)
                .unwrap()
                .get_v2_indexed_merkle_tree()
                .ok_or(IndexerError::Unknown(
                    "Failed to get v2 indexed merkle tree".into(),
                ))?
                .merkle_tree
                .rightmost_index;

            let address_proofs = self
                .get_queue_elements(
                    merkle_tree_pubkey.to_bytes(),
                    QueueType::AddressV2,
                    zkp_batch_size,
                    None,
                    None,
                )
                .await
                .map_err(|_| IndexerError::Unknown("Failed to get queue elements".into()))?
                .value;

            let addresses: Vec<AddressQueueIndex> = address_proofs
                .items
                .iter()
                .enumerate()
                .map(|(i, proof)| AddressQueueIndex {
                    address: proof.account_hash,
                    queue_index: proof.root_seq + i as u64,
                })
                .collect();
            let non_inclusion_proofs = self
                .get_multiple_new_address_proofs(
                    merkle_tree_pubkey.to_bytes(),
                    address_proofs
                        .items
                        .iter()
                        .map(|x| x.account_hash)
                        .collect(),
                    None,
                )
                .await
                .map_err(|_| {
                    IndexerError::Unknown(
                        "Failed to get get_multiple_new_address_proofs_full".into(),
                    )
                })?
                .value;

            let subtrees = self
                .get_subtrees(merkle_tree_pubkey.to_bytes(), None)
                .await
                .map_err(|_| IndexerError::Unknown("Failed to get subtrees".into()))?
                .value;

            Ok(Response {
                context: Context {
                    slot: self.get_current_slot(),
                },
                value: BatchAddressUpdateIndexerResponse {
                    batch_start_index: batch_start_index as u64,
                    addresses,
                    non_inclusion_proofs: non_inclusion_proofs.items,
                    subtrees: subtrees.items,
                },
            })
        }
    }

    // New required trait methods
    async fn get_compressed_balance_by_owner(
        &self,
        _owner: &Pubkey,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<u64>, IndexerError> {
        todo!("get_compressed_balance_by_owner not implemented")
    }

    async fn get_compressed_mint_token_holders(
        &self,
        _mint: &Pubkey,
        _options: Option<PaginatedOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<OwnerBalance>>, IndexerError> {
        todo!("get_compressed_mint_token_holders not implemented")
    }

    async fn get_compressed_token_accounts_by_delegate(
        &self,
        _delegate: &Pubkey,
        _options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<TokenAccount>>, IndexerError> {
        todo!("get_compressed_token_accounts_by_delegate not implemented")
    }

    async fn get_compression_signatures_for_address(
        &self,
        _address: &[u8; 32],
        _options: Option<PaginatedOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        todo!("get_compression_signatures_for_address not implemented")
    }

    async fn get_compression_signatures_for_owner(
        &self,
        _owner: &Pubkey,
        _options: Option<PaginatedOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        todo!("get_compression_signatures_for_owner not implemented")
    }

    async fn get_compression_signatures_for_token_owner(
        &self,
        _owner: &Pubkey,
        _options: Option<PaginatedOptions>,
        _config: Option<IndexerRpcConfig>,
    ) -> Result<Response<ItemsWithCursor<SignatureWithMetadata>>, IndexerError> {
        todo!("get_compression_signatures_for_token_owner not implemented")
    }

    async fn get_indexer_health(&self, _config: Option<RetryConfig>) -> Result<bool, IndexerError> {
        todo!("get_indexer_health not implemented")
    }
}

#[async_trait]
impl TestIndexerExtensions for TestIndexer {
    fn get_address_merkle_tree(
        &self,
        merkle_tree_pubkey: Pubkey,
    ) -> Option<&AddressMerkleTreeBundle> {
        self.address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    fn add_compressed_accounts_with_token_data(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) {
        TestIndexer::add_event_and_compressed_accounts(self, slot, event);
    }

    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str) {
        let decoded_hash: [u8; 32] = bs58::decode(account_hash)
            .into_vec()
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();

        if let Some(state_tree_bundle) = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
        {
            if let Some(leaf_index) = state_tree_bundle.merkle_tree.get_leaf_index(&decoded_hash) {
                state_tree_bundle
                    .merkle_tree
                    .update(&[0u8; 32], leaf_index)
                    .unwrap();
            }
        }
    }

    fn address_tree_updated(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    ) {
        info!("Updating address tree...");
        let pos = self
            .address_merkle_trees
            .iter()
            .position(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();
        let new_low_element = context.new_low_element.clone().unwrap();
        let new_element = context.new_element.clone().unwrap();
        let new_element_next_value = context.new_element_next_value.clone().unwrap();
        // It can only be v1 address tree because proof with context has len 16.
        self.address_merkle_trees[pos]
            .get_v1_indexed_merkle_tree_mut()
            .expect("Failed to get v1 indexed merkle tree.")
            .update(&new_low_element, &new_element, &new_element_next_value)
            .unwrap();
        self.address_merkle_trees[pos]
            .append_with_low_element_index(new_low_element.index, &new_element.value)
            .unwrap();
        info!("Address tree updated");
    }

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
        pubkeys
            .iter()
            .map(|x| {
                self.state_merkle_trees
                    .iter()
                    .find(|y| y.accounts.merkle_tree == *x || y.accounts.nullifier_queue == *x)
                    .unwrap()
                    .accounts
            })
            .collect::<Vec<_>>()
    }

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle> {
        &self.state_merkle_trees
    }

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle> {
        &mut self.state_merkle_trees
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        &self.address_merkle_trees
    }

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle> {
        &mut self.address_merkle_trees
    }

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext> {
        &self.token_compressed_accounts
    }

    fn get_group_pda(&self) -> &Pubkey {
        &self.group_pda
    }

    fn add_address_merkle_tree_accounts(
        &mut self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        info!("Adding address merkle tree accounts...");
        let address_merkle_tree_accounts = AddressMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
        };
        self.address_merkle_trees
            .push(Self::add_address_merkle_tree_bundle(address_merkle_tree_accounts).unwrap());
        info!(
            "Address merkle tree accounts added. Total: {}",
            self.address_merkle_trees.len()
        );
        address_merkle_tree_accounts
    }

    fn get_compressed_accounts_with_merkle_context_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .cloned()
            .collect()
    }

    fn add_state_bundle(&mut self, state_bundle: StateMerkleTreeBundle) {
        Self::get_state_merkle_trees_mut(self).push(state_bundle);
    }

    fn add_event_and_compressed_accounts(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    ) {
        let mut compressed_accounts = Vec::new();
        let mut token_compressed_accounts = Vec::new();
        let event_inputs_len = event.input_compressed_account_hashes.len();
        let event_outputs_len = event.output_compressed_account_hashes.len();
        for i in 0..std::cmp::max(event_inputs_len, event_outputs_len) {
            self.process_v1_compressed_account(
                slot,
                event,
                i,
                &mut token_compressed_accounts,
                &mut compressed_accounts,
            );
        }

        self.events.push(event.clone());
        (compressed_accounts, token_compressed_accounts)
    }

    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> MerkleProof {
        let bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        while bundle.merkle_tree.leaves().len() <= index as usize {
            bundle.merkle_tree.append(&[0u8; 32]).unwrap();
        }

        let leaf = match bundle.merkle_tree.get_leaf(index as usize) {
            Ok(leaf) => leaf,
            Err(_) => {
                bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                bundle.merkle_tree.get_leaf(index as usize).unwrap()
            }
        };

        let proof = bundle
            .merkle_tree
            .get_proof_of_leaf(index as usize, true)
            .unwrap()
            .to_vec();

        MerkleProof {
            hash: leaf,
            leaf_index: index,
            merkle_tree: merkle_tree_pubkey,
            proof,
            root_seq: bundle.merkle_tree.sequence_number as u64,
            root: bundle.merkle_tree.root(),
        }
    }

    async fn finalize_batched_address_tree_update(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        account_data: &mut [u8],
    ) {
        let onchain_account =
            BatchedMerkleTreeAccount::address_from_bytes(account_data, &merkle_tree_pubkey.into())
                .unwrap();
        let address_tree = self
            .address_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();
        let address_tree_index = address_tree.right_most_index();
        let onchain_next_index = onchain_account.next_index;
        let diff_onchain_indexer = onchain_next_index - address_tree_index as u64;
        let addresses = address_tree.queue_elements[0..diff_onchain_indexer as usize].to_vec();
        for _ in 0..diff_onchain_indexer {
            address_tree.queue_elements.remove(0);
        }
        for new_element_value in &addresses {
            address_tree
                .append(&BigUint::from_bytes_be(new_element_value))
                .unwrap();
        }
        match &mut address_tree.merkle_tree {
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.num_root_updates += 1,
            IndexedMerkleTreeVersion::V1(_) => {
                unimplemented!("finalize_batched_address_tree_update not implemented for v1 trees.")
            }
        }
        let onchain_root = onchain_account.root_history.last().unwrap();
        let new_root = address_tree.root();
        assert_eq!(*onchain_root, new_root);
    }
}

impl TestIndexer {
    fn get_current_slot(&self) -> u64 {
        // For testing, we can use a fixed slot or MAX
        u64::MAX
    }

    pub async fn init_from_acounts(
        payer: &Keypair,
        env: &TestAccounts,
        output_queue_batch_size: usize,
    ) -> Self {
        // Create a vector of StateMerkleTreeAccounts from all v1 and v2 state trees
        let mut state_merkle_tree_accounts = env.v1_state_trees.clone();

        // Add v2 state trees converting from StateMerkleTreeAccountsV2 to StateMerkleTreeAccounts
        for v2_state_tree in &env.v2_state_trees {
            state_merkle_tree_accounts.push(StateMerkleTreeAccounts {
                merkle_tree: v2_state_tree.merkle_tree,
                nullifier_queue: v2_state_tree.output_queue,
                cpi_context: v2_state_tree.cpi_context,
            });
        }

        // Create a vector of AddressMerkleTreeAccounts from all v1 address trees
        let mut address_merkle_tree_accounts = env.v1_address_trees.clone();

        // Add v2 address trees (each entry is both the merkle tree and queue)
        for &v2_address_tree in &env.v2_address_trees {
            address_merkle_tree_accounts.push(AddressMerkleTreeAccounts {
                merkle_tree: v2_address_tree,
                queue: v2_address_tree,
            });
        }

        Self::new(
            state_merkle_tree_accounts,
            address_merkle_tree_accounts,
            payer.insecure_clone(),
            env.protocol.group_pda,
            output_queue_batch_size,
        )
        .await
    }

    pub async fn new(
        state_merkle_tree_accounts: Vec<StateMerkleTreeAccounts>,
        address_merkle_tree_accounts: Vec<AddressMerkleTreeAccounts>,
        payer: Keypair,
        group_pda: Pubkey,
        output_queue_batch_size: usize,
    ) -> Self {
        let mut state_merkle_trees = Vec::new();
        for state_merkle_tree_account in state_merkle_tree_accounts.iter() {
            let test_batched_output_queue =
                Keypair::from_bytes(&BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR).unwrap();
            let (tree_type, merkle_tree, output_queue_batch_size) = if state_merkle_tree_account
                .nullifier_queue
                == test_batched_output_queue.pubkey()
            {
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new_with_history(
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                    0,
                    0,
                    DEFAULT_BATCH_STATE_ROOT_HISTORY_LEN as usize,
                ));
                (
                    TreeType::StateV2,
                    merkle_tree,
                    Some(output_queue_batch_size),
                )
            } else {
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new_with_history(
                    account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
                    account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                    0,
                    account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize,
                ));
                (TreeType::StateV1, merkle_tree, None)
            };

            state_merkle_trees.push(StateMerkleTreeBundle {
                accounts: *state_merkle_tree_account,
                merkle_tree,
                rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
                tree_type,
                output_queue_elements: vec![],
                input_leaf_indices: vec![],
                output_queue_batch_size,
                num_inserted_batches: 0,
            });
        }

        let mut address_merkle_trees = Vec::new();
        for address_merkle_tree_account in address_merkle_tree_accounts {
            address_merkle_trees
                .push(Self::add_address_merkle_tree_bundle(address_merkle_tree_account).unwrap());
        }

        Self {
            state_merkle_trees,
            address_merkle_trees,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
            group_pda,
        }
    }

    pub fn add_address_merkle_tree_bundle(
        address_merkle_tree_accounts: AddressMerkleTreeAccounts,
        // TODO: add config here
    ) -> Result<AddressMerkleTreeBundle, IndexerError> {
        if address_merkle_tree_accounts.merkle_tree == address_merkle_tree_accounts.queue {
            AddressMerkleTreeBundle::new_v2(address_merkle_tree_accounts)
        } else {
            AddressMerkleTreeBundle::new_v1(address_merkle_tree_accounts)
        }
    }

    async fn add_address_merkle_tree_v1<R: Rpc>(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> Result<AddressMerkleTreeAccounts, RpcError> {
        create_address_merkle_tree_and_queue_account(
            &self.payer,
            true,
            rpc,
            merkle_tree_keypair,
            queue_keypair,
            owning_program_id,
            None,
            &AddressMerkleTreeConfig::default(),
            &AddressQueueConfig::default(),
            0,
        )
        .await?;

        let accounts = <TestIndexer as TestIndexerExtensions>::add_address_merkle_tree_accounts(
            self,
            merkle_tree_keypair,
            queue_keypair,
            owning_program_id,
        );
        Ok(accounts)
    }

    #[cfg(feature = "devenv")]
    async fn add_address_merkle_tree_v2<R: Rpc>(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> Result<AddressMerkleTreeAccounts, RpcError> {
        info!(
            "Adding address merkle tree accounts v2 {:?}",
            merkle_tree_keypair.pubkey()
        );

        let params = light_batched_merkle_tree::initialize_address_tree::InitAddressTreeAccountsInstructionData::test_default();

        info!(
            "Creating batched address merkle tree {:?}",
            merkle_tree_keypair.pubkey()
        );
        create_batch_address_merkle_tree(rpc, &self.payer, merkle_tree_keypair, params).await?;
        info!(
            "Batched address merkle tree created {:?}",
            merkle_tree_keypair.pubkey()
        );

        let accounts = self.add_address_merkle_tree_accounts(
            merkle_tree_keypair,
            queue_keypair,
            _owning_program_id,
        );
        Ok(accounts)
    }

    pub async fn add_address_merkle_tree<R: Rpc>(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        tree_type: TreeType,
    ) -> Result<AddressMerkleTreeAccounts, RpcError> {
        if tree_type == TreeType::AddressV1 {
            self.add_address_merkle_tree_v1(
                rpc,
                merkle_tree_keypair,
                queue_keypair,
                owning_program_id,
            )
            .await
        } else if tree_type == TreeType::AddressV2 {
            #[cfg(not(feature = "devenv"))]
            panic!("Batched address merkle trees require the 'devenv' feature to be enabled");
            #[cfg(feature = "devenv")]
            self.add_address_merkle_tree_v2(
                rpc,
                merkle_tree_keypair,
                queue_keypair,
                owning_program_id,
            )
            .await
        } else {
            Err(RpcError::CustomError(format!(
                "add_address_merkle_tree: Version not supported, {}. Versions: AddressV1, AddressV2",
                tree_type
            )))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_state_merkle_tree<R: Rpc>(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        cpi_context_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        forester: Option<Pubkey>,
        tree_type: TreeType,
    ) {
        let (rollover_fee, merkle_tree, output_queue_batch_size) = match tree_type {
            TreeType::StateV1 => {
                create_state_merkle_tree_and_queue_account(
                    &self.payer,
                    true,
                    rpc,
                    merkle_tree_keypair,
                    queue_keypair,
                    Some(cpi_context_keypair),
                    owning_program_id,
                    forester,
                    self.state_merkle_trees.len() as u64,
                    &StateMerkleTreeConfig::default(),
                    &NullifierQueueConfig::default(),
                )
                    .await
                    .unwrap();
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new_with_history(
                    account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
                    account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                    0,
                    account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize,

                ));
                (FeeConfig::default().state_merkle_tree_rollover as i64,merkle_tree, None)
            }
            TreeType::StateV2 => {
                #[cfg(feature = "devenv")]
                {
                    let params =  light_batched_merkle_tree::initialize_state_tree::InitStateTreeAccountsInstructionData::test_default();

                    create_batched_state_merkle_tree(
                        &self.payer,
                        true,
                        rpc,
                        merkle_tree_keypair,
                        queue_keypair,
                        cpi_context_keypair,
                        params,
                    ).await.unwrap();
                    let merkle_tree = Box::new(MerkleTree::<Poseidon>::new_with_history(
                        DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                        0,
                        0,
                        DEFAULT_BATCH_STATE_ROOT_HISTORY_LEN as usize,

                    ));
                    (FeeConfig::test_batched().state_merkle_tree_rollover as i64,merkle_tree, Some(params.output_queue_batch_size as usize))
                }

                #[cfg(not(feature = "devenv"))]
                panic!("Batched state merkle trees require the 'devenv' feature to be enabled")
            }
            _ => panic!(
                "add_state_merkle_tree: tree_type not supported, {}. tree_type: 1 concurrent, 2 batched",
                tree_type
            ),
        };
        let state_merkle_tree_account = StateMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            nullifier_queue: queue_keypair.pubkey(),
            cpi_context: cpi_context_keypair.pubkey(),
        };

        self.state_merkle_trees.push(StateMerkleTreeBundle {
            merkle_tree,
            accounts: state_merkle_tree_account,
            rollover_fee,
            tree_type,
            output_queue_elements: vec![],
            input_leaf_indices: vec![],
            num_inserted_batches: 0,
            output_queue_batch_size,
        });
        println!(
            "creating Merkle tree bundle {:?}",
            self.state_merkle_trees
                .iter()
                .map(|x| x.accounts.merkle_tree)
                .collect::<Vec<_>>()
        );
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    pub fn add_lamport_compressed_accounts(&mut self, slot: u64, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        // TODO: map event type
        <TestIndexer as TestIndexerExtensions>::add_event_and_compressed_accounts(
            self, slot, &event,
        );
    }

    /// returns the compressed sol balance of the owner pubkey
    pub fn get_compressed_balance(&self, owner: &Pubkey) -> u64 {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .map(|x| x.compressed_account.lamports)
            .sum()
    }

    /// returns the compressed token balance of the owner pubkey for a token by mint
    pub fn get_compressed_token_balance(&self, owner: &Pubkey, mint: &Pubkey) -> u64 {
        self.token_compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.compressed_account.owner == *owner
                    && x.token_data.mint == *mint
            })
            .map(|x| x.token_data.amount)
            .sum()
    }

    fn process_v1_compressed_account(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
        i: usize,
        token_compressed_accounts: &mut Vec<TokenDataWithMerkleContext>,
        compressed_accounts: &mut Vec<CompressedAccountWithMerkleContext>,
    ) {
        let mut input_addresses = vec![];
        if event.input_compressed_account_hashes.len() > i {
            let tx_hash: [u8; 32] = create_tx_hash(
                &event.input_compressed_account_hashes,
                &event.output_compressed_account_hashes,
                slot,
            )
            .unwrap();
            let hash = event.input_compressed_account_hashes[i];
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x.hash().unwrap() == hash);
            let (leaf_index, merkle_tree_pubkey) = if let Some(index) = index {
                self.nullified_compressed_accounts
                    .push(self.compressed_accounts[index].clone());
                let leaf_index = self.compressed_accounts[index].merkle_context.leaf_index;
                let merkle_tree_pubkey = self.compressed_accounts[index]
                    .merkle_context
                    .merkle_tree_pubkey;
                if let Some(address) = self.compressed_accounts[index].compressed_account.address {
                    input_addresses.push(address);
                }
                self.compressed_accounts.remove(index);
                (leaf_index, merkle_tree_pubkey)
            } else {
                let index = self
                    .token_compressed_accounts
                    .iter()
                    .position(|x| x.compressed_account.hash().unwrap() == hash)
                    .expect("input compressed account not found");
                self.token_nullified_compressed_accounts
                    .push(self.token_compressed_accounts[index].clone());
                let leaf_index = self.token_compressed_accounts[index]
                    .compressed_account
                    .merkle_context
                    .leaf_index;
                let merkle_tree_pubkey = self.token_compressed_accounts[index]
                    .compressed_account
                    .merkle_context
                    .merkle_tree_pubkey;
                self.token_compressed_accounts.remove(index);
                (leaf_index, merkle_tree_pubkey)
            };
            let bundle =
                &mut <TestIndexer as TestIndexerExtensions>::get_state_merkle_trees_mut(self)
                    .iter_mut()
                    .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
                    .unwrap();
            // Store leaf indices of input accounts for batched trees
            if bundle.tree_type == TreeType::StateV2 {
                let leaf_hash = event.input_compressed_account_hashes[i];
                bundle.input_leaf_indices.push(LeafIndexInfo {
                    leaf_index,
                    leaf: leaf_hash,
                    tx_hash,
                });
            }
        }
        let mut new_addresses = vec![];
        if event.output_compressed_accounts.len() > i {
            let compressed_account = &event.output_compressed_accounts[i];
            if let Some(address) = compressed_account.compressed_account.address {
                if !input_addresses.iter().any(|x| x == &address) {
                    new_addresses.push(address);
                }
            }
            let merkle_tree = self.state_merkle_trees.iter().find(|x| {
                x.accounts.merkle_tree
                    == event.pubkey_array
                        [event.output_compressed_accounts[i].merkle_tree_index as usize]
            });
            // Check for output queue
            let merkle_tree = if let Some(merkle_tree) = merkle_tree {
                merkle_tree
            } else {
                self.state_merkle_trees
                    .iter()
                    .find(|x| {
                        x.accounts.nullifier_queue
                            == event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize]
                    })
                    .unwrap()
            };
            let nullifier_queue_pubkey = merkle_tree.accounts.nullifier_queue;
            let merkle_tree_pubkey = merkle_tree.accounts.merkle_tree;
            // if data is some, try to deserialize token data, if it fails, add to compressed_accounts
            // if data is none add to compressed_accounts
            // new accounts are inserted in front so that the newest accounts are found first
            match compressed_account.compressed_account.data.as_ref() {
                Some(data) => {
                    if compressed_account.compressed_account.owner == light_compressed_token::ID
                        && data.discriminator == light_compressed_token::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR
                    {
                        if let Ok(token_data) = TokenData::deserialize(&mut data.data.as_slice()) {
                            let token_account = TokenDataWithMerkleContext {
                                token_data,
                                compressed_account: CompressedAccountWithMerkleContext {
                                    compressed_account: compressed_account
                                        .compressed_account
                                        .clone(),
                                    merkle_context: MerkleContext {
                                        leaf_index: event.output_leaf_indices[i],
                                        merkle_tree_pubkey,
                                        queue_pubkey: nullifier_queue_pubkey,
                                        prove_by_index: false,
                                        tree_type:merkle_tree.tree_type,
                                    },
                                },
                            };
                            token_compressed_accounts.push(token_account.clone());
                            self.token_compressed_accounts.insert(0, token_account);
                        }
                    } else {
                        let compressed_account = CompressedAccountWithMerkleContext {
                            compressed_account: compressed_account.compressed_account.clone(),
                            merkle_context: MerkleContext {
                                leaf_index: event.output_leaf_indices[i],
                                merkle_tree_pubkey,
                                queue_pubkey: nullifier_queue_pubkey,
                                prove_by_index: false,
                                tree_type: merkle_tree.tree_type
                            },
                        };
                        compressed_accounts.push(compressed_account.clone());
                        self.compressed_accounts.insert(0, compressed_account);
                    }
                }
                None => {
                    let compressed_account = CompressedAccountWithMerkleContext {
                        compressed_account: compressed_account.compressed_account.clone(),
                        merkle_context: MerkleContext {
                            leaf_index: event.output_leaf_indices[i],
                            merkle_tree_pubkey,
                            queue_pubkey: nullifier_queue_pubkey,
                            prove_by_index: false,
                            tree_type: merkle_tree.tree_type,
                        },
                    };
                    compressed_accounts.push(compressed_account.clone());
                    self.compressed_accounts.insert(0, compressed_account);
                }
            };
            let merkle_tree = &mut self.state_merkle_trees.iter_mut().find(|x| {
                x.accounts.merkle_tree
                    == event.pubkey_array
                        [event.output_compressed_accounts[i].merkle_tree_index as usize]
            });
            if merkle_tree.is_some() {
                let merkle_tree = merkle_tree.as_mut().unwrap();
                let leaf_hash = compressed_account
                    .compressed_account
                    .hash(
                        &event.pubkey_array
                            [event.output_compressed_accounts[i].merkle_tree_index as usize],
                        &event.output_leaf_indices[i],
                        false,
                    )
                    .unwrap();
                merkle_tree
                    .merkle_tree
                    .append(&leaf_hash)
                    .expect("insert failed");
            } else {
                let merkle_tree = &mut self
                    .state_merkle_trees
                    .iter_mut()
                    .find(|x| {
                        x.accounts.nullifier_queue
                            == event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize]
                    })
                    .unwrap();

                merkle_tree.output_queue_elements.push((
                    event.output_compressed_account_hashes[i],
                    event.output_leaf_indices[i].into(),
                ));
            }
        }
        // checks whether there are addresses in outputs which don't exist in inputs.
        // if so check pubkey_array for the first address Merkle tree and append to the bundles queue elements.
        // Note:
        // - creating addresses in multiple address Merkle trees in one tx is not supported
        // TODO: reimplement this is not a good solution
        // - take addresses and address Merkle tree pubkeys from cpi to account compression program
        if !new_addresses.is_empty() {
            for pubkey in event.pubkey_array.iter() {
                if let Some((_, address_merkle_tree)) = self
                    .address_merkle_trees
                    .iter_mut()
                    .enumerate()
                    .find(|(_, x)| x.accounts.merkle_tree == *pubkey)
                {
                    address_merkle_tree
                        .queue_elements
                        .append(&mut new_addresses);
                }
            }
        }
    }

    async fn _get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        full: bool,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError> {
        let mut proofs: Vec<NewAddressProofWithContext> = Vec::new();

        for address in addresses.iter() {
            info!("Getting new address proof for {:?}", address);
            let pubkey = Pubkey::from(merkle_tree_pubkey);
            let address_tree_bundle = self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == pubkey)
                .unwrap();

            let address_biguint = BigUint::from_bytes_be(address.as_slice());
            let (old_low_address, _old_low_address_next_value) =
                address_tree_bundle.find_low_element_for_nonexistent(&address_biguint)?;
            let address_bundle = address_tree_bundle
                .new_element_with_low_element_index(old_low_address.index, &address_biguint)?;

            let (old_low_address, old_low_address_next_value) =
                address_tree_bundle.find_low_element_for_nonexistent(&address_biguint)?;

            // Get the Merkle proof for updating low element.
            let low_address_proof =
                address_tree_bundle.get_proof_of_leaf(old_low_address.index, full)?;

            let low_address_index: u64 = old_low_address.index as u64;
            let low_address_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address.value).unwrap();
            let low_address_next_index: u64 = old_low_address.next_index as u64;
            let low_address_next_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address_next_value).unwrap();
            let proof = NewAddressProofWithContext {
                merkle_tree: Pubkey::new_from_array(merkle_tree_pubkey),
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
                root: address_tree_bundle.root(),
                root_seq: address_tree_bundle.sequence_number(),
                new_low_element: Some(address_bundle.new_low_element),
                new_element: Some(address_bundle.new_element),
                new_element_next_value: Some(address_bundle.new_element_next_value),
            };
            proofs.push(proof);
        }
        Ok(proofs)
    }
}

impl TestIndexer {
    async fn process_inclusion_proofs(
        &self,
        merkle_tree_pubkeys: &[Pubkey],
        accounts: &[[u8; 32]],
    ) -> Result<
        (
            Option<BatchInclusionJsonStruct>,
            Option<BatchInclusionJsonStructLegacy>,
            Vec<AccountProofInputs>,
        ),
        IndexerError,
    > {
        let mut inclusion_proofs = Vec::new();
        let mut account_proof_inputs = Vec::new();
        let mut height = 0;
        let mut queues = vec![];
        let mut cpi_contextes = vec![];
        let mut tree_types = vec![];
        // Collect all proofs first before any await points
        let proof_data: Vec<_> = accounts
            .iter()
            .zip(merkle_tree_pubkeys.iter())
            .map(|(account, &pubkey)| {
                let bundle = self
                    .state_merkle_trees
                    .iter()
                    .find(|x| {
                        x.accounts.merkle_tree == pubkey || x.accounts.nullifier_queue == pubkey
                    })
                    .unwrap();
                let merkle_tree = &bundle.merkle_tree;
                queues.push(bundle.accounts.nullifier_queue);
                cpi_contextes.push(bundle.accounts.cpi_context);
                tree_types.push(bundle.tree_type);
                let leaf_index = merkle_tree.get_leaf_index(account).unwrap();
                let proof = merkle_tree.get_proof_of_leaf(leaf_index, true).unwrap();

                // Convert proof to owned data that implements Send
                let proof: Vec<BigInt> = proof.iter().map(|x| BigInt::from_be_bytes(x)).collect();

                if height == 0 {
                    height = merkle_tree.height;
                } else {
                    assert_eq!(height, merkle_tree.height);
                }
                let root_index = if bundle.tree_type == TreeType::StateV1 {
                    merkle_tree.get_history_root_index().unwrap()
                } else {
                    merkle_tree.get_history_root_index_v2().unwrap()
                };

                Ok((leaf_index, proof, merkle_tree.root(), root_index))
            })
            .collect::<Result<_, IndexerError>>()?;

        // Now handle the async operations with the collected data
        for (i, (leaf_index, proof, merkle_root, root_index)) in proof_data.into_iter().enumerate()
        {
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(merkle_root.as_slice()),
                leaf: BigInt::from_be_bytes(&accounts[i]),
                path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()),
                path_elements: proof,
            });

            account_proof_inputs.push(AccountProofInputs {
                root_index: Some(root_index),
                root: merkle_root,
                leaf_index: leaf_index as u64,
                hash: accounts[i],
                tree_info: light_client::indexer::TreeInfo {
                    cpi_context: Some(cpi_contextes[i]),
                    next_tree_info: None,
                    queue: queues[i],
                    tree: merkle_tree_pubkeys[i],
                    tree_type: tree_types[i],
                },
            });
        }

        let (batch_inclusion_proof_inputs, legacy) = if height
            == DEFAULT_BATCH_STATE_TREE_HEIGHT as usize
        {
            let inclusion_proof_inputs =
                InclusionProofInputs::new(inclusion_proofs.as_slice()).unwrap();
            (
                Some(BatchInclusionJsonStruct::from_inclusion_proof_inputs(
                    &inclusion_proof_inputs,
                )),
                None,
            )
        } else if height == account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize
        {
            let inclusion_proof_inputs = InclusionProofInputsLegacy(inclusion_proofs.as_slice());
            (
                None,
                Some(BatchInclusionJsonStructLegacy::from_inclusion_proof_inputs(
                    &inclusion_proof_inputs,
                )),
            )
        } else {
            return Err(IndexerError::CustomError(
                "Unsupported tree height".to_string(),
            ));
        };

        Ok((batch_inclusion_proof_inputs, legacy, account_proof_inputs))
    }

    async fn process_non_inclusion_proofs(
        &self,
        address_merkle_tree_pubkeys: &[Pubkey],
        addresses: Vec<[u8; 32]>,
    ) -> Result<
        (
            Option<BatchNonInclusionJsonStruct>,
            Option<BatchNonInclusionJsonStructLegacy>,
            Vec<AddressProofInputs>,
        ),
        IndexerError,
    > {
        let mut non_inclusion_proofs = Vec::new();
        let mut address_root_indices = Vec::new();
        let mut tree_heights = Vec::new();
        for (i, address) in addresses.iter().enumerate() {
            let address_tree = self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == address_merkle_tree_pubkeys[i])
                .unwrap();
            tree_heights.push(address_tree.height());

            let proof_inputs = address_tree.get_non_inclusion_proof_inputs(address)?;
            non_inclusion_proofs.push(proof_inputs);

            let (root_index, root, tree_type) = match &address_tree.merkle_tree {
                super::address_tree::IndexedMerkleTreeVersion::V1(tree) => (
                    tree.merkle_tree.get_history_root_index().unwrap() + 1,
                    tree.merkle_tree.root(),
                    TreeType::AddressV1,
                ),
                super::address_tree::IndexedMerkleTreeVersion::V2(tree) => (
                    tree.merkle_tree.get_history_root_index_v2().unwrap(),
                    tree.merkle_tree.root(),
                    TreeType::AddressV2,
                ),
            };
            address_root_indices.push(AddressProofInputs {
                root_index,
                root,
                address: *address,
                tree_info: light_client::indexer::TreeInfo {
                    cpi_context: None,
                    next_tree_info: None,
                    queue: address_tree.accounts.queue,
                    tree: address_tree.accounts.merkle_tree,
                    tree_type,
                },
            });
        }
        // if tree heights are not the same, panic
        if tree_heights.iter().any(|&x| x != tree_heights[0]) {
            return Err(IndexerError::CustomError(format!(
                "All address merkle trees must have the same height {:?}",
                tree_heights
            )));
        }
        let (batch_non_inclusion_proof_inputs, batch_non_inclusion_proof_inputs_legacy) =
            if tree_heights[0] == 26 {
                let non_inclusion_proof_inputs =
                    NonInclusionProofInputsLegacy::new(non_inclusion_proofs.as_slice());
                (
                    None,
                    Some(
                        BatchNonInclusionJsonStructLegacy::from_non_inclusion_proof_inputs(
                            &non_inclusion_proof_inputs,
                        ),
                    ),
                )
            } else if tree_heights[0] == 40 {
                let non_inclusion_proof_inputs =
                    NonInclusionProofInputs::new(non_inclusion_proofs.as_slice()).unwrap();
                (
                    Some(
                        BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
                            &non_inclusion_proof_inputs,
                        ),
                    ),
                    None,
                )
            } else {
                return Err(IndexerError::CustomError(
                    "Unsupported tree height".to_string(),
                ));
            };
        Ok((
            batch_non_inclusion_proof_inputs,
            batch_non_inclusion_proof_inputs_legacy,
            address_root_indices,
        ))
    }
}

impl TestIndexer {
    async fn _get_validity_proof_v1_implementation(
        &self,
        hashes: Vec<[u8; 32]>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ValidityProofWithContext, IndexerError> {
        let mut state_merkle_tree_pubkeys = Vec::new();

        for hash in hashes.iter() {
            state_merkle_tree_pubkeys.push(
                self.get_compressed_account(None, Some(*hash), None)
                    .await?
                    .value
                    .merkle_context
                    .tree,
            );
        }

        let state_merkle_tree_pubkeys = if state_merkle_tree_pubkeys.is_empty() {
            None
        } else {
            Some(state_merkle_tree_pubkeys)
        };
        let hashes = if hashes.is_empty() {
            None
        } else {
            Some(hashes)
        };
        let new_addresses = if new_addresses_with_trees.is_empty() {
            None
        } else {
            Some(
                new_addresses_with_trees
                    .iter()
                    .map(|x| x.address)
                    .collect::<Vec<[u8; 32]>>(),
            )
        };
        let address_merkle_tree_pubkeys = if new_addresses_with_trees.is_empty() {
            None
        } else {
            Some(
                new_addresses_with_trees
                    .iter()
                    .map(|x| x.tree)
                    .collect::<Vec<Pubkey>>(),
            )
        };

        {
            let compressed_accounts = hashes;
            if compressed_accounts.is_some()
                && ![1usize, 2usize, 3usize, 4usize, 8usize]
                    .contains(&compressed_accounts.as_ref().unwrap().len())
            {
                return Err(IndexerError::CustomError(format!(
                    "compressed_accounts must be of length 1, 2, 3, 4 or 8 != {}",
                    compressed_accounts.unwrap().len()
                )));
            }
            if new_addresses.is_some()
                && ![1usize, 2usize, 3usize, 4usize, 8usize]
                    .contains(&new_addresses.as_ref().unwrap().len())
            {
                return Err(IndexerError::CustomError(format!(
                    "new_addresses must be of length 1, 2, 3, 4 or 8 != {}",
                    new_addresses.unwrap().len()
                )));
            }
            let client = Client::new();
            let (account_proof_inputs, address_proof_inputs, json_payload) =
                match (compressed_accounts, new_addresses) {
                    (Some(accounts), None) => {
                        let (payload, payload_legacy, indices) = self
                            .process_inclusion_proofs(
                                &state_merkle_tree_pubkeys.unwrap(),
                                &accounts,
                            )
                            .await?;
                        if let Some(payload) = payload {
                            (indices, Vec::new(), payload.to_string())
                        } else {
                            (indices, Vec::new(), payload_legacy.unwrap().to_string())
                        }
                    }
                    (None, Some(addresses)) => {
                        let (payload, payload_legacy, indices) = self
                            .process_non_inclusion_proofs(
                                address_merkle_tree_pubkeys.unwrap().as_slice(),
                                addresses,
                            )
                            .await?;
                        let payload_string = if let Some(payload) = payload {
                            payload.to_string()
                        } else {
                            payload_legacy.unwrap().to_string()
                        };
                        (Vec::new(), indices, payload_string)
                    }
                    (Some(accounts), Some(addresses)) => {
                        let (inclusion_payload, inclusion_payload_legacy, inclusion_indices) = self
                            .process_inclusion_proofs(
                                &state_merkle_tree_pubkeys.unwrap(),
                                &accounts,
                            )
                            .await?;

                        let (
                            non_inclusion_payload,
                            non_inclusion_payload_legacy,
                            non_inclusion_indices,
                        ) = self
                            .process_non_inclusion_proofs(
                                address_merkle_tree_pubkeys.unwrap().as_slice(),
                                addresses,
                            )
                            .await?;
                        let json_payload = if let Some(non_inclusion_payload) =
                            non_inclusion_payload
                        {
                            let public_input_hash = BigInt::from_bytes_be(
                                num_bigint::Sign::Plus,
                                &create_hash_chain_from_slice(&[
                                    bigint_to_u8_32(
                                        &string_to_big_int(
                                            &inclusion_payload.as_ref().unwrap().public_input_hash,
                                        )
                                        .unwrap(),
                                    )
                                    .unwrap(),
                                    bigint_to_u8_32(
                                        &string_to_big_int(
                                            &non_inclusion_payload.public_input_hash,
                                        )
                                        .unwrap(),
                                    )
                                    .unwrap(),
                                ])
                                .unwrap(),
                            );

                            CombinedJsonStruct {
                                circuit_type: ProofType::Combined.to_string(),
                                state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
                                address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
                                public_input_hash: big_int_to_string(&public_input_hash),
                                inclusion: inclusion_payload.unwrap().inputs,
                                non_inclusion: non_inclusion_payload.inputs,
                            }
                            .to_string()
                        } else if let Some(non_inclusion_payload) = non_inclusion_payload_legacy {
                            CombinedJsonStructLegacy {
                                circuit_type: ProofType::Combined.to_string(),
                                state_tree_height: 26,
                                address_tree_height: 26,
                                inclusion: inclusion_payload_legacy.unwrap().inputs,
                                non_inclusion: non_inclusion_payload.inputs,
                            }
                            .to_string()
                        } else {
                            panic!("Unsupported tree height")
                        };
                        (inclusion_indices, non_inclusion_indices, json_payload)
                    }
                    _ => {
                        panic!(
                            "At least one of compressed_accounts or new_addresses must be provided"
                        )
                    }
                };

            let mut retries = 3;
            while retries > 0 {
                let response_result = client
                    .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body(json_payload.clone())
                    .send()
                    .await;
                if let Ok(response_result) = response_result {
                    if response_result.status().is_success() {
                        let body = response_result.text().await.unwrap();
                        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
                        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
                        let (proof_a, proof_b, proof_c) =
                            compress_proof(&proof_a, &proof_b, &proof_c);
                        return Ok(ValidityProofWithContext {
                            accounts: account_proof_inputs,
                            addresses: address_proof_inputs,
                            proof: CompressedProof {
                                a: proof_a,
                                b: proof_b,
                                c: proof_c,
                            }
                            .into(),
                        });
                    }
                } else {
                    println!("Error: {:#?}", response_result);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    retries -= 1;
                }
            }
            Err(IndexerError::CustomError(
                "Failed to get proof from server".to_string(),
            ))
        }
    }
}
