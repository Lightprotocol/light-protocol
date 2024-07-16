use std::ops::DerefMut;
use std::sync::Arc;

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use log::info;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use tokio::sync::Mutex;

use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};
use account_compression::{
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig,
    QueueAccount, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_registry::sdk::{
    create_rollover_address_merkle_tree_instruction, create_rollover_state_merkle_tree_instruction,
    CreateRolloverMerkleTreeInstructionInputs,
};
use light_test_utils::address_merkle_tree_config::{
    get_address_bundle_config, get_state_bundle_config,
};
use light_test_utils::indexer::{
    AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts, StateMerkleTreeBundle,
};
use light_test_utils::registry::RentExemption;
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::{
    create_account_instruction, get_concurrent_merkle_tree, get_indexed_merkle_tree,
};

use crate::errors::ForesterError;
use crate::tree_sync::TreeData;
use crate::{ForesterConfig, RpcPool, TreeType};

pub async fn is_tree_ready_for_rollover<R: RpcConnection>(
    rpc: &Arc<Mutex<R>>,
    tree_pubkey: Pubkey,
    tree_type: TreeType,
) -> Result<bool, ForesterError> {
    info!(
        "Checking if tree is ready for rollover: {:?}",
        tree_pubkey.to_string()
    );
    let mut rpc = rpc.lock().await;
    match tree_type {
        TreeType::State => {
            let account = rpc
                .get_anchor_account::<StateMerkleTreeAccount>(&tree_pubkey)
                .await?
                .unwrap();
            info!("Account: {:?}", account);
            let is_already_rolled_over =
                account.metadata.rollover_metadata.rolledover_slot != u64::MAX;
            if is_already_rolled_over {
                return Ok(false);
            }
            let merkle_tree =
                get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                    &mut rpc,
                    tree_pubkey,
                )
                .await;
            let height = 26;
            let threshold = ((1 << height) * account.metadata.rollover_metadata.rollover_threshold
                / 100) as usize;

            Ok(merkle_tree.next_index() >= threshold)
        }
        TreeType::Address => {
            let account = rpc
                .get_anchor_account::<AddressMerkleTreeAccount>(&tree_pubkey)
                .await?
                .unwrap();
            info!("Account: {:?}", account);
            let is_already_rolled_over =
                account.metadata.rollover_metadata.rolledover_slot != u64::MAX;
            if is_already_rolled_over {
                return Ok(false);
            }

            let merkle_tree =
                get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                    &mut rpc,
                    tree_pubkey,
                )
                .await;

            let height = 26;
            let threshold = ((1 << height) * account.metadata.rollover_metadata.rollover_threshold
                / 100) as usize;

            Ok(merkle_tree.next_index() >= threshold)
        }
    }
}

#[allow(dead_code)]
pub async fn rollover_state_merkle_tree<R: RpcConnection, I: Indexer<R>>(
    config: &Arc<ForesterConfig>,
    rpc_pool: &RpcPool<R>,
    indexer: &Arc<Mutex<I>>,
    tree_data: &TreeData,
) -> Result<(), ForesterError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();
    let new_cpi_signature_keypair = Keypair::new();

    let rpc = rpc_pool.get_connection().await;
    let rollover_signature = perform_state_merkle_tree_roll_over_forester(
        &config.payer_keypair,
        rpc.clone(),
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &new_cpi_signature_keypair,
        &tree_data.tree_pubkey,
        &tree_data.queue_pubkey,
    )
    .await?;
    println!("Rollover signature: {:?}", rollover_signature);
    init_cpi_context_account(
        rpc,
        &new_merkle_tree_keypair.pubkey(),
        &new_cpi_signature_keypair,
        &config.payer_keypair,
    )
    .await;

    let state_bundle = StateMerkleTreeBundle {
        // TODO: fetch correct fee when this property is used
        rollover_fee: 0,
        accounts: StateMerkleTreeAccounts {
            merkle_tree: new_merkle_tree_keypair.pubkey(),
            nullifier_queue: new_nullifier_queue_keypair.pubkey(),
            cpi_context: new_cpi_signature_keypair.pubkey(),
        },
        merkle_tree: Box::new(MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        )),
    };
    indexer.lock().await.add_state_bundle(state_bundle);
    Ok(())
}

pub async fn perform_state_merkle_tree_roll_over_forester<R: RpcConnection>(
    payer: &Keypair,
    context: Arc<Mutex<R>>,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    cpi_context: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instructions = create_rollover_state_merkle_tree_instructions(
        context.clone(),
        &payer.pubkey(),
        new_queue_keypair,
        new_address_merkle_tree_keypair,
        old_merkle_tree_pubkey,
        old_queue_pubkey,
        &cpi_context.pubkey(),
    )
    .await;
    let mut context = context.lock().await;
    let blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![&payer, &new_queue_keypair, &new_address_merkle_tree_keypair],
        blockhash,
    );
    context.process_transaction(transaction).await
}

pub async fn init_cpi_context_account<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    merkle_tree_pubkey: &Pubkey,
    cpi_account_keypair: &Keypair,
    payer: &Keypair,
) -> Pubkey {
    let mut rpc = rpc.lock().await;
    let account_size: usize = 20 * 1024 + 8;
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        account_size,
        rpc.get_minimum_balance_for_rent_exemption(account_size)
            .await
            .unwrap(),
        &light_system_program::ID,
        Some(cpi_account_keypair),
    );
    let data = light_system_program::instruction::InitCpiContextAccount {};
    let accounts = light_system_program::accounts::InitializeCpiContextAccount {
        fee_payer: payer.pubkey(),
        cpi_context_account: cpi_account_keypair.pubkey(),
        system_program: system_program::ID,
        associated_merkle_tree: *merkle_tree_pubkey,
    };
    let instruction = Instruction {
        program_id: light_system_program::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: data.data(),
    };
    rpc.create_and_send_transaction(
        &[account_create_ix, instruction],
        &payer.pubkey(),
        &[payer, cpi_account_keypair],
    )
    .await
    .unwrap();
    cpi_account_keypair.pubkey()
}

