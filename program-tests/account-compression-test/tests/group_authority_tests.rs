#![cfg(feature = "test-sbf")]

use std::str::FromStr;

use account_compression::{
    self,
    errors::AccountCompressionErrorCode,
    utils::constants::{CPI_AUTHORITY_PDA_SEED, GROUP_AUTHORITY_SEED},
    GroupAuthority, RegisteredProgram, RegisteredProgramV1, ID,
};
use anchor_lang::{system_program, AnchorDeserialize, InstructionData, ToAccountMetas};
use light_program_test::{
    accounts::{initialize::get_group_pda, test_keypairs::OLD_SYSTEM_PROGRAM_ID_TEST_KEYPAIR},
    program_test::{LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig,
};
use light_test_utils::{
    airdrop_lamports, registered_program_accounts_v1::get_registered_program_pda, Rpc,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

/// Tests:
/// 1. Create group authority
/// 2. Update group authority
/// 3. Cannot update with invalid authority
/// 4. Add program to group
/// 5. Cannot add program to group with invalid authority
#[tokio::test]
async fn test_create_and_update_group() {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();

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

    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
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

    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
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

    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
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
    let system_program_id_keypair =
        Keypair::try_from(OLD_SYSTEM_PROGRAM_ID_TEST_KEYPAIR.as_slice()).unwrap();
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
        context.get_latest_blockhash().await.unwrap().0,
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

    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
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
            registered_program_pda,
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
    // deregister program with invalid group
    {
        let invalid_group_authority = Keypair::new();
        context
            .airdrop_lamports(&invalid_group_authority.pubkey(), 1_000_000_000)
            .await
            .unwrap();
        let invalid_group = get_group_pda(invalid_group_authority.pubkey());

        let instruction_data = account_compression::instruction::InitializeGroupAuthority {
            authority: invalid_group_authority.pubkey(),
        };

        let instruction = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(invalid_group_authority.pubkey(), true),
                AccountMeta::new(invalid_group_authority.pubkey(), true),
                AccountMeta::new(invalid_group, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        };
        context
            .create_and_send_transaction(
                &[instruction],
                &invalid_group_authority.pubkey(),
                &[&invalid_group_authority],
            )
            .await
            .unwrap();
        let close_recipient = Pubkey::new_unique();
        let deregister_program_ix = account_compression::instruction::DeregisterProgram {};
        let accounts = account_compression::accounts::DeregisterProgram {
            authority: invalid_group_authority.pubkey(),
            registered_program_pda,
            group_authority_pda: invalid_group,
            close_recipient,
        };
        let instruction = Instruction {
            program_id: ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: deregister_program_ix.data(),
        };
        let result = context
            .create_and_send_transaction(
                &[instruction],
                &invalid_group_authority.pubkey(),
                &[&invalid_group_authority],
            )
            .await;
        assert_rpc_error(result, 0, AccountCompressionErrorCode::InvalidGroup.into()).unwrap();
    }
    // successfully deregister program
    {
        let close_recipient = Pubkey::new_unique();
        let deregister_program_ix = account_compression::instruction::DeregisterProgram {};
        let accounts = account_compression::accounts::DeregisterProgram {
            authority: updated_keypair.pubkey(),
            registered_program_pda,
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
        assert_eq!(closed_registered_program_account, None);
        let recipient_balance = context.get_balance(&close_recipient).await.unwrap();
        let rent_exemption = context
            .get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap();
        assert_eq!(recipient_balance, rent_exemption);
    }
}

#[tokio::test]
async fn test_resize_registered_program_pda() {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();
    let system_program_id =
        Pubkey::from_str("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7").unwrap();
    let registered_program = Pubkey::find_program_address(
        &[system_program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    context.set_account(registered_program, get_registered_program_pda());

    let payer = context.get_payer().insecure_clone();

    let instruction_data = account_compression::instruction::ResizeRegisteredProgramPda {};
    let accounts = account_compression::accounts::ResizeRegisteredProgramPda {
        authority: context.get_payer().pubkey(),
        registered_program_pda: registered_program,
        system_program: Pubkey::default(),
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    };
    // Resize
    {
        let pre_account = context
            .get_account(registered_program)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pre_account.data.len(), RegisteredProgramV1::LEN);
        let account_data = RegisteredProgramV1::deserialize(&mut &pre_account.data[8..]).unwrap();
        println!("account_data: {:?}", account_data);
        let mut transaction =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);
        context.process_transaction(transaction).await.unwrap();

        let account = context
            .get_account(registered_program)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.data.len(), RegisteredProgram::LEN);
        let expected_registered_program = RegisteredProgram {
            registered_program_id: system_program_id,
            group_authority_pda: account_data.group_authority_pda,
            registered_program_signer_pda: Pubkey::find_program_address(
                &[CPI_AUTHORITY_PDA_SEED],
                &system_program_id,
            )
            .0,
        };
        let account_des = RegisteredProgram::deserialize(&mut &account.data[8..]).unwrap();
        assert_eq!(expected_registered_program, account_des);
    }
    // Resize again should fail.
    {
        let mut transaction =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);
        let result = context.process_transaction(transaction).await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintRaw.into(),
        )
        .unwrap();
    }

    // Invalid program owner.
    {
        let mut account = get_registered_program_pda();
        account.owner = Pubkey::new_unique();
        let config = ProgramTestConfig {
            skip_protocol_init: true,
            ..Default::default()
        };
        let mut context = LightProgramTest::new(config).await.unwrap();
        let system_program_id =
            Pubkey::from_str("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7").unwrap();
        let registered_program = Pubkey::find_program_address(
            &[system_program_id.to_bytes().as_slice()],
            &account_compression::ID,
        )
        .0;
        context.set_account(registered_program, account);
        let payer = context.get_payer().insecure_clone();

        let instruction_data = account_compression::instruction::ResizeRegisteredProgramPda {};
        let accounts = account_compression::accounts::ResizeRegisteredProgramPda {
            authority: context.get_payer().pubkey(),
            registered_program_pda: registered_program,
            system_program: Pubkey::default(),
        };
        let instruction = Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let mut transaction =
            Transaction::new_with_payer(std::slice::from_ref(&instruction), Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);
        let result = context.process_transaction(transaction).await;
        assert_rpc_error(
            result,
            0,
            light_account_checks::error::AccountError::AccountOwnedByWrongProgram.into(),
        )
        .unwrap();
    }
    // Invalid account discriminator.
    {
        let mut account = get_registered_program_pda();
        account.data[0..8].copy_from_slice(&[1u8; 8]);
        let system_program_id =
            Pubkey::from_str("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7").unwrap();
        let registered_program = Pubkey::find_program_address(
            &[system_program_id.to_bytes().as_slice()],
            &account_compression::ID,
        )
        .0;
        let config = ProgramTestConfig {
            skip_protocol_init: true,
            ..Default::default()
        };
        let mut context = LightProgramTest::new(config).await.unwrap();
        context.set_account(registered_program, account);

        let payer = context.get_payer().insecure_clone();
        let instruction_data = account_compression::instruction::ResizeRegisteredProgramPda {};
        let accounts = account_compression::accounts::ResizeRegisteredProgramPda {
            authority: context.get_payer().pubkey(),
            registered_program_pda: registered_program,
            system_program: Pubkey::default(),
        };
        let instruction = Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        let recent_blockhash = context.get_latest_blockhash().await.unwrap().0;
        transaction.sign(&[&payer], recent_blockhash);
        let result = context.process_transaction(transaction).await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
}
