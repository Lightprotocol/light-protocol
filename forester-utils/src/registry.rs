use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, QueueAccount,
    StateMerkleTreeConfig,
};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{Rpc, RpcError},
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_rollover_state_merkle_tree_instruction, CreateRolloverMerkleTreeInstructionInputs,
    },
    protocol_config::state::ProtocolConfig,
    sdk::create_update_forester_pda_instruction,
    utils::get_forester_pda,
    ForesterConfig, ForesterPda,
};
use light_sdk;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::{
    address_merkle_tree_config::{get_address_bundle_config, get_state_bundle_config},
    instructions::create_account::create_account_instruction,
};

pub async fn update_test_forester<R: Rpc>(
    rpc: &mut R,
    forester_authority: &Keypair,
    derivation_key: &Pubkey,
    new_forester_authority: Option<&Keypair>,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let mut pre_account_state = rpc
        .get_anchor_account::<ForesterPda>(&get_forester_pda(derivation_key).0)
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
        derivation_key,
        new_forester_authority,
        Some(config),
    );

    rpc.create_and_send_transaction(&[ix], &forester_authority.pubkey(), &signers)
        .await?;

    pre_account_state.config = config;
    assert_registered_forester(rpc, derivation_key, pre_account_state).await
}

pub async fn assert_registered_forester<R: Rpc>(
    rpc: &mut R,
    forester: &Pubkey,
    expected_account: ForesterPda,
) -> Result<(), RpcError> {
    let pda = get_forester_pda(forester).0;
    let account_data = rpc.get_anchor_account::<ForesterPda>(&pda).await?.unwrap();
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

pub async fn get_rent_exemption_for_address_merkle_tree_and_queue<R: Rpc>(
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

pub async fn get_rent_exemption_for_state_merkle_tree_and_queue<R: Rpc>(
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

#[allow(clippy::too_many_arguments)]
pub async fn create_rollover_address_merkle_tree_instructions<R: Rpc>(
    rpc: &mut R,
    authority: &Pubkey,
    derivation: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
    is_metadata_forester: bool,
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
            derivation: *derivation,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
            cpi_context_account: None,
            is_metadata_forester,
        },epoch
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_state_merkle_tree_roll_over<R: Rpc>(
    rpc: &mut R,
    authority: &Keypair,
    derivation: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<(), RpcError> {
    let instructions = create_rollover_address_merkle_tree_instructions(
        rpc,
        &authority.pubkey(),
        derivation,
        new_nullifier_queue_keypair,
        new_state_merkle_tree_keypair,
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        epoch,
        is_metadata_forester,
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
pub async fn create_rollover_state_merkle_tree_instructions<R: Rpc>(
    rpc: &mut R,
    authority: &Pubkey,
    derivation: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    new_cpi_context_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
    is_metadata_forester: bool,
) -> Vec<Instruction> {
    let (merkle_tree_config, queue_config) = get_state_bundle_config(
        rpc,
        StateMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            nullifier_queue: *nullifier_queue_pubkey,
            cpi_context: new_cpi_context_keypair.pubkey(),
            tree_type: light_compressed_account::TreeType::StateV1, // not used
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
    let account_size: usize = ProtocolConfig::default().cpi_context_size as usize;
    let create_cpi_context_account_instruction = create_account_instruction(
        authority,
        account_size,
        rpc.get_minimum_balance_for_rent_exemption(account_size)
            .await
            .unwrap(),
        &Pubkey::from(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
        Some(new_cpi_context_keypair),
    );
    let instruction = create_rollover_state_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            derivation: *derivation,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
            cpi_context_account: Some(new_cpi_context_keypair.pubkey()),
            is_metadata_forester,
        },
        epoch,
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        create_cpi_context_account_instruction,
        instruction,
    ]
}
