#![cfg(feature = "test-sbf")]

use anchor_lang::{InstructionData, ToAccountMetas};
use light_registry::account_compression_cpi::sdk::{
    create_nullify_instruction, create_update_address_merkle_tree_instruction,
    CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
};
use light_registry::delegate::get_escrow_token_authority;
use light_registry::delegate::state::DelegateAccount;
use light_registry::epoch::claim_forester::CompressedForesterEpochAccount;
use light_registry::errors::RegistryError;
use light_registry::protocol_config::state::{ProtocolConfig, ProtocolConfigPda};
use light_registry::sdk::{
    create_finalize_registration_instruction, create_report_work_instruction,
};
use light_registry::utils::{
    get_forester_epoch_pda_address, get_forester_pda_address, get_forester_token_pool_pda,
    get_protocol_config_pda_address,
};
use light_registry::{ForesterAccount, ForesterConfig, ForesterEpochPda, MINT};
use light_test_utils::assert_epoch::{
    assert_epoch_pda, assert_finalized_epoch_registration, assert_registered_forester_pda,
    assert_report_work, fetch_epoch_and_forester_pdas,
};
use light_test_utils::e2e_test_env::{init_program_test_env, TestForester};
use light_test_utils::forester_epoch::{get_epoch_phases, Epoch, Forester, TreeAccounts, TreeType};
use light_test_utils::indexer::{Indexer, TestIndexer};

use light_test_utils::registry::{
    delegate_test, deposit_test, forester_claim_test, mint_standard_tokens, sync_delegate_test,
    undelegate_test, withdraw_test, DelegateInputs, DepositInputs, SyncDelegateInputs,
    UndelegateInputs, WithdrawInputs,
};
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{
    create_delegate, deposit_to_delegate_account_helper, set_env_with_delegate_and_forester,
    setup_accounts_devnet, setup_test_programs_with_accounts_with_protocol_config, EnvAccounts,
    STANDARD_TOKEN_MINT_KEYPAIR,
};
use light_test_utils::test_forester::{empty_address_queue_test, nullify_compressed_accounts};
use light_test_utils::{get_custom_compressed_account, spl};
use light_test_utils::{
    registry::{
        create_rollover_address_merkle_tree_instructions,
        create_rollover_state_merkle_tree_instructions, register_test_forester,
    },
    rpc::{errors::assert_rpc_error, rpc_connection::RpcConnection, SolanaRpcConnection},
    test_env::{
        get_test_env_accounts, register_program_with_registry_program,
        setup_test_programs_with_accounts,
    },
};
use rand::Rng;
use solana_sdk::program_pack::Pack;
use solana_sdk::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};

#[tokio::test]
async fn test_register_program() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let random_program_keypair = Keypair::new();
    register_program_with_registry_program(
        &mut rpc,
        &env.governance_authority,
        &env.group_pda,
        &random_program_keypair,
    )
    .await
    .unwrap();
}

