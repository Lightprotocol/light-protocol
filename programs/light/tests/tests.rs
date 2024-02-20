#![cfg(feature = "test-sbf")]

use account_compression::{
    self, utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority, RegisteredProgram, ID,
};
use anchor_lang::{system_program, InstructionData};
use light_test_utils::{airdrop_lamports, create_and_send_transaction, get_account};
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
};

pub fn create_initiatialize_group_authority_instruction(
    signer_pubkey: Pubkey,
    authority_pda: Pubkey,
    group_accounts: Pubkey,
    seed: [u8; 32],
) -> Instruction {
    let instruction_data = account_compression::instruction::InitializeGroupAuthority {
        _seed: seed,
        authority: authority_pda,
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(group_accounts, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };
    instruction
}

pub fn create_update_authority_instruction(
    signer_pubkey: Pubkey,
    authority_pda: (Pubkey, u8),
    group_account: Pubkey,
    new_authority: Pubkey,
) -> Instruction {
    let update_authority_ix = light::instruction::UpdateAuthority {
        bump: authority_pda.1,
        new_authority,
    };

    // update with new authority
    let instruction = Instruction {
        program_id: light::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
            AccountMeta::new(group_account, false),
            AccountMeta::new_readonly(account_compression::ID, false),
        ],
        data: update_authority_ix.data(),
    };
    instruction
}

pub fn create_register_program_instruction(
    signer_pubkey: Pubkey,
    authority_pda: (Pubkey, u8),
    group_account: Pubkey,
    program_id_to_be_registered: Pubkey,
) -> (Instruction, Pubkey) {
    let registered_program_pda =
        Pubkey::find_program_address(&[program_id_to_be_registered.to_bytes().as_slice()], &ID).0;

    let register_program_ix = light::instruction::RegisterSystemProgram {
        bump: authority_pda.1,
        program_id: program_id_to_be_registered,
    };

    let instruction = Instruction {
        program_id: light::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
            AccountMeta::new(group_account, false),
            AccountMeta::new_readonly(account_compression::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new(registered_program_pda, false),
        ],
        data: register_program_ix.data(),
    };
    (instruction, registered_program_pda)
}

#[tokio::test]
async fn test_create_and_update_group() {
    let mut program_test = ProgramTest::default();
    println!("account compression ID: {:?}", ID);
    program_test.add_program("light", light::ID, None);
    println!(" light::ID: {:?}", light::ID);

    program_test.add_program("account_compression", ID, None);
    program_test.set_compute_max_units(1_400_000u64);
    let authority_pda = Pubkey::find_program_address(
        &[light::AUTHORITY_PDA_SEED, &light::ID.to_bytes().as_slice()],
        &light::ID,
    );
    let mut context = program_test.start_with_context().await;
    let updated_keypair = Keypair::new();

    airdrop_lamports(&mut context, &updated_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let seed = [1u8; 32];
    let group_accounts = anchor_lang::prelude::Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.as_slice()],
        &account_compression::ID,
    );

    let instruction = create_initiatialize_group_authority_instruction(
        updated_keypair.pubkey(),
        authority_pda.0,
        group_accounts.0,
        seed,
    );

    create_and_send_transaction(&mut context, &[instruction], &updated_keypair)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;
    assert_eq!(group_authority.authority, authority_pda.0);
    assert_eq!(group_authority.seed, seed);

    let instruction = create_update_authority_instruction(
        updated_keypair.pubkey(),
        authority_pda,
        group_accounts.0,
        updated_keypair.pubkey(),
    );
    create_and_send_transaction(&mut context, &[instruction], &updated_keypair)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;

    assert_eq!(group_authority.authority, updated_keypair.pubkey());
    assert_eq!(group_authority.seed, seed);

    // update with authority pda
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: authority_pda.0,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(updated_keypair.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    create_and_send_transaction(&mut context, &[instruction], &updated_keypair)
        .await
        .unwrap();

    // register itself as system program (doesn't make sense it's just a test)
    let (instruction, _) = create_register_program_instruction(
        updated_keypair.pubkey(),
        authority_pda,
        group_accounts.0,
        light::ID,
    );

    let transfer_instruction = system_instruction::transfer(
        &updated_keypair.pubkey(),
        &authority_pda.0,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(RegisteredProgram::LEN),
    );

    create_and_send_transaction(
        &mut context,
        &[transfer_instruction, instruction],
        &updated_keypair,
    )
    .await
    .unwrap();
}
