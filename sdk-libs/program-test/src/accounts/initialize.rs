use account_compression::{utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{Rpc, RpcError},
};
use light_compressed_account::TreeType;
use light_registry::{
    account_compression_cpi::sdk::get_registered_program_pda,
    sdk::{
        create_initialize_governance_authority_instruction,
        create_initialize_group_authority_instruction, create_update_protocol_config_instruction,
    },
    utils::{get_cpi_authority_pda, get_forester_pda, get_protocol_config_pda_address},
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
        state_tree::create_state_merkle_tree_and_queue_account,
        test_accounts::{ProtocolAccounts, StateMerkleTreeAccountsV2, TestAccounts},
        test_keypairs::*,
    },
    compressible::FundingPoolConfig,
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
        v2_state_tree_config,
        v2_address_tree_config,
        skip_register_programs,
        skip_second_v1_tree,
        v1_state_tree_config,
        v1_nullifier_queue_config,
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

        // V1 address trees are now loaded from JSON files instead of created fresh
        // to avoid registry validation issues with deprecated V1 address trees.
        // See light_program_test.rs for the JSON loading logic.
        // create_address_merkle_tree_and_queue_account(
        //     &keypairs.governance_authority,
        //     true,
        //     context,
        //     &keypairs.address_merkle_tree,
        //     &keypairs.address_merkle_tree_queue,
        //     None,
        //     None,
        //     v1_address_tree_config,
        //     v1_address_queue_config,
        //     0,
        // )
        // .await?;
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

    // Register forester for epoch 0 if enabled
    if config.with_forester {
        use crate::forester::register_forester::register_forester_for_compress_and_close;
        register_forester_for_compress_and_close(context, &keypairs.forester).await?;
    }

    use solana_sdk::pubkey;
    Ok(TestAccounts {
        protocol: ProtocolAccounts {
            governance_authority: keypairs.governance_authority.insecure_clone(),
            governance_authority_pda: protocol_config_pda.0,
            group_pda,
            forester: keypairs.forester.insecure_clone(),
            registered_program_pda: registered_system_program_pda,
            registered_registry_program_pda,
            registered_forester_pda: get_forester_pda(&keypairs.forester.pubkey()).0,
        },
        v1_state_trees: vec![
            StateMerkleTreeAccounts {
                merkle_tree: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                nullifier_queue: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
                cpi_context: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
                tree_type: TreeType::StateV1,
            },
            StateMerkleTreeAccounts {
                merkle_tree: pubkey!("smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho"),
                nullifier_queue: pubkey!("nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X"),
                cpi_context: pubkey!("cpi2cdhkH5roePvcudTgUL8ppEBfTay1desGh8G8QxK"),
                tree_type: TreeType::StateV1,
            },
        ],
        v1_address_trees: vec![AddressMerkleTreeAccounts {
            merkle_tree: keypairs.address_merkle_tree.pubkey(),
            queue: keypairs.address_merkle_tree_queue.pubkey(),
        }],
        v2_state_trees: vec![
            StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
                output_queue: pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
                cpi_context: pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y"),
            },
            StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
                output_queue: pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
                cpi_context: pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B"),
            },
            StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
                output_queue: pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
                cpi_context: pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf"),
            },
            StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
                output_queue: pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
                cpi_context: pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc"),
            },
            StateMerkleTreeAccountsV2 {
                merkle_tree: pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
                output_queue: pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
                cpi_context: pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6"),
            },
        ],
        v2_address_trees: vec![keypairs.batch_address_merkle_tree.pubkey()],
        funding_pool_config: FundingPoolConfig::get_v1(),
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