/// Functional tests:
/// 1. create delegate account and deposit
/// 2. deposit into existing account
/// 3. withdrawal
#[tokio::test]
async fn test_deposit() {
    let token_mint_keypair = Keypair::from_bytes(STANDARD_TOKEN_MINT_KEYPAIR.as_slice()).unwrap();

    let protocol_config = ProtocolConfig {
        mint: token_mint_keypair.pubkey(),
        ..Default::default()
    };
    let (mut rpc, env) =
        setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, false).await;
    let delegate_keypair = Keypair::new();
    rpc.airdrop_lamports(&delegate_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let mut e2e_env = init_program_test_env(rpc, &env).await;

    mint_standard_tokens::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>(
        &mut e2e_env.rpc,
        &mut e2e_env.indexer,
        &env.governance_authority,
        &delegate_keypair.pubkey(),
        1_000_000_000,
        &env.merkle_tree_pubkey,
    )
    .await
    .unwrap();
    // 1. Functional create account and deposit
    {
        let token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&delegate_keypair.pubkey());
        let deposit_amount = 1_000_000;
        let escrow_pda_authority = get_escrow_token_authority(&delegate_keypair.pubkey(), 0).0;
        // approve amount is expected to equal deposit amount
        spl::approve_test(
            &delegate_keypair,
            &mut e2e_env.rpc,
            &mut e2e_env.indexer,
            token_accounts,
            deposit_amount,
            None,
            &escrow_pda_authority,
            &env.merkle_tree_pubkey,
            &env.merkle_tree_pubkey,
            None,
        )
        .await;
        let token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&delegate_keypair.pubkey())
            .iter()
            .filter(|a| a.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<_>>();

        let deposit_inputs = DepositInputs {
            sender: &delegate_keypair,
            amount: deposit_amount,
            delegate_account: None,
            input_token_data: token_accounts,
            input_escrow_token_account: None,
            epoch: 0,
        };
        deposit_test(&mut e2e_env.rpc, &mut e2e_env.indexer, deposit_inputs)
            .await
            .unwrap();
    }
    // 2. Functional deposit into existing account
    {
        println!("\n\n fetching accounts for 2nd deposit \n\n");
        let escrow_pda_authority = get_escrow_token_authority(&delegate_keypair.pubkey(), 0).0;

        let escrow_token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&escrow_pda_authority);

        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );

        let deposit_amount = 1_000_000;
        let token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&delegate_keypair.pubkey());
        // approve amount is expected to equal deposit amount
        spl::approve_test(
            &delegate_keypair,
            &mut e2e_env.rpc,
            &mut e2e_env.indexer,
            token_accounts,
            deposit_amount,
            None,
            &escrow_pda_authority,
            &env.merkle_tree_pubkey,
            &env.merkle_tree_pubkey,
            None,
        )
        .await;
        let token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&delegate_keypair.pubkey())
            .iter()
            .filter(|a| a.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let deposit_inputs = DepositInputs {
            sender: &delegate_keypair,
            amount: deposit_amount,
            delegate_account: delegate_account[0].clone(),
            input_token_data: token_accounts,
            input_escrow_token_account: Some(escrow_token_accounts[0].clone()),
            epoch: 0,
        };
        deposit_test(&mut e2e_env.rpc, &mut e2e_env.indexer, deposit_inputs)
            .await
            .unwrap();
    }
    // 3. Functional withdrawal
    {
        println!("\n\n fetching accounts for withdrawal \n\n");
        let escrow_pda_authority = get_escrow_token_authority(&delegate_keypair.pubkey(), 0).0;

        let escrow_token_accounts = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&escrow_pda_authority);

        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );
        let inputs = WithdrawInputs {
            sender: &delegate_keypair,
            amount: 99999,
            delegate_account: delegate_account[0].as_ref().unwrap().clone(),
            input_escrow_token_account: escrow_token_accounts[0].clone(),
        };
        withdraw_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_delegate() {
    let token_mint_keypair = Keypair::from_bytes(STANDARD_TOKEN_MINT_KEYPAIR.as_slice()).unwrap();

    let protocol_config = ProtocolConfig {
        mint: token_mint_keypair.pubkey(),
        ..Default::default()
    };
    let (mut rpc, env) =
        setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, true).await;
    let delegate_keypair = Keypair::new();
    rpc.airdrop_lamports(&delegate_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let mut e2e_env = init_program_test_env(rpc, &env).await;

    mint_standard_tokens::<ProgramTestRpcConnection, TestIndexer<ProgramTestRpcConnection>>(
        &mut e2e_env.rpc,
        &mut e2e_env.indexer,
        &env.governance_authority,
        &delegate_keypair.pubkey(),
        1_000_000_000,
        &env.merkle_tree_pubkey,
    )
    .await
    .unwrap();
    let forester_pda = env.registered_forester_pda;
    let deposit_amount = 1_000_000;
    deposit_to_delegate_account_helper(
        &mut e2e_env,
        &delegate_keypair,
        deposit_amount,
        &env,
        0,
        None,
        None,
    )
    .await;
    // delegate to forester
    {
        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );
        let inputs = DelegateInputs {
            sender: &delegate_keypair,
            amount: deposit_amount,
            delegate_account: delegate_account[0].as_ref().unwrap().clone(),
            forester_pda,
            no_sync: false,
            output_merkle_tree: env.merkle_tree_pubkey,
        };
        delegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
            .await
            .unwrap();
    }
    let current_slot = e2e_env.rpc.get_slot().await.unwrap();
    e2e_env
        .rpc
        .warp_to_slot(current_slot + protocol_config.active_phase_length)
        .unwrap();
    // undelegate from forester
    {
        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );
        let inputs = UndelegateInputs {
            sender: &delegate_keypair,
            amount: deposit_amount - 1,
            delegate_account: delegate_account[0].as_ref().unwrap().clone(),
            forester_pda,
            no_sync: false,
            output_merkle_tree: env.merkle_tree_pubkey,
        };
        undelegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
            .await
            .unwrap();
    }
    // undelegate from forester
    {
        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );
        let inputs = UndelegateInputs {
            sender: &delegate_keypair,
            amount: 1,
            delegate_account: delegate_account[0].as_ref().unwrap().clone(),
            forester_pda,
            no_sync: false,
            output_merkle_tree: env.merkle_tree_pubkey,
        };
        undelegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
            .await
            .unwrap();
    }
}

use anchor_lang::AccountDeserialize;
use rand::SeedableRng;