pub async fn rollover_address_merkle_tree<R: RpcConnection, I: Indexer<R>>(
    config: &Arc<ForesterConfig>,
    rpc_pool: &RpcPool<R>,
    indexer: &Arc<Mutex<I>>,
    tree_data: &TreeData,
) -> Result<(), RpcError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();
    let rpc = rpc_pool.get_connection().await;
    perform_address_merkle_tree_roll_over(
        &config.payer_keypair,
        rpc.clone(),
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &tree_data.tree_pubkey,
        &tree_data.queue_pubkey,
    )
    .await?;

    indexer.lock().await.add_address_merkle_tree_accounts(
        &new_merkle_tree_keypair,
        &new_nullifier_queue_keypair,
        None,
    );
    Ok(())
}

pub async fn perform_address_merkle_tree_roll_over<R: RpcConnection>(
    payer: &Keypair,
    context: Arc<Mutex<R>>,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instructions = create_rollover_address_merkle_tree_instructions(
        context.clone(),
        &payer.pubkey(),
        new_queue_keypair,
        new_address_merkle_tree_keypair,
        old_merkle_tree_pubkey,
        old_queue_pubkey,
    )
    .await;
    let mut context = context.lock().await;
    let blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![&payer, &new_queue_keypair, &new_address_merkle_tree_keypair],
        blockhash,
    );
    context.process_transaction(transaction).await
}

pub async fn create_rollover_address_merkle_tree_instructions<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
) -> Vec<Instruction> {
    let mut rpc = rpc.lock().await;
    let (merkle_tree_config, queue_config) = get_address_bundle_config(
        rpc.deref_mut(),
        AddressMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            queue: *nullifier_queue_pubkey,
        },
    )
    .await;
    let (merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_address_merkle_tree_and_queue(
            rpc.deref_mut(),
            &merkle_tree_config,
            &queue_config,
        )
        .await;
    let create_nullifier_queue_instruction = create_account_instruction(
        authority,
        queue_rent_exemption.size,
        queue_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_nullifier_queue_keypair),
    );
    let create_state_merkle_tree_instruction = create_account_instruction(
        authority,
        merkle_tree_rent_exemption.size,
        merkle_tree_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_address_merkle_tree_keypair),
    );
    let instruction = create_rollover_address_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        },
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

pub async fn create_rollover_state_merkle_tree_instructions<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    cpi_context: &Pubkey,
) -> Vec<Instruction> {
    let mut rpc = rpc.lock().await;
    let (merkle_tree_config, queue_config) = get_state_bundle_config(
        rpc.deref_mut(),
        StateMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            nullifier_queue: *nullifier_queue_pubkey,
            cpi_context: *cpi_context,
        },
    )
    .await;
    let (state_merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_state_merkle_tree_and_queue(
            rpc.deref_mut(),
            &merkle_tree_config,
            &queue_config,
        )
        .await;
    let create_nullifier_queue_instruction = create_account_instruction(
        authority,
        queue_rent_exemption.size,
        queue_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_nullifier_queue_keypair),
    );
    let create_state_merkle_tree_instruction = create_account_instruction(
        authority,
        state_merkle_tree_rent_exemption.size,
        state_merkle_tree_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_state_merkle_tree_keypair),
    );
    let instruction =
        create_rollover_state_merkle_tree_instruction(CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        });
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

pub async fn get_rent_exemption_for_state_merkle_tree_and_queue<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) -> (RentExemption, RentExemption) {
    let queue_size = QueueAccount::size(queue_config.capacity as usize).unwrap();

    let queue_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(queue_size)
        .await
        .unwrap();
    let tree_size = account_compression::state::StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    let merkle_tree_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(tree_size)
        .await
        .unwrap();
    (
        RentExemption {
            lamports: merkle_tree_rent_exempt_lamports,
            size: tree_size,
        },
        RentExemption {
            lamports: queue_rent_exempt_lamports,
            size: queue_size,
        },
    )
}

pub async fn get_rent_exemption_for_address_merkle_tree_and_queue<R: RpcConnection>(
    rpc: &mut R,
    address_merkle_tree_config: &AddressMerkleTreeConfig,
    address_queue_config: &AddressQueueConfig,
) -> (RentExemption, RentExemption) {
    let queue_size = QueueAccount::size(address_queue_config.capacity as usize).unwrap();

    let queue_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(queue_size)
        .await
        .unwrap();
    let tree_size = account_compression::state::AddressMerkleTreeAccount::size(
        address_merkle_tree_config.height as usize,
        address_merkle_tree_config.changelog_size as usize,
        address_merkle_tree_config.roots_size as usize,
        address_merkle_tree_config.canopy_depth as usize,
        address_merkle_tree_config.address_changelog_size as usize,
    );
    let merkle_tree_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(tree_size)
        .await
        .unwrap();
    (
        RentExemption {
            lamports: merkle_tree_rent_exempt_lamports,
            size: tree_size,
        },
        RentExemption {
            lamports: queue_rent_exempt_lamports,
            size: queue_size,
        },
    )
}
