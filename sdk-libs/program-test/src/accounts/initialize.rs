use account_compression::{utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority};
use forester_utils::{
    forester_epoch::{Epoch, TreeAccounts},
    registry::register_test_forester,
};
use light_client::rpc::{RpcConnection, RpcError};
use light_compressed_account::TreeType;
use light_registry::{
    account_compression_cpi::sdk::get_registered_program_pda,
    sdk::{
        create_finalize_registration_instruction,
        create_initialize_governance_authority_instruction,
        create_initialize_group_authority_instruction, create_register_program_instruction,
        create_update_protocol_config_instruction,
    },
    utils::{get_cpi_authority_pda, get_forester_pda, get_protocol_config_pda_address},
    ForesterConfig,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[cfg(feature = "devenv")]
use crate::accounts::register_program::register_program_with_registry_program;
#[cfg(feature = "devenv")]
use crate::test_batch_forester::{
    create_batch_address_merkle_tree, create_batched_state_merkle_tree,
};
use crate::{
    accounts::{
        address_merkle_tree::create_address_merkle_tree_and_queue_account,
        env_accounts::EnvAccounts, env_keypairs::*,
        state_merkle_tree::create_state_merkle_tree_and_queue_account,
    },
    test_env::ProgramTestConfig,
    test_rpc::TestRpcConnection,
};

#[allow(clippy::too_many_arguments)]
pub async fn initialize_accounts<R: RpcConnection + TestRpcConnection>(
    context: &mut R,
    config: &ProgramTestConfig,
    keypairs: EnvAccountKeypairs,
    // protocol_config: ProtocolConfig,
    // register_forester_and_advance_to_active_phase: bool,
    // _skip_register_programs: bool,
    // skip_second_v1_tree: bool,
    // v1_state_tree_config: StateMerkleTreeConfig,
    // v1_nullifier_queue_config: NullifierQueueConfig,
    // v1_address_tree_config: AddressMerkleTreeConfig,
    // v1_address_queue_config: AddressQueueConfig,
    // batched_tree_init_params: InitStateTreeAccountsInstructionData,
    // _batched_address_tree_init_params: Option<InitAddressTreeAccountsInstructionData>,
) -> Result<EnvAccounts, RpcError> {
    let ProgramTestConfig {
        protocol_config,
        register_forester_and_advance_to_active_phase,
        batched_tree_init_params,
        batched_address_tree_init_params,
        skip_register_programs,
        skip_second_v1_tree,
        v1_state_tree_config,
        v1_nullifier_queue_config,
        v1_address_tree_config,
        v1_address_queue_config,
        ..
    } = config;
    let _batched_address_tree_init_params = batched_address_tree_init_params;
    let _batched_tree_init_params = batched_tree_init_params;
    let _skip_register_programs = skip_register_programs;
    let cpi_authority_pda = get_cpi_authority_pda();
    let protocol_config_pda = get_protocol_config_pda_address();
    let instruction = create_initialize_governance_authority_instruction(
        keypairs.governance_authority.pubkey(),
        keypairs.governance_authority.pubkey(),
        *protocol_config,
    );
    let update_instruction = create_update_protocol_config_instruction(
        keypairs.governance_authority.pubkey(),
        Some(keypairs.governance_authority.pubkey()),
        None,
    );
    context
        .create_and_send_transaction(
            &[instruction, update_instruction],
            &keypairs.governance_authority.pubkey(),
            &[&keypairs.governance_authority],
        )
        .await?;

    let group_pda = initialize_new_group(
        &keypairs.group_pda_seed,
        &keypairs.governance_authority,
        context,
        cpi_authority_pda.0,
    )
    .await?;

    let gov_authority = context
        .get_anchor_account::<GroupAuthority>(&protocol_config_pda.0)
        .await?
        .ok_or(RpcError::AccountDoesNotExist(
            protocol_config_pda.0.to_string(),
        ))?;
    assert_eq!(
        gov_authority.authority,
        keypairs.governance_authority.pubkey()
    );
    if gov_authority.authority != keypairs.governance_authority.pubkey() {
        return Err(RpcError::CustomError(
            "Invalid governance authority.".to_string(),
        ));
    }

    register_test_forester(
        context,
        &keypairs.governance_authority,
        &keypairs.forester.pubkey(),
        ForesterConfig::default(),
    )
    .await?;
    println!("Registered register_test_forester ");

    #[cfg(feature = "devenv")]
    if !_skip_register_programs {
        register_program_with_registry_program(
            context,
            &keypairs.governance_authority,
            &group_pda,
            &keypairs.system_program,
        )
        .await?;
        register_program_with_registry_program(
            context,
            &keypairs.governance_authority,
            &group_pda,
            &keypairs.registry_program,
        )
        .await?;
    }
    println!("Registered system program");
    let merkle_tree_pubkey = keypairs.state_merkle_tree.pubkey();
    let nullifier_queue_pubkey = keypairs.nullifier_queue.pubkey();
    create_state_merkle_tree_and_queue_account(
        &keypairs.governance_authority,
        true,
        context,
        &keypairs.state_merkle_tree,
        &keypairs.nullifier_queue,
        Some(&keypairs.cpi_context_account),
        None,
        None,
        1,
        v1_state_tree_config,
        v1_nullifier_queue_config,
    )
    .await?;

    if !skip_second_v1_tree {
        create_state_merkle_tree_and_queue_account(
            &keypairs.governance_authority,
            true,
            context,
            &keypairs.state_merkle_tree_2,
            &keypairs.nullifier_queue_2,
            Some(&keypairs.cpi_context_2),
            None,
            None,
            2,
            v1_state_tree_config,
            v1_nullifier_queue_config,
        )
        .await?;
    }
    #[cfg(feature = "devenv")]
    if let Some(batched_tree_init_params) = _batched_tree_init_params {
        create_batched_state_merkle_tree(
            &keypairs.governance_authority,
            true,
            context,
            &keypairs.batched_state_merkle_tree,
            &keypairs.batched_output_queue,
            &keypairs.batched_cpi_context,
            *batched_tree_init_params,
        )
        .await?;
    }
    #[cfg(feature = "devenv")]
    if let Some(params) = _batched_address_tree_init_params {
        create_batch_address_merkle_tree(
            context,
            &keypairs.governance_authority,
            &keypairs.batch_address_merkle_tree,
            *params,
        )
        .await?;
    }
    create_address_merkle_tree_and_queue_account(
        &keypairs.governance_authority,
        true,
        context,
        &keypairs.address_merkle_tree,
        &keypairs.address_merkle_tree_queue,
        None,
        None,
        v1_address_tree_config,
        v1_address_queue_config,
        0,
    )
    .await?;

    let registered_system_program_pda = get_registered_program_pda(&light_system_program::ID);
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    let forester_epoch = if *register_forester_and_advance_to_active_phase {
        let mut registered_epoch = Epoch::register(
            context,
            protocol_config,
            &keypairs.forester,
            &keypairs.forester.pubkey(),
        )
        .await?
        .unwrap();
        context
            .warp_to_slot(registered_epoch.phases.active.start)
            .await?;
        let tree_accounts = vec![
            TreeAccounts {
                tree_type: TreeType::StateV1,
                merkle_tree: merkle_tree_pubkey,
                queue: nullifier_queue_pubkey,
                is_rolledover: false,
            },
            TreeAccounts {
                tree_type: TreeType::AddressV1,
                merkle_tree: keypairs.address_merkle_tree.pubkey(),
                queue: keypairs.address_merkle_tree_queue.pubkey(),
                is_rolledover: false,
            },
        ];

        registered_epoch
            .fetch_account_and_add_trees_with_schedule(context, &tree_accounts)
            .await?;
        let ix = create_finalize_registration_instruction(
            &keypairs.forester.pubkey(),
            &keypairs.forester.pubkey(),
            0,
        );
        context
            .create_and_send_transaction(&[ix], &keypairs.forester.pubkey(), &[&keypairs.forester])
            .await?;
        Some(registered_epoch)
    } else {
        None
    };
    Ok(EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: keypairs.governance_authority.insecure_clone(),
        governance_authority_pda: protocol_config_pda.0,
        forester: keypairs.forester.insecure_clone(),
        registered_program_pda: registered_system_program_pda,
        address_merkle_tree_pubkey: keypairs.address_merkle_tree.pubkey(),
        address_merkle_tree_queue_pubkey: keypairs.address_merkle_tree_queue.pubkey(),
        cpi_context_account_pubkey: keypairs.cpi_context_account.pubkey(),
        registered_registry_program_pda,
        registered_forester_pda: get_forester_pda(&keypairs.forester.pubkey()).0,
        forester_epoch,
        batched_cpi_context: keypairs.batched_cpi_context.pubkey(),
        batched_output_queue: keypairs.batched_output_queue.pubkey(),
        batched_state_merkle_tree: keypairs.batched_state_merkle_tree.pubkey(),
        batch_address_merkle_tree: keypairs.batch_address_merkle_tree.pubkey(),
    })
}