// TODO: (doesn't work) add multiple foresters with their own delegates (do it here)
// - have a vector of foresters and their delegates -> this way we can also easily skip epochs
// TODO: (done) have foresters skip epochs
// TODO: make sure that delegates can always undelegate and withdraw regardless of forester actions
// TODO: add check for empty accounts sync delegate
// TODO: add readonly account and use it
// TODO: add inflation curve
// TODO: add timelocked stake accounts
// TODO: add a test where the stake percentage of one delegate stays constant while others change so that we can assert the rewards
#[tokio::test]
async fn test_e2e() {
    let (mut e2e_env, delegate_keypair, env, tree_accounts, registered_epoch) =
        set_env_with_delegate_and_forester(None, None, None, 0, None).await;
    let mut previous_hash = [0u8; 32];
    // let current_epoch = registered_epoch.clone();
    let mut phases = registered_epoch.phases.clone();
    let num_epochs = 40;
    let mut completed_epochs = 0;
    let add_delegates = true;
    let pre_mint_account = e2e_env.rpc.get_account(MINT).await.unwrap().unwrap();
    let pre_token_balance = spl_token::state::Mint::unpack(&pre_mint_account.data)
        .unwrap()
        .supply;
    let mut rng = rand::rngs::ThreadRng::default();
    let seed = rng.gen::<u64>();
    let mut rng_from_seed = rand::rngs::StdRng::seed_from_u64(seed);
    // let mut counter = 0;
    let forester_keypair = env.forester.insecure_clone();
    let delegates = vec![(delegate_keypair, 0)];
    let mut epoch = registered_epoch.epoch;

    // Forester keypair, delegates,  active_epoch, registered_epoch
    let mut foresters: Vec<(Keypair, Vec<(Keypair, i32)>, Option<Epoch>, Option<Epoch>)> =
        vec![(forester_keypair, delegates, Some(registered_epoch), None)];
    // adding a second forester with stake it will not be registered until next epoch
    // env fails with account not found when registering the second forester
    // {
    //     println!("adding second forester");
    //     println!("epoch {}", epoch);
    //     let forester = TestForester {
    //         keypair: Keypair::new(),
    //         forester: Forester::default(),
    //         is_registered: None,
    //     };
    //     let forester_config = ForesterConfig {
    //         fee: rng_from_seed.gen_range(0..=100),
    //         fee_recipient: forester.keypair.pubkey(),
    //     };
    //     register_test_forester(
    //         &mut e2e_env.rpc,
    //         &env.governance_authority,
    //         &forester.keypair,
    //         forester_config,
    //     )
    //     .await
    //     .unwrap();

    //     let forester_pda = get_forester_pda_address(&forester.keypair.pubkey()).0;
    //     // TODO: investigate why + 1
    //     let delgate_keypair =
    //         create_delegate(&mut e2e_env, &env, 1_000_000, forester_pda, epoch + 1, None).await;
    //     foresters.push((forester.keypair, vec![(delgate_keypair, 0)], None, None));
    // }

    // let pre_forester_two_balance = e2e_env
    //     .indexer
    //     .get_compressed_token_balance(&foresters[1].0.pubkey(), &MINT);
    let mut num_mint_tos = 0;
    for i in 1..=num_epochs {
        // Prints
        {
            println!(
            "-------------------------------\n\n  epoch: {} \n\n -------------------------------",
            i
        );
            let registered_forester_is_some = foresters.iter().any(|f| f.2.is_some());
            if !registered_forester_is_some {
                println!("no registered forester skipping epoch");
                // continue;
            } else {
                completed_epochs += 1;
            }

            let current_mint_account = e2e_env.rpc.get_account(MINT).await.unwrap().unwrap();
            let current_token_balance =
                spl_token::state::Mint::unpack(&current_mint_account.data.as_slice())
                    .unwrap()
                    .supply;
            println!("current_token_balance: {}", current_token_balance);
            let escrow_authority = get_escrow_token_authority(&foresters[0].1[0].0.pubkey(), 0).0;
            let delegate_balance = e2e_env
                .indexer
                .get_compressed_token_balance(&escrow_authority, &MINT);
            println!("delegate_balance: {}", delegate_balance);
            let forester_pda_pubkey = get_forester_pda_address(&foresters[0].0.pubkey()).0;
            let forester_pda_account = e2e_env.rpc.get_account(forester_pda_pubkey).await.unwrap();
            if let Some(account) = forester_pda_account {
                let forester_pda =
                    ForesterAccount::try_deserialize(&mut account.data.as_slice()).unwrap();
                println!("forester_pda: {:?}", forester_pda);
            }
            let forester_epoch_pda_pubkey =
                get_forester_epoch_pda_address(&foresters[0].0.pubkey(), epoch).0;

            let forester_epoch_pda = e2e_env
                .rpc
                .get_account(forester_epoch_pda_pubkey)
                .await
                .unwrap();
            if let Some(account) = forester_epoch_pda {
                let forester_epoch_pda =
                    ForesterEpochPda::try_deserialize(&mut account.data.as_slice()).unwrap();
                println!("forester_epoch_pda: {:?}", forester_epoch_pda);
            }
            let forester_token_pool = get_forester_token_pool_pda(&foresters[0].0.pubkey());
            let forester_token_pool_account =
                e2e_env.rpc.get_account(forester_token_pool).await.unwrap();
            if let Some(account) = forester_token_pool_account {
                let forester_token_pool_balance =
                    spl_token::state::Account::unpack(&account.data.as_slice())
                        .unwrap()
                        .amount;
                println!(
                    "forester_token_pool_balance: {}",
                    forester_token_pool_balance
                );
            }

            println!("\n\n\n");
        }
        for (forester_keypair, _, epoch, _) in foresters.iter() {
            // find next slot
            let current_slot = e2e_env.rpc.get_slot().await.unwrap();

            if let Some(epoch) = epoch {
                let treeschedule = epoch
                    .merkle_trees
                    .iter()
                    .find(|t| t.tree_pubkey.tree_type == TreeType::State)
                    .unwrap();
                let next_eligible_light_slot = treeschedule
                    .slots
                    .iter()
                    .find(|s| s.is_some() && s.as_ref().unwrap().start_solana_slot > current_slot)
                    .unwrap();
                e2e_env
                    .rpc
                    .warp_to_slot(next_eligible_light_slot.as_ref().unwrap().start_solana_slot)
                    .unwrap();
                // create work 1 item in nullifier queue
                perform_work(&mut e2e_env, &forester_keypair, &env, epoch.epoch).await;
            }
        }
        // advance epoch to report work and next registration phase
        e2e_env
            .rpc
            .warp_to_slot(phases.report_work.start - 1)
            .unwrap();
        let protocol_config = e2e_env
            .rpc
            .get_anchor_account::<ProtocolConfigPda>(&env.governance_authority_pda)
            .await
            .unwrap()
            .unwrap()
            .config;

        // register for next epoch
        for (forester_keypair, _, _, next_epoch) in foresters.iter_mut() {
            let register_or_not = rng_from_seed.gen_bool(0.6);
            if register_or_not {
                let next_registered_epoch =
                    Epoch::register(&mut e2e_env.rpc, &protocol_config, &forester_keypair)
                        .await
                        .unwrap();
                assert!(next_registered_epoch.is_some());
                let next_registered_epoch = next_registered_epoch.unwrap();
                let forester_pda_pubkey = get_forester_pda_address(&forester_keypair.pubkey()).0;
                let forester_pda = e2e_env
                    .rpc
                    .get_anchor_account::<ForesterAccount>(&forester_pda_pubkey)
                    .await
                    .unwrap()
                    .unwrap();
                let expected_stake = forester_pda.active_stake_weight;
                assert_epoch_pda(
                    &mut e2e_env.rpc,
                    next_registered_epoch.epoch,
                    expected_stake,
                )
                .await;
                assert_registered_forester_pda(
                    &mut e2e_env.rpc,
                    &next_registered_epoch.forester_epoch_pda,
                    &forester_keypair.pubkey(),
                    next_registered_epoch.epoch,
                )
                .await;
                *next_epoch = Some(next_registered_epoch);
            } else {
                *next_epoch = None;
            }
        }
        // // // check that we can still forest the last epoch
        // perform_work(&mut e2e_env, &forester_keypair, &env, current_epoch.epoch).await;

        e2e_env.rpc.warp_to_slot(phases.report_work.start).unwrap();
        // report work
        for (forester_keypair, _, current_epoch, _) in foresters.iter_mut() {
            if let Some(current_epoch) = current_epoch {
                let (pre_forester_epoch_pda, pre_epoch_pda) = fetch_epoch_and_forester_pdas(
                    &mut e2e_env.rpc,
                    &current_epoch.forester_epoch_pda,
                    &current_epoch.epoch_pda,
                )
                .await;
                let ix =
                    create_report_work_instruction(&forester_keypair.pubkey(), current_epoch.epoch);
                e2e_env
                    .rpc
                    .create_and_send_transaction(
                        &[ix],
                        &forester_keypair.pubkey(),
                        &[&forester_keypair],
                    )
                    .await
                    .unwrap();
                assert_report_work(
                    &mut e2e_env.rpc,
                    &current_epoch.forester_epoch_pda,
                    &current_epoch.epoch_pda,
                    pre_forester_epoch_pda,
                    pre_epoch_pda,
                )
                .await;
            }
        }
        for (forester_keypair, _, current_epoch, next_registered_epoch) in foresters.iter_mut() {
            if let Some(next_registered_epoch) = next_registered_epoch {
                let ix = create_finalize_registration_instruction(
                    &env.forester.pubkey(),
                    next_registered_epoch.epoch,
                );

                println!("epoch: {}", epoch);
                println!("next_registered_epoch: {:?}", next_registered_epoch.epoch);
                e2e_env
                    .rpc
                    .create_and_send_transaction(&[ix], &env.forester.pubkey(), &[&env.forester])
                    .await
                    .unwrap();
                next_registered_epoch
                    .fetch_account_and_add_trees_with_schedule(
                        &mut e2e_env.rpc,
                        tree_accounts.clone(),
                    )
                    .await
                    .unwrap();
            }
            if let Some(current_epoch) = current_epoch {
                forester_claim_test(
                    &mut e2e_env.rpc,
                    &mut e2e_env.indexer,
                    &forester_keypair,
                    current_epoch.epoch,
                    env.merkle_tree_pubkey,
                )
                .await
                .unwrap();
            }
            // switch to next epoch
            *current_epoch = next_registered_epoch.clone();
            epoch += 1;
        }
        let forester_keypair = foresters[0].0.insecure_clone();
        // delegates only delegate to the first forester
        for (index, (delegate_keypair, counter)) in foresters[0].1.iter_mut().enumerate() {
            let sync = rng_from_seed.gen_bool(0.3);
            let sync_tokens = rng_from_seed.gen_bool(0.1);

            if i > 0 && sync || num_epochs - 1 == i || *counter == 4 {
                println!("\n\n--------------------------------------\n\n");
                println!("syncing balance of delegate {}", index);
                println!("forester index: {}", index);
                println!("\n\n--------------------------------------\n\n");
                let forester_pda_pubkey = get_forester_pda_address(&forester_keypair.pubkey()).0;

                let compressed_epoch_pda =
                    get_custom_compressed_account::<_, _, CompressedForesterEpochAccount>(
                        &mut e2e_env.indexer,
                        &forester_pda_pubkey,
                        &light_registry::ID,
                    );
                println!("compressed_epoch_pda: {:?}", compressed_epoch_pda);

                let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
                    &mut e2e_env.indexer,
                    &delegate_keypair.pubkey(),
                    &light_registry::ID,
                );
                println!("delegate_account: {:?}", delegate_account);

                let deserialized_delegate =
                    delegate_account[0].as_ref().unwrap().deserialized_account;
                if deserialized_delegate.last_sync_epoch >= epoch {
                    println!("delegate {} already synced", index);
                    continue;
                }

                let escrow_authority = get_escrow_token_authority(&delegate_keypair.pubkey(), 0).0;
                let escrow = e2e_env
                    .indexer
                    .get_compressed_token_accounts_by_owner(&escrow_authority);
                println!("escrow: {:?}", escrow);
                let inputs = SyncDelegateInputs {
                    sender: &delegate_keypair,
                    delegate_account: delegate_account[0].as_ref().unwrap().clone(),
                    compressed_forester_epoch_pdas: compressed_epoch_pda,
                    forester: forester_keypair.pubkey(),
                    output_merkle_tree: env.merkle_tree_pubkey,
                    sync_delegate_token_account: sync_tokens,
                    previous_hash,
                    input_escrow_token_account: Some(escrow[0].clone()),
                };
                sync_delegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
                    .await
                    .unwrap();
                let forester_pda: ForesterAccount = e2e_env
                    .rpc
                    .get_anchor_account::<ForesterAccount>(&forester_pda_pubkey)
                    .await
                    .unwrap()
                    .unwrap();
                previous_hash = forester_pda.last_compressed_forester_epoch_pda_hash;
                *counter = 0;
                // undelegate after syncing
                {
                    let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
                        &mut e2e_env.indexer,
                        &delegate_keypair.pubkey(),
                        &light_registry::ID,
                    );
                    let max_amount = delegate_account[0]
                        .as_ref()
                        .unwrap()
                        .deserialized_account
                        .delegated_stake_weight;
                    if max_amount == 0 {
                        println!("epoch {}", i);
                        println!(
                            "delegate {} has no stake -----------------------------------------",
                            index
                        );
                        println!(
                            "delegate account {:?}",
                            delegate_account[0].as_ref().unwrap().deserialized_account
                        );
                        continue;
                    }
                    let amount = rng_from_seed.gen_range(1..=max_amount);
                    println!(
                        "delegate {} start undelegating {} -----------------------------------------",
                        index, amount
                    );
                    println!("delegate_account: {:?}", delegate_account);
                    let inputs = UndelegateInputs {
                        sender: delegate_keypair,
                        amount,
                        delegate_account: delegate_account[0].as_ref().unwrap().clone(),
                        forester_pda: env.registered_forester_pda,
                        no_sync: false,
                        output_merkle_tree: env.merkle_tree_pubkey,
                    };
                    undelegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
                        .await
                        .unwrap();
                    println!(
                        "delegate {} undelegated {} -----------------------------------------",
                        index, amount
                    );
                }
                // // delegate
                {
                    create_delegate(
                        &mut e2e_env,
                        &env,
                        1_000_000,
                        env.registered_forester_pda,
                        epoch,
                        Some(delegate_keypair.insecure_clone()),
                    )
                    .await;
                    num_mint_tos += 1;
                }
            } else {
                *counter += 1;
            }
        }
        let num_add_delegates = if add_delegates {
            rng_from_seed.gen_range(0..3)
        } else {
            0
        };
        for _ in 0..num_add_delegates {
            let deposit_amount = rng_from_seed.gen_range(1_000_000..1_000_000_000);
            let delegate_keypair = create_delegate(
                &mut e2e_env,
                &env,
                deposit_amount,
                env.registered_forester_pda,
                epoch,
                None,
            )
            .await;
            foresters[0].1.push((delegate_keypair, 0));
        }
        println!(
            "added  {} delegates -----------------------------------------",
            num_add_delegates
        );

        let forester_token_pool = get_forester_token_pool_pda(&foresters[0].0.pubkey());
        let forester_token_pool_account =
            e2e_env.rpc.get_account(forester_token_pool).await.unwrap();
        if let Some(account) = forester_token_pool_account {
            let forester_token_pool_balance =
                spl_token::state::Account::unpack(&account.data.as_slice())
                    .unwrap()
                    .amount;
            println!("epoch: {}", i);
            println!(
                "forester_token_pool_balance: {}",
                forester_token_pool_balance
            );
            if forester_token_pool_balance == 990000 || forester_token_pool_balance == 0 {
                println!(
                    "forester_token_pool_balance: {}",
                    forester_token_pool_balance
                );
                // print delegate account for every delegate
                for (delegate_keypair, _) in foresters[0].1.iter() {
                    let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
                        &mut e2e_env.indexer,
                        &delegate_keypair.pubkey(),
                        &light_registry::ID,
                    );
                    println!("delegate_account: {:?}", delegate_account);
                }
                // print all epoch accounts
                let forester_pda_pubkey = get_forester_pda_address(&foresters[0].0.pubkey()).0;

                let compressed_epoch_pda =
                    get_custom_compressed_account::<_, _, CompressedForesterEpochAccount>(
                        &mut e2e_env.indexer,
                        &forester_pda_pubkey,
                        &light_registry::ID,
                    );
                println!("compressed_epoch_pda: {:?}", compressed_epoch_pda);
                // print allforester accounts
                let forester_pda: ForesterAccount = e2e_env
                    .rpc
                    .get_anchor_account::<ForesterAccount>(&forester_pda_pubkey)
                    .await
                    .unwrap()
                    .unwrap();
                println!("forester_pda: {:?}", forester_pda);
            }
        }
        // // current_epoch = next_registered_epoch;
        // for forester in foresters.iter_mut() {
        //     if let Some(next_registered_epoch) = &forester.3 {
        //         phases = next_registered_epoch.phases.clone();
        //     }
        // }
        phases = get_epoch_phases(&protocol_config, epoch);
    }
    let post_mint_account = e2e_env.rpc.get_account(MINT).await.unwrap().unwrap();
    let post_token_balance = spl_token::state::Mint::unpack(&post_mint_account.data)
        .unwrap()
        .supply;
    let expected_amount_minted = completed_epochs * e2e_env.protocol_config.epoch_reward;

    let forester_keypair = foresters[0].0.insecure_clone();
    for (index, (delegate_keypair, _)) in foresters[0].1.iter_mut().enumerate() {
        println!("\n\n--------------------------------------\n\n");
        println!("syncing balance of delegate {}", index);
        println!("\n\n--------------------------------------\n\n");
        let forester_pda_pubkey = get_forester_pda_address(&forester_keypair.pubkey()).0;

        let compressed_epoch_pda =
            get_custom_compressed_account::<_, _, CompressedForesterEpochAccount>(
                &mut e2e_env.indexer,
                &forester_pda_pubkey,
                &light_registry::ID,
            );
        println!("compressed_epoch_pda: {:?}", compressed_epoch_pda);

        let delegate_account = get_custom_compressed_account::<_, _, DelegateAccount>(
            &mut e2e_env.indexer,
            &delegate_keypair.pubkey(),
            &light_registry::ID,
        );
        println!("delegate_account: {:?}", delegate_account);

        let deserialized_delegate = delegate_account[0].as_ref().unwrap().deserialized_account;
        if deserialized_delegate.last_sync_epoch >= epoch {
            println!("delegate {} already synced", index);
            continue;
        }

        let escrow_authority = get_escrow_token_authority(&delegate_keypair.pubkey(), 0).0;
        let escrow = e2e_env
            .indexer
            .get_compressed_token_accounts_by_owner(&escrow_authority);
        println!("escrow: {:?}", escrow);
        let inputs = SyncDelegateInputs {
            sender: &delegate_keypair,
            delegate_account: delegate_account[0].as_ref().unwrap().clone(),
            compressed_forester_epoch_pdas: compressed_epoch_pda,
            forester: forester_keypair.pubkey(),
            output_merkle_tree: env.merkle_tree_pubkey,
            sync_delegate_token_account: true,
            previous_hash,
            input_escrow_token_account: Some(escrow[0].clone()),
        };
        sync_delegate_test(&mut e2e_env.rpc, &mut e2e_env.indexer, inputs)
            .await
            .unwrap();
    }
    let forester_token_pool = get_forester_token_pool_pda(&foresters[0].0.pubkey());
    let forester_token_pool_account = e2e_env.rpc.get_account(forester_token_pool).await.unwrap();
    if let Some(account) = forester_token_pool_account {
        let forester_token_pool_balance =
            spl_token::state::Account::unpack(&account.data.as_slice())
                .unwrap()
                .amount;
        println!(
            "forester_token_pool_balance: {}",
            forester_token_pool_balance
        );
    }
    println!("completed epochs: {}", completed_epochs);

    assert_eq!(
        post_token_balance,
        pre_token_balance * (foresters[0].1.len() as u64 + num_mint_tos) + expected_amount_minted
    );
    // let forester_two_balance = e2e_env
    //     .indexer
    //     .get_compressed_token_balance(&foresters[1].0.pubkey(), &MINT);
    // println!("forester_two_balance: {}", forester_two_balance);
    // println!("pre_forester_two_balance: {}", pre_forester_two_balance);
    // assert!(forester_two_balance > pre_forester_two_balance);
}

