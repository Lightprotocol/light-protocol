#![cfg(feature = "test-sbf")]

use account_compression::errors::AccountCompressionErrorCode;
use account_compression::{
    self, utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority, RegisteredProgram, ID,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_test_utils::rpc::errors::assert_rpc_error;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::{airdrop_lamports, test_env::SYSTEM_PROGRAM_ID_TEST_KEYPAIR};
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

/// Tests:
/// 1. Create group authority
/// 2. Update group authority
/// 3. Cannot update with invalid authority
/// 4. Add program to group
/// 5. Cannot add program to group with invalid authority
#[tokio::test]
async fn test_create_and_update_group() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    let system_program_id =
        Pubkey::from_str("H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN").unwrap();
    program_test.add_program("light_system_program", system_program_id, None);

    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };

    let seed = Keypair::new();
    let group_accounts = Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.pubkey().to_bytes().as_slice()],
        &ID,
    );

    let instruction_data = account_compression::instruction::InitializeGroupAuthority {
        authority: context.get_payer().pubkey(),
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.get_payer().pubkey(), true),
            AccountMeta::new(seed.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };

    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer(), &seed],
        latest_blockhash,
    );
    context.process_transaction(transaction).await.unwrap();

    let group_authority = context
        .get_anchor_account::<GroupAuthority>(&group_accounts.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(group_authority.authority, context.get_payer().pubkey());
    assert_eq!(group_authority.seed, seed.pubkey());

    let updated_keypair = Keypair::new();
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: updated_keypair.pubkey(),
    };

    // update with new authority
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.get_payer().pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer()],
        latest_blockhash,
    );
    context.process_transaction(transaction).await.unwrap();

    let group_authority = context
        .get_anchor_account::<GroupAuthority>(&group_accounts.0)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(group_authority.authority, updated_keypair.pubkey());
    assert_eq!(group_authority.seed, seed.pubkey());

    // update with old authority should fail
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: context.get_payer().pubkey(),
    };
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.get_payer().pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer()],
        latest_blockhash,
    );
    let update_error = context.process_transaction(transaction).await;
    assert!(update_error.is_err());

    airdrop_lamports(&mut context, &updated_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let system_program_id_keypair = Keypair::from_bytes(&SYSTEM_PROGRAM_ID_TEST_KEYPAIR).unwrap();
    // add new program to group
    let registered_program_pda = Pubkey::find_program_address(
        &[system_program_id_keypair.pubkey().to_bytes().as_slice()],
        &ID,
    )
    .0;

    let register_program_ix = account_compression::instruction::RegisterProgramToGroup {};
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(updated_keypair.pubkey(), true),
            AccountMeta::new(system_program_id_keypair.pubkey(), true),
            AccountMeta::new(registered_program_pda, false),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: register_program_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&updated_keypair.pubkey()),
        &vec![&updated_keypair, &system_program_id_keypair],
        context.get_latest_blockhash().await.unwrap(),
    );
    context.process_transaction(transaction).await.unwrap();
    let registered_program_account = context
        .get_anchor_account::<RegisteredProgram>(&registered_program_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        registered_program_account.registered_program_id,
        system_program_id_keypair.pubkey()
    );
    assert_eq!(
        registered_program_account.group_authority_pda,
        group_accounts.0
    );
    // add new program to group with invalid authority
    let other_program_keypair = Keypair::new();
    let other_program_id = other_program_keypair.pubkey();
    let registered_program_pda =
        Pubkey::find_program_address(&[other_program_id.to_bytes().as_slice()], &ID).0;

    let register_program_ix = account_compression::instruction::RegisterProgramToGroup {};
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.get_payer().pubkey(), true),
            AccountMeta::new(other_program_id, true),
            AccountMeta::new(registered_program_pda, false),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: register_program_ix.data(),
    };

    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer(), &other_program_keypair],
        latest_blockhash,
    );
    let result = context.process_transaction(transaction).await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::InvalidAuthority.into(),
    )
    .unwrap();

    let registered_program_pda = Pubkey::find_program_address(
        &[system_program_id_keypair.pubkey().to_bytes().as_slice()],
        &ID,
    )
    .0;
    // deregister program with invalid authority
    {
        let close_recipient = Pubkey::new_unique();
        let deregister_program_ix = account_compression::instruction::DeregisterProgram {};
        let accounts = account_compression::accounts::DeregisterProgram {
            authority: context.get_payer().pubkey(),
            registered_program_pda: registered_program_pda,
            group_authority_pda: group_accounts.0,
            close_recipient,
        };
        let instruction = Instruction {
            program_id: ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: deregister_program_ix.data(),
        };
        let payer = context.get_payer().insecure_clone();
        let result = context
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }
    // successfully deregister program
    {
        let close_recipient = Pubkey::new_unique();
        let deregister_program_ix = account_compression::instruction::DeregisterProgram {};
        let accounts = account_compression::accounts::DeregisterProgram {
            authority: updated_keypair.pubkey(),
            registered_program_pda: registered_program_pda,
            group_authority_pda: group_accounts.0,
            close_recipient,
        };
        let instruction = Instruction {
            program_id: ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: deregister_program_ix.data(),
        };
        context
            .create_and_send_transaction(
                &[instruction],
                &updated_keypair.pubkey(),
                &[&updated_keypair],
            )
            .await
            .unwrap();
        let closed_registered_program_account =
            context.get_account(registered_program_pda).await.unwrap();
        assert!(closed_registered_program_account.is_none());
        let recpient_balance = context.get_balance(&close_recipient).await.unwrap();
        let rent_exemption = context
            .get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap();
        assert_eq!(recpient_balance, rent_exemption);
    }
}