#[cfg(feature = "devenv")]
pub async fn setup_accounts(
    keypairs: EnvAccountKeypairs,
    url: light_client::rpc::solana_rpc::SolanaRpcUrl,
) -> Result<EnvAccounts, RpcError> {
    let mut rpc = light_client::rpc::SolanaRpcConnection::new(url, None, true);

    initialize_accounts(
        &mut rpc,
        // ProtocolConfig::default(),
        // false,
        // false,
        // false,
        // StateMerkleTreeConfig::default(),
        // NullifierQueueConfig::default(),
        // AddressMerkleTreeConfig::default(),
        // AddressQueueConfig::default(),
        // params,
        // Some(InitAddressTreeAccountsInstructionData::test_default()),
        &ProgramTestConfig::default_with_batched_trees(),
        keypairs,
    )
    .await
}

pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub async fn initialize_new_group<R: RpcConnection>(
    group_seed_keypair: &Keypair,
    payer: &Keypair,
    context: &mut R,
    authority: Pubkey,
) -> Result<Pubkey, RpcError> {
    let group_pda = Pubkey::find_program_address(
        &[
            GROUP_AUTHORITY_SEED,
            group_seed_keypair.pubkey().to_bytes().as_slice(),
        ],
        &account_compression::ID,
    )
    .0;

    let instruction = create_initialize_group_authority_instruction(
        payer.pubkey(),
        group_pda,
        group_seed_keypair.pubkey(),
        authority,
    );

    context
        .create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[payer, group_seed_keypair],
        )
        .await?;
    let group_authority = context
        .get_anchor_account::<GroupAuthority>(&group_pda)
        .await?
        .ok_or(RpcError::CustomError(
            "Group authority account does not exist.".to_string(),
        ))?;
    if group_authority.authority != authority {
        return Err(RpcError::CustomError(
            "Group authority account does not match the provided authority.".to_string(),
        ));
    }
    if group_authority.seed != group_seed_keypair.pubkey() {
        return Err(RpcError::CustomError(
            "Group authority account does not match the provided seed.".to_string(),
        ));
    }
    Ok(group_pda)
}

// TODO: unify with keypairs
pub fn get_test_env_accounts() -> EnvAccounts {
    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = get_group_pda(group_seed_keypair.pubkey());

    let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
    let protocol_config_pda = get_protocol_config_pda_address();
    let (_, registered_program_pda) = create_register_program_instruction(
        payer.pubkey(),
        protocol_config_pda,
        group_pda,
        light_system_program::ID,
    );

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    let cpi_context_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();
    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: payer,
        governance_authority_pda: protocol_config_pda.0,
        registered_forester_pda: get_forester_pda(&forester.pubkey()).0,
        forester,
        registered_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
        cpi_context_account_pubkey: cpi_context_keypair.pubkey(),
        registered_registry_program_pda,
        forester_epoch: None,
        batched_cpi_context: Keypair::from_bytes(&BATCHED_CPI_CONTEXT_TEST_KEYPAIR)
            .unwrap()
            .pubkey(),
        batched_output_queue: Keypair::from_bytes(&BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR)
            .unwrap()
            .pubkey(),
        batched_state_merkle_tree: Keypair::from_bytes(&BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR)
            .unwrap()
            .pubkey(),
        batch_address_merkle_tree: Keypair::from_bytes(&BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR)
            .unwrap()
            .pubkey(),
    }
}
