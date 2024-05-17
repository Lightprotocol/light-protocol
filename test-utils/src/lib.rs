use anchor_lang::{
    solana_program::{pubkey::Pubkey, system_instruction},
    AnchorDeserialize,
};
use light_hash_set::HashSet;
use num_bigint::ToBigUint;
use num_traits::{Bounded, CheckedAdd, CheckedSub, Unsigned};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    account::Account,
    instruction::{Instruction, InstructionError},
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::{fmt, marker::PhantomData, mem, pin::Pin};
pub mod address_tree_rollover;
pub mod assert_compressed_tx;
pub mod assert_token_tx;
pub mod e2e_test_env;
pub mod spl;
pub mod state_tree_rollover;
pub mod system_program;
pub mod test_env;
pub mod test_forester;
pub mod test_indexer;
#[derive(Debug, Clone)]
pub struct AccountZeroCopy<'a, T> {
    pub account: Pin<Box<Account>>,
    deserialized: *const T,
    _phantom_data: PhantomData<&'a T>,
}

impl<'a, T> AccountZeroCopy<'a, T> {
    pub async fn new(context: &mut ProgramTestContext, address: Pubkey) -> AccountZeroCopy<'a, T> {
        let account = Box::pin(
            context
                .banks_client
                .get_account(address)
                .await
                .unwrap()
                .unwrap(),
        );
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

pub async fn get_account<T: AnchorDeserialize>(
    context: &mut ProgramTestContext,
    pubkey: Pubkey,
) -> T {
    let account = context
        .banks_client
        .get_account(pubkey)
        .await
        .unwrap()
        .unwrap();
    T::deserialize(&mut &account.data[8..]).unwrap()
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
pub async unsafe fn get_hash_set<I, T>(
    context: &mut ProgramTestContext,
    pubkey: Pubkey,
) -> HashSet<I>
where
    I: Bounded
        + CheckedAdd
        + CheckedSub
        + Clone
        + Copy
        + fmt::Display
        + From<u8>
        + PartialEq
        + PartialOrd
        + ToBigUint
        + TryFrom<u64>
        + TryFrom<usize>
        + Unsigned,
    f64: From<I>,
    u64: TryFrom<I>,
    usize: TryFrom<I>,
    <usize as TryFrom<I>>::Error: fmt::Debug,
{
    let mut account = context
        .banks_client
        .get_account(pubkey)
        .await
        .unwrap()
        .unwrap();

    HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<T>()..]).unwrap()
}

pub async fn airdrop_lamports(
    banks_client: &mut ProgramTestContext,
    destination_pubkey: &Pubkey,
    lamports: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a transfer instruction
    let transfer_instruction =
        system_instruction::transfer(&banks_client.payer.pubkey(), destination_pubkey, lamports);
    let latest_blockhash = banks_client.get_new_latest_blockhash().await.unwrap();
    // Create and sign a transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&banks_client.payer.pubkey()),
        &vec![&banks_client.payer],
        latest_blockhash,
    );

    // Send the transaction
    banks_client
        .banks_client
        .process_transaction(transaction)
        .await?;

    Ok(())
}

pub async fn create_and_send_transaction(
    context: &mut ProgramTestContext,
    instruction: &[Instruction],
    payer: &Pubkey,
    signers: &[&Keypair],
) -> Result<Signature, BanksClientError> {
    let transaction = Transaction::new_signed_with_payer(
        instruction,
        Some(payer),
        &signers.to_vec(),
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let signature = transaction.signatures[0];
    context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(signature)
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeeConfig {
    pub state_merkle_tree_rollover: u64,
    pub nullifier_queue_rollover: u64,
    pub address_queue_rollover: u64,
    pub tip: u64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            state_merkle_tree_rollover: 149,
            nullifier_queue_rollover: 29,
            address_queue_rollover: 181,
            tip: 5000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionParams {
    pub num_input_compressed_accounts: u8,
    pub num_output_compressed_accounts: u8,
    pub num_new_addresses: u8,
    pub compress: i64,
    pub fee_config: FeeConfig,
}

pub async fn create_and_send_transaction_with_event<T>(
    context: &mut ProgramTestContext,
    instruction: &[Instruction],
    payer: &Pubkey,
    signers: &[&Keypair],
    transaction_params: Option<TransactionParams>,
) -> Result<Option<T>, BanksClientError>
where
    T: AnchorDeserialize,
{
    let pre_balance = context
        .banks_client
        .get_account(*payer)
        .await?
        .unwrap()
        .lamports;

    let transaction = Transaction::new_signed_with_payer(
        instruction,
        Some(payer),
        signers,
        context.get_new_latest_blockhash().await?,
    );

    // Simulate the transaction. Currently, in banks-client/server, only
    // simulations are able to track CPIs. Therefore, simulating is the
    // only way to retrieve the event.
    let simulation_result = context
        .banks_client
        .simulate_transaction(transaction.clone())
        .await?;
    // Handle an error nested in the simulation result.
    if let Some(Err(e)) = simulation_result.result {
        return Err(BanksClientError::TransactionError(e));
    }

    // Retrieve the event.
    let event = simulation_result
        .simulation_details
        .and_then(|details| details.inner_instructions)
        .and_then(|instructions| {
            instructions.iter().flatten().find_map(|inner_instruction| {
                T::try_from_slice(inner_instruction.instruction.data.as_slice()).ok()
            })
        });
    // If transaction was successful, execute it.
    if let Some(Ok(())) = simulation_result.result {
        context
            .banks_client
            .process_transaction(transaction)
            .await?;
    }

    // assert correct rollover fee and tip distribution
    if let Some(transaction_params) = transaction_params {
        let mut signers = signers.to_vec();
        signers.dedup();
        let post_balance = context
            .banks_client
            .get_account(*payer)
            .await?
            .unwrap()
            .lamports;
        let expected_post_balance = pre_balance as i64
            - i64::from(transaction_params.num_new_addresses)
                * transaction_params.fee_config.address_queue_rollover as i64
            - i64::from(transaction_params.num_input_compressed_accounts)
                * transaction_params.fee_config.nullifier_queue_rollover as i64
            - i64::from(transaction_params.num_output_compressed_accounts)
                * transaction_params.fee_config.state_merkle_tree_rollover as i64
            - transaction_params.compress
            - 5000 * signers.len() as i64
            - transaction_params.fee_config.tip as i64;

        if post_balance as i64 != expected_post_balance {
            println!("transaction_params: {:?}", transaction_params);
            println!("pre_balance: {}", pre_balance);
            println!("post_balance: {}", post_balance);
            println!("expected post_balance: {}", expected_post_balance);
            println!(
                "diff post_balance: {}",
                post_balance as i64 - expected_post_balance
            );
            println!("tip: {}", transaction_params.fee_config.tip);
            return Err(BanksClientError::TransactionError(
                solana_sdk::transaction::TransactionError::InstructionError(
                    0,
                    InstructionError::Custom(11111),
                ),
            ));
        }
    }
    Ok(event)
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
    res: solana_program_test::BanksTransactionResultWithMetadata,
    error_code: u32,
) -> Result<(), solana_sdk::transaction::TransactionError> {
    if !(res.result
        == Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(error_code),
        ))
        || res.result
            == Err(solana_sdk::transaction::TransactionError::InstructionError(
                0,
                InstructionError::ProgramFailedToComplete,
            )))
    {
        println!("result {:?}", res.result);
        println!("error_code {:?}", error_code);
        return Err(res.result.unwrap_err());
    }
    Ok(())
}
