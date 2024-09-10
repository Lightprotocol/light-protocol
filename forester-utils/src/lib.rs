use crate::rpc::{RpcConnection, RpcError};
use account_compression::initialize_address_merkle_tree::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_instruction;
use light_concurrent_merkle_tree::copy::ConcurrentMerkleTreeCopy;
use light_hash_set::HashSet;
use light_hasher::Hasher;
use light_indexed_merkle_tree::copy::IndexedMerkleTreeCopy;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};
use solana_sdk::account::Account;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::marker::PhantomData;
use std::pin::Pin;
use std::{fmt, mem};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;

pub mod address_merkle_tree_config;
pub mod forester_epoch;
pub mod indexer;
pub mod registry;
pub mod rpc;
pub mod transaction_params;

pub fn create_account_instruction(
    payer: &Pubkey,
    size: usize,
    rent: u64,
    id: &Pubkey,
    keypair: Option<&Keypair>,
) -> Instruction {
    let keypair = match keypair {
        Some(keypair) => keypair.insecure_clone(),
        None => Keypair::new(),
    };
    system_instruction::create_account(payer, &keypair.pubkey(), rent, size as u64, id)
}

#[derive(Debug, Clone)]
pub struct AccountZeroCopy<'a, T> {
    pub account: Pin<Box<Account>>,
    deserialized: *const T,
    _phantom_data: PhantomData<&'a T>,
}

impl<'a, T> AccountZeroCopy<'a, T> {
    pub async fn new<R: RpcConnection>(rpc: &mut R, address: Pubkey) -> AccountZeroCopy<'a, T> {
        let account = Box::pin(rpc.get_account(address).await.unwrap().unwrap());
        let deserialized = account.data[8..].as_ptr() as *const T;

        Self {
            account,
            deserialized,
            _phantom_data: PhantomData,
        }
    }

    // Safe method to access `deserialized` ensuring the lifetime is respected
    pub fn deserialized(&self) -> &'a T {
        unsafe { &*self.deserialized }
    }
}

/// Fetches the given account, then copies and serializes it as a `HashSet`.
///
/// # Safety
///
/// This is highly unsafe. Ensuring that:
///
/// * The correct account is used.
/// * The account has enough space to be treated as a HashSet with specified
///   parameters.
/// * The account data is aligned.
///
/// Is the caller's responsibility.
pub async unsafe fn get_hash_set<T, R: RpcConnection>(rpc: &mut R, pubkey: Pubkey) -> HashSet {
    let mut account = rpc.get_account(pubkey).await.unwrap().unwrap();

    HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<T>()..]).unwrap()
}

/// Fetches the fiven account, then copies and serializes it as a
/// `ConcurrentMerkleTree`.
pub async fn get_concurrent_merkle_tree<T, R, H, const HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> ConcurrentMerkleTreeCopy<H, HEIGHT>
where
    R: RpcConnection,
    H: Hasher,
{
    let account = rpc.get_account(pubkey).await.unwrap().unwrap();

    ConcurrentMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).unwrap()
}
// TODO: do discriminator check
/// Fetches the fiven account, then copies and serializes it as an
/// `IndexedMerkleTree`.
pub async fn get_indexed_merkle_tree<T, R, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> IndexedMerkleTreeCopy<H, I, HEIGHT, NET_HEIGHT>
where
    R: RpcConnection,
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    let account = rpc.get_account(pubkey).await.unwrap().unwrap();

    IndexedMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).unwrap()
}

pub async fn airdrop_lamports<R: RpcConnection>(
    rpc: &mut R,
    destination_pubkey: &Pubkey,
    lamports: u64,
) -> Result<(), RpcError> {
    // Create a transfer instruction
    let transfer_instruction =
        system_instruction::transfer(&rpc.get_payer().pubkey(), destination_pubkey, lamports);
    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    // Create and sign a transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&rpc.get_payer().pubkey()),
        &vec![&rpc.get_payer()],
        latest_blockhash,
    );

    // Send the transaction
    rpc.process_transaction(
        transaction,
        CommitmentConfig::confirmed(),
        RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: None,
            encoding: None,
            max_retries: None,
            min_context_slot: None,
        }
    ).await?;
    Ok(())
}
