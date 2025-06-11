use account_compression::{utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority};
use forester_utils::{
    forester_epoch::{Epoch, TreeAccounts},
    registry::register_test_forester,
};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{Rpc, RpcError},
};
use light_compressed_account::TreeType;
use light_registry::{
    account_compression_cpi::sdk::get_registered_program_pda,
    sdk::{
        create_finalize_registration_instruction,
        create_initialize_governance_authority_instruction,
        create_initialize_group_authority_instruction, create_update_protocol_config_instruction,
    },
    utils::{get_cpi_authority_pda, get_forester_pda, get_protocol_config_pda_address},
    ForesterConfig,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[cfg(feature = "v2")]
use super::{
    address_tree_v2::create_batch_address_merkle_tree,
    state_tree_v2::create_batched_state_merkle_tree,
};
#[cfg(feature = "devenv")]
use crate::accounts::register_program::register_program_with_registry_program;
use crate::{
    accounts::{
        address_tree::create_address_merkle_tree_and_queue_account,
        state_tree::create_state_merkle_tree_and_queue_account,
        test_accounts::{ProtocolAccounts, StateMerkleTreeAccountsV2, TestAccounts},
        test_keypairs::*,
    },
    program_test::TestRpc,
    ProgramTestConfig,
};

#[allow(clippy::too_many_arguments)]
pub async fn initialize_accounts<R: Rpc + TestRpc>(
    context: &mut R,
    config: &ProgramTestConfig,
    keypairs: &TestKeypairs,
) -> Result<TestAccounts, RpcError> {
    let ProgramTestConfig {
        protocol_config,
        register_forester_and_advance_to_active_phase,
        v2_state_tree_config,
        v2_address_tree_config,
        skip_register_programs,
        skip_second_v1_tree,
        v1_state_tree_config,
        v1_nullifier_queue_config,
        v1_address_tree_config,
        v1_address_queue_config,
        ..
    } = config;
    let _v2_address_tree_config = v2_address_tree_config;
    let _v2_state_tree_config = v2_state_tree_config;
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
    let merkle_tree_pubkey = keypairs.state_merkle_tree.pubkey();
    let nullifier_queue_pubkey = keypairs.nullifier_queue.pubkey();
    if !config.skip_v1_trees {
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
    }
    #[cfg(feature = "v2")]
    if let Some(v2_state_tree_config) = _v2_state_tree_config {
        create_batched_state_merkle_tree(
            &keypairs.governance_authority,
            true,
            context,
            &keypairs.batched_state_merkle_tree,
            &keypairs.batched_output_queue,
            &keypairs.batched_cpi_context,
            *v2_state_tree_config,
        )
        .await?;
    }
    #[cfg(feature = "v2")]
    if let Some(params) = _v2_address_tree_config {
        create_batch_address_merkle_tree(
            context,
            &keypairs.governance_authority,
            &keypairs.batch_address_merkle_tree,
            *params,
        )
        .await?;
    }

    let registered_system_program_pda =
        get_registered_program_pda(&Pubkey::from(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID));
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
        context.warp_to_slot(registered_epoch.phases.active.start)?;
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
    Ok(TestAccounts {
        protocol: ProtocolAccounts {
            governance_authority: keypairs.governance_authority.insecure_clone(),
            governance_authority_pda: protocol_config_pda.0,
            group_pda,
            forester: keypairs.forester.insecure_clone(),
            registered_program_pda: registered_system_program_pda,
            registered_registry_program_pda,
            registered_forester_pda: get_forester_pda(&keypairs.forester.pubkey()).0,
            forester_epoch,
        },
        v1_state_trees: vec![StateMerkleTreeAccounts {
            merkle_tree: merkle_tree_pubkey,
            nullifier_queue: nullifier_queue_pubkey,
            cpi_context: keypairs.cpi_context_account.pubkey(),
        }],
        v1_address_trees: vec![AddressMerkleTreeAccounts {
            merkle_tree: keypairs.address_merkle_tree.pubkey(),
            queue: keypairs.address_merkle_tree_queue.pubkey(),
        }],
        v2_state_trees: vec![StateMerkleTreeAccountsV2 {
            merkle_tree: keypairs.batched_state_merkle_tree.pubkey(),
            output_queue: keypairs.batched_output_queue.pubkey(),
            cpi_context: keypairs.batched_cpi_context.pubkey(),
        }],
        v2_address_trees: vec![keypairs.batch_address_merkle_tree.pubkey()],
    })
}

pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub async fn initialize_new_group<R: Rpc>(
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
