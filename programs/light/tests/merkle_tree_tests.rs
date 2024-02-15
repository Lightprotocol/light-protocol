#![cfg(feature = "test-sbf")]

use std::str::FromStr;

use account_compression::{
    self, indexed_array_from_bytes, utils::constants::GROUP_AUTHORITY_SEED, GroupAuthority, ID,
};
use anchor_lang::{system_program, AnchorDeserialize, InstructionData, ToAccountMetas};
use ark_ff::BigInteger256;
use ark_serialize::CanonicalDeserialize;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexingArray;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};

pub async fn airdrop_lamports(
    banks_client: &mut ProgramTestContext,
    destination_pubkey: &Pubkey,
    lamports: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a transfer instruction
    let transfer_instruction =
        system_instruction::transfer(&banks_client.payer.pubkey(), destination_pubkey, lamports);

    // Create and sign a transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&banks_client.payer.pubkey()),
        &vec![&banks_client.payer],
        banks_client.last_blockhash,
    );

    // Send the transaction
    banks_client
        .banks_client
        .process_transaction(transaction)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_create_and_update_group() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program("light", light::ID, None);

    let compressed_pda_id =
        Pubkey::from_str("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ").unwrap();
    program_test.add_program("psp_compressed_pda", compressed_pda_id, None);

    program_test.set_compute_max_units(1_400_000u64);

    let mut context = program_test.start_with_context().await;

    let seed = [1u8; 32];
    let group_accounts = anchor_lang::prelude::Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.as_slice()],
        &account_compression::ID,
    );

    let instruction_data = account_compression::instruction::InitializeGroupAuthority {
        _seed: seed,
        authority: context.payer.pubkey(),
    };

    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;
    assert_eq!(group_authority.authority, context.payer.pubkey());
    assert_eq!(group_authority.seed, seed);

    let updated_keypair = Keypair::new();
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: updated_keypair.pubkey(),
    };

    // update with new authority
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let group_authority = get_account::<GroupAuthority>(&mut context, group_accounts.0).await;

    assert_eq!(group_authority.authority, updated_keypair.pubkey());
    assert_eq!(group_authority.seed, seed);

    // update with old authority should fail
    let update_group_authority_ix = account_compression::instruction::UpdateGroupAuthority {
        authority: context.payer.pubkey(),
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(updated_keypair.pubkey(), false),
        ],
        data: update_group_authority_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer],
        context.last_blockhash,
    );
    let update_error = context.banks_client.process_transaction(transaction).await;
    assert!(update_error.is_err());

    airdrop_lamports(&mut context, &updated_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    // add new program to group
    let registered_program_pda =
        Pubkey::find_program_address(&[compressed_pda_id.to_bytes().as_slice()], &ID).0;

    let register_program_ix = account_compression::instruction::RegisterProgramToGroup {
        program_id: compressed_pda_id,
    };

    // update with new authority
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(updated_keypair.pubkey(), true),
            AccountMeta::new(registered_program_pda, false),
            AccountMeta::new(group_accounts.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: register_program_ix.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&updated_keypair.pubkey()),
        &vec![&updated_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn get_account<T: AnchorDeserialize>(context: &mut ProgramTestContext, pubkey: Pubkey) -> T {
    let account = context
        .banks_client
        .get_account(pubkey)
        .await
        .unwrap()
        .unwrap();
    T::deserialize(&mut &account.data[8..]).unwrap()
}
