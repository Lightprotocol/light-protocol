use crate::address_merkle_tree_config::{get_address_bundle_config, get_state_bundle_config};
use crate::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use crate::rpc::rpc_connection::RpcConnection;
use crate::{create_account_instruction, rpc::errors::RpcError};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, QueueAccount,
    StateMerkleTreeConfig,
};

use light_registry::account_compression_cpi::sdk::{
    create_rollover_state_merkle_tree_instruction, CreateRolloverMerkleTreeInstructionInputs,
};
use light_registry::sdk::{
    create_register_forester_instruction, create_update_forester_pda_instruction,
};
use light_registry::utils::get_forester_pda_address;
use light_registry::{ForesterAccount, ForesterConfig};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Creates and asserts forester account creation.
pub async fn register_test_forester<R: RpcConnection>(
    rpc: &mut R,
    governance_authority: &Keypair,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let ix = create_register_forester_instruction(
        &governance_authority.pubkey(),
        forester_authority,
        config,
    );
    rpc.create_and_send_transaction(
        &[ix],
        &governance_authority.pubkey(),
        &[governance_authority],
    )
    .await?;
    assert_registered_forester(
        rpc,
        forester_authority,
        ForesterAccount {
            authority: *forester_authority,
            config,
            active_stake_weight: 1,
            ..Default::default()
        },
    )
    .await
}

pub async fn update_test_forester<R: RpcConnection>(
    rpc: &mut R,
    forester_authority: &Keypair,
    new_forester_authority: Option<&Keypair>,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let mut pre_account_state = rpc
        .get_anchor_account::<ForesterAccount>(
            &get_forester_pda_address(&forester_authority.pubkey()).0,
        )
        .await?
        .unwrap();
    let (signers, new_forester_authority) = if let Some(new_authority) = new_forester_authority {
        pre_account_state.authority = new_authority.pubkey();

        (
            vec![forester_authority, &new_authority],
            Some(new_authority.pubkey()),
        )
    } else {
        (vec![forester_authority], None)
    };
    let ix = create_update_forester_pda_instruction(
        &forester_authority.pubkey(),
        new_forester_authority,
        config,
    );

    rpc.create_and_send_transaction(&[ix], &forester_authority.pubkey(), &signers)
        .await?;

    pre_account_state.config = config;
    assert_registered_forester(rpc, &forester_authority.pubkey(), pre_account_state).await
}

pub async fn assert_registered_forester<R: RpcConnection>(
    rpc: &mut R,
    forester: &Pubkey,
    expected_account: ForesterAccount,
) -> Result<(), RpcError> {
    let pda = get_forester_pda_address(forester).0;
    let account_data = rpc
        .get_anchor_account::<ForesterAccount>(&pda)
        .await?
        .unwrap();
    if account_data != expected_account {
        return Err(RpcError::AssertRpcError(format!(
            "Expected account data: {:?}, got: {:?}",
            expected_account, account_data
        )));
    }
    Ok(())
}

pub struct RentExemption {
    pub size: usize,
    pub lamports: u64,
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

pub async fn create_rollover_address_merkle_tree_instructions<R: RpcConnection>(
    rpc: &mut R,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
) -> Vec<Instruction> {
    let (merkle_tree_config, queue_config) = get_address_bundle_config(
        rpc,
        AddressMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            queue: *nullifier_queue_pubkey,
        },
    )
    .await;
    let (merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_address_merkle_tree_and_queue(
            rpc,
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
    let instruction = light_registry::account_compression_cpi::sdk::create_rollover_address_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        },epoch
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

pub async fn perform_state_merkle_tree_roll_over<R: RpcConnection>(
    rpc: &mut R,
    authority: &Keypair,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
) -> Result<(), RpcError> {
    let instructions = create_rollover_address_merkle_tree_instructions(
        rpc,
        &authority.pubkey(),
        new_nullifier_queue_keypair,
        new_state_merkle_tree_keypair,
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        epoch,
    )
    .await;
    rpc.create_and_send_transaction(
        &instructions,
        &authority.pubkey(),
        &[
            authority,
            new_nullifier_queue_keypair,
            new_state_merkle_tree_keypair,
        ],
    )
    .await?;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub async fn create_rollover_state_merkle_tree_instructions<R: RpcConnection>(
    rpc: &mut R,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    cpi_context: &Pubkey,
    epoch: u64,
) -> Vec<Instruction> {
    let (merkle_tree_config, queue_config) = get_state_bundle_config(
        rpc,
        StateMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            nullifier_queue: *nullifier_queue_pubkey,
            cpi_context: *cpi_context,
        },
    )
    .await;
    let (state_merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_state_merkle_tree_and_queue(rpc, &merkle_tree_config, &queue_config)
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
    let instruction = create_rollover_state_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        },
        epoch,
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}