pub async fn perform_work(
    e2e_env: &mut light_test_utils::e2e_test_env::E2ETestEnv<
        ProgramTestRpcConnection,
        TestIndexer<ProgramTestRpcConnection>,
    >,
    forester_keypair: &Keypair,
    _env: &EnvAccounts,
    epoch: u64,
) {
    // create work 1 item in address and nullifier queue each

    e2e_env.create_address(None).await;
    e2e_env
        .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
        .await;
    e2e_env
        .transfer_sol_deterministic(&forester_keypair, &Pubkey::new_unique(), None)
        .await
        .unwrap();

    println!("performed transactions -----------------------------------------");
    // perform 1 work
    nullify_compressed_accounts(
        &mut e2e_env.rpc,
        &forester_keypair,
        &mut e2e_env.indexer.state_merkle_trees[0],
        epoch,
    )
    .await;
    // empty_address_queue_test(
    //     &forester_keypair,
    //     &mut e2e_env.rpc,
    //     &mut e2e_env.indexer.address_merkle_trees[0],
    //     false,
    //     epoch,
    // )
    // .await
    // .unwrap();
}

/// Test:
/// 1. SUCCESS: Register a forester
/// 2. SUCCESS: Update forester authority
/// 3. SUCESS: Register forester for epoch
#[tokio::test]
async fn test_register_and_update_forester_pda() {
    // TODO: add setup test programs wrapper that allows for non default protocol config
    let token_mint_keypair = Keypair::from_bytes(STANDARD_TOKEN_MINT_KEYPAIR.as_slice()).unwrap();

    let protocol_config = ProtocolConfig {
        mint: token_mint_keypair.pubkey(),
        ..Default::default()
    };
    let (mut rpc, env) =
        setup_test_programs_with_accounts_with_protocol_config(None, protocol_config, false).await;
    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    println!("rpc.air -----------------------------------------");
    let config = ForesterConfig {
        fee: 1,
        fee_recipient: Pubkey::new_unique(),
    };
    // 1. SUCCESS: Register a forester
    register_test_forester(
        &mut rpc,
        &env.governance_authority,
        &forester_keypair,
        config,
    )
    .await
    .unwrap();
    println!("registered _test_forester -----------------------------------------");

    // // 2. SUCCESS: Update forester authority
    // let new_forester_keypair = Keypair::new();
    // rpc.airdrop_lamports(&new_forester_keypair.pubkey(), 1_000_000_000)
    //     .await
    //     .unwrap();
    // let config = ForesterConfig { fee: 2 };

    // update_test_forester(
    //     &mut rpc,
    //     &forester_keypair,
    //     Some(&new_forester_keypair),
    //     config,
    // )
    // .await
    // .unwrap();
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

    // 3. SUCCESS: register forester for epoch
    let tree_accounts = vec![
        TreeAccounts {
            tree_type: TreeType::State,
            merkle_tree: env.merkle_tree_pubkey,
            queue: env.nullifier_queue_pubkey,
            is_rolledover: false,
        },
        TreeAccounts {
            tree_type: TreeType::Address,
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
            is_rolledover: false,
        },
    ];

    let registered_epoch = Epoch::register(&mut rpc, &protocol_config, &forester_keypair)
        .await
        .unwrap();
    assert!(registered_epoch.is_some());
    let mut registered_epoch = registered_epoch.unwrap();
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert!(forester_epoch_pda.total_epoch_state_weight.is_none());
    assert_eq!(forester_epoch_pda.epoch, 0);
    let epoch = 0;
    let expected_stake = 1;
    assert_epoch_pda(&mut rpc, epoch, expected_stake).await;
    assert_registered_forester_pda(
        &mut rpc,
        &registered_epoch.forester_epoch_pda,
        &forester_keypair.pubkey(),
        epoch,
    )
    .await;

    // advance epoch to active phase
    rpc.warp_to_slot(registered_epoch.phases.active.start)
        .unwrap();
    // finalize registration
    {
        registered_epoch
            .fetch_account_and_add_trees_with_schedule(&mut rpc, tree_accounts)
            .await
            .unwrap();
        let ix = create_finalize_registration_instruction(
            &forester_keypair.pubkey(),
            registered_epoch.epoch,
        );
        rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
            .await
            .unwrap();
        assert_finalized_epoch_registration(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
        )
        .await;
    }
    // TODO: write an e2e test with multiple foresters - essentially integrate into e2e tests and make every round a slot
    // 1. create multiple foresters
    // 2. register them etc.
    // 3. iterate over every light slot
    // 3.1. give a random number of work items
    // 3.2. check for every forester who is eligible

    // create work 1 item in address and nullifier queue each
    let (mut state_merkle_tree_bundle, mut address_merkle_tree, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env).await;
        e2e_env.create_address(None).await;
        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester_keypair, &Pubkey::new_unique(), None)
            .await
            .unwrap();

        (
            e2e_env.indexer.state_merkle_trees[0].clone(),
            e2e_env.indexer.address_merkle_trees[0].clone(),
            e2e_env.rpc,
        )
    };
    println!("performed transactions -----------------------------------------");
    // perform 1 work
    nullify_compressed_accounts(
        &mut rpc,
        &forester_keypair,
        &mut state_merkle_tree_bundle,
        epoch,
    )
    .await;
    empty_address_queue_test(
        &forester_keypair,
        &mut rpc,
        &mut address_merkle_tree,
        false,
        epoch,
    )
    .await
    .unwrap();

    // advance epoch to report work and next registration phase
    rpc.warp_to_slot(
        registered_epoch.phases.report_work.start - protocol_config.registration_phase_length,
    )
    .unwrap();
    // register for next epoch
    let next_registered_epoch = Epoch::register(&mut rpc, &protocol_config, &forester_keypair)
        .await
        .unwrap();
    assert!(next_registered_epoch.is_some());
    let next_registered_epoch = next_registered_epoch.unwrap();
    assert_eq!(next_registered_epoch.epoch, 1);
    assert_epoch_pda(&mut rpc, next_registered_epoch.epoch, expected_stake).await;
    assert_registered_forester_pda(
        &mut rpc,
        &next_registered_epoch.forester_epoch_pda,
        &forester_keypair.pubkey(),
        next_registered_epoch.epoch,
    )
    .await;
    // TODO: check that we can still forest the last epoch
    rpc.warp_to_slot(registered_epoch.phases.report_work.start)
        .unwrap();
    // report work
    {
        let (pre_forester_epoch_pda, pre_epoch_pda) = fetch_epoch_and_forester_pdas(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
        )
        .await;
        let ix = create_report_work_instruction(&forester_keypair.pubkey(), registered_epoch.epoch);
        rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
            .await
            .unwrap();
        assert_report_work(
            &mut rpc,
            &registered_epoch.forester_epoch_pda,
            &registered_epoch.epoch_pda,
            pre_forester_epoch_pda,
            pre_epoch_pda,
        )
        .await;
    }

    // TODO: test claim once implemented
    // advance to post epoch phase
}
// TODO: test edge cases first and last slot of every phase
// TODO: add failing tests, perform actions in invalid phases, pass unrelated epoch and forester pda

