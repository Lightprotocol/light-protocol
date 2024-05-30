use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use anchor_lang::solana_program::{pubkey::Pubkey, system_instruction};
use light_hash_set::HashSet;
use solana_sdk::{
    account::Account,
    instruction::{Instruction, InstructionError},
    signature::Keypair,
    signer::Signer,
    transaction,
    transaction::Transaction,
};
use std::{marker::PhantomData, mem, pin::Pin};

pub mod address_tree_rollover;
pub mod assert_address_merkle_tree;
pub mod assert_compressed_tx;
pub mod assert_merkle_tree;
pub mod assert_queue;
pub mod assert_rollover;
pub mod assert_token_tx;
pub mod e2e_test_env;
pub mod rpc;
pub mod spl;
pub mod state_tree_rollover;
pub mod system_program;
pub mod test_env;
pub mod test_forester;
pub mod test_indexer;
pub mod transaction_params;

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
    rpc.process_transaction(transaction).await?;
    Ok(())
}

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

/// Asserts that the given `BanksTransactionResultWithMetadata` is an error with a custom error code
/// or a program error.
/// Unfortunately BanksTransactionResultWithMetadata does not reliably expose the custom error code, so
/// we allow program error as well.
// TODO: add generic that parses the error code from the result
pub fn assert_custom_error_or_program_error(
    result: Result<(), RpcError>,
    error_code: u32,
) -> Result<(), RpcError> {
    let accepted_errors = [
        (0, InstructionError::ProgramFailedToComplete),
        (0, InstructionError::Custom(error_code)),
    ];

    let is_accepted = accepted_errors.iter().any(|(index, error)| {
        matches!(result, Err(RpcError::TransactionError(transaction::TransactionError::InstructionError(i, ref e))) if i == (*index as u8) && e == error)
    });

    if !is_accepted {
        println!("result {:?}", result);
        println!("error_code {:?}", error_code);
        return Err(result.unwrap_err());
    }

    Ok(())
}