/// Test:
/// 1. FAIL: Register a forester with invalid authority
/// 2. FAIL: Update forester authority with invalid authority
/// 3. FAIL: Nullify with invalid authority
/// 4. FAIL: Update address tree with invalid authority
/// 5. FAIL: Rollover address tree with invalid authority
/// 6. FAIL: Rollover state tree with invalid authority
#[tokio::test]
async fn failing_test_forester() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    // 1. FAIL: Register a forester with invalid authority
    {
        let result =
            register_test_forester(&mut rpc, &payer, &Keypair::new(), ForesterConfig::default())
                .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 2. FAIL: Update forester authority with invalid authority
    {
        let (forester_pda, _) = get_forester_pda_address(&env.forester.pubkey());
        let forester_epoch_pda = get_forester_epoch_pda_address(&forester_pda, 0).0;
        let instruction_data = light_registry::instruction::UpdateForesterEpochPda {
            authority: Keypair::new().pubkey(),
        };
        let accounts = light_registry::accounts::UpdateForesterEpochPda {
            forester_epoch_pda,
            signer: payer.pubkey(),
        };
        let ix = Instruction {
            program_id: light_registry::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code = anchor_lang::error::ErrorCode::ConstraintAddress as u32;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 3. FAIL: Nullify with invalid authority
    {
        let expected_error_code =
            light_registry::errors::RegistryError::InvalidForester as u32 + 6000;
        let inputs = CreateNullifyInstructionInputs {
            authority: payer.pubkey(),
            nullifier_queue: env.nullifier_queue_pubkey,
            merkle_tree: env.merkle_tree_pubkey,
            change_log_indices: vec![1],
            leaves_queue_indices: vec![1u16],
            indices: vec![0u64],
            proofs: vec![vec![[0u8; 32]; 26]],
            derivation: payer.pubkey(),
        };
        let mut ix = create_nullify_instruction(inputs, 0);
        let (forester_pda, _) = get_forester_pda_address(&env.forester.pubkey());
        // Swap the derived forester pda with an initialized but invalid one.
        ix.accounts[0].pubkey = get_forester_epoch_pda_address(&forester_pda, 0).0;
        let result = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 4 FAIL: update address Merkle tree failed
    {
        let expected_error_code =
            light_registry::errors::RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority: authority.pubkey(),
                address_merkle_tree: env.address_merkle_tree_pubkey,
                address_queue: env.address_merkle_tree_queue_pubkey,
                changelog_index: 0,
                indexed_changelog_index: 0,
                value: 1,
                low_address_index: 1,
                low_address_value: [0u8; 32],
                low_address_next_index: 1,
                low_address_next_value: [0u8; 32],
                low_address_proof: [[0u8; 32]; 16],
            },
            0,
        );
        let (forester_pda, _) = get_forester_pda_address(&env.forester.pubkey());
        // Swap the derived forester pda with an initialized but invalid one.
        instruction.accounts[0].pubkey = get_forester_epoch_pda_address(&forester_pda, 0).0;

        let result = rpc
            .create_and_send_transaction(&[instruction], &authority.pubkey(), &[&authority])
            .await;
        assert_rpc_error(result, 0, expected_error_code).unwrap();
    }
    // 5. FAIL: rollover address tree with invalid authority
    {
        let new_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_address_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &new_queue_keypair,
            &new_merkle_tree_keypair,
            &env.address_merkle_tree_pubkey,
            &env.address_merkle_tree_queue_pubkey,
            0, // TODO: adapt epoch
        )
        .await;
        let (forester_pda, _) = get_forester_pda_address(&env.forester.pubkey());
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey = get_forester_epoch_pda_address(&forester_pda, 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[&authority, &new_queue_keypair, &new_merkle_tree_keypair],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
    // 6. FAIL: rollover state tree with invalid authority
    {
        let new_nullifier_queue_keypair = Keypair::new();
        let new_state_merkle_tree_keypair = Keypair::new();
        let new_cpi_context = Keypair::new();
        let expected_error_code = RegistryError::InvalidForester as u32 + 6000;
        let authority = rpc.get_payer().insecure_clone();
        let mut instructions = create_rollover_state_merkle_tree_instructions(
            &mut rpc,
            &authority.pubkey(),
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
            &env.merkle_tree_pubkey,
            &env.nullifier_queue_pubkey,
            &new_cpi_context.pubkey(),
            0, // TODO: adapt epoch
        )
        .await;
        let (forester_pda, _) = get_forester_pda_address(&env.forester.pubkey());
        // Swap the derived forester pda with an initialized but invalid one.
        instructions[2].accounts[0].pubkey = get_forester_epoch_pda_address(&forester_pda, 0).0;

        let result = rpc
            .create_and_send_transaction(
                &instructions,
                &authority.pubkey(),
                &[
                    &authority,
                    &new_nullifier_queue_keypair,
                    &new_state_merkle_tree_keypair,
                ],
            )
            .await;
        assert_rpc_error(result, 2, expected_error_code).unwrap();
    }
}

// // cargo test-sbf -p registry-test -- --test update_registry_governance_on_testnet update_forester_on_testnet --ignored --nocapture
// #[ignore]
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn update_forester_on_testnet() {
//     let env_accounts = get_test_env_accounts();
//     let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Devnet, None);
//     // rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 100)
//     //     .await
//     //     .unwrap();
//     let forester_pubkey = Pubkey::from_str("8KEKiyAMugpKq9XCGzx81UtTBuytByW8arm9EaBVpD5k").unwrap();
//     // let forester_account_pubkey = get_forester_pda_address(forester_pubkey).0;
//     let forester_epoch = rpc
//         .get_anchor_account::<ForesterAccount>(&forester_pubkey)
//         .await
//         .unwrap()
//         .unwrap();
//     println!("ForesterEpoch: {:?}", forester_epoch);
//     assert_eq!(forester_epoch.authority, env_accounts.forester.pubkey());
//     panic!("");
//     let updated_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
//     println!("updated keypair: {:?}", updated_keypair.pubkey());
//     update_test_forester(
//         &mut rpc,
//         &env_accounts.forester,
//         Some(&updated_keypair),
//         ForesterConfig::default(),
//     )
//     .await
//     .unwrap();
//     let forester_epoch = rpc
//         .get_anchor_account::<ForesterAccount>(&env_accounts.registered_forester_pda)
//         .await
//         .unwrap()
//         .unwrap();
//     println!("ForesterEpoch: {:?}", forester_epoch);
//     assert_eq!(forester_epoch.authority, updated_keypair.pubkey());
// }

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn update_registry_governance_on_testnet() {
    let env_accounts = get_test_env_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::ZKTestnet, None);
    rpc.airdrop_lamports(
        &env_accounts.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await
    .unwrap();
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env_accounts.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    println!("GroupAuthority: {:?}", governance_authority);
    assert_eq!(
        governance_authority.authority,
        env_accounts.governance_authority.pubkey()
    );

    let updated_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    println!("updated keypair: {:?}", updated_keypair.pubkey());
    let (_, bump) = get_protocol_config_pda_address();
    let instruction = light_registry::instruction::UpdateGovernanceAuthority {
        new_authority: updated_keypair.pubkey(),
        _bump: bump,
        new_config: ProtocolConfig::default(),
    };
    let accounts = light_registry::accounts::UpdateAuthority {
        authority_pda: env_accounts.governance_authority_pda,
        authority: env_accounts.governance_authority.pubkey(),
        new_authority: updated_keypair.pubkey(),
    };
    let ix = Instruction {
        program_id: light_registry::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    let signature = rpc
        .create_and_send_transaction(
            &[ix],
            &env_accounts.governance_authority.pubkey(),
            &[&env_accounts.governance_authority],
        )
        .await
        .unwrap();
    println!("signature: {:?}", signature);
    let governance_authority = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env_accounts.governance_authority_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(governance_authority.authority, updated_keypair.pubkey());
}

// cargo test-sbf -p registry-test -- --test init_accounts --ignored --nocapture
// TODO: refactor into xtask
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn init_accounts() {
    let authority_keypair =
        read_keypair_file("../../target/governance-authority-keypair.json").unwrap();
    let forester_keypair = read_keypair_file("../../target/forester-keypair.json").unwrap();
    println!("authority pubkey: {:?}", authority_keypair.pubkey());
    println!("forester pubkey: {:?}", forester_keypair.pubkey());
    setup_accounts_devnet(&authority_keypair, &forester_keypair).await;
}
