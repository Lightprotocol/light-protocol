// Tests for ApproveCTokenCpi and RevokeCTokenCpi invoke() and invoke_signed()

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_ctoken_interface::state::CToken;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use native_ctoken_examples::{ApproveData, InstructionType, ID, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test approving a delegate using ApproveCTokenCpi::invoke()
#[tokio::test]
async fn test_approve_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint with an ATA for the payer with 1000 tokens
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_compressed_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];
    let delegate = Keypair::new();
    let approve_amount = 100u64;

    // Build approve instruction via wrapper program
    let mut instruction_data = vec![InstructionType::ApproveInvoke as u8];
    let approve_data = ApproveData {
        amount: approve_amount,
    };
    approve_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(C_TOKEN_PROGRAM_ID);
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                        // token_account
            AccountMeta::new_readonly(delegate.pubkey(), false), // delegate
            AccountMeta::new(payer.pubkey(), true),              // owner (signer)
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(ctoken_program, false),    // ctoken_program
        ],
        data: instruction_data,
    };

    // Execute the approve instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the delegate was set
    let ata_account = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken = CToken::deserialize(&mut &ata_account.data[..]).unwrap();

    assert_eq!(
        ctoken.delegate,
        Some(delegate.pubkey().to_bytes().into()),
        "Delegate should be set after approve"
    );
    assert_eq!(
        ctoken.delegated_amount, approve_amount,
        "Delegated amount should match"
    );
}

/// Test approving a delegate for a PDA-owned account using ApproveCTokenCpi::invoke_signed()
#[tokio::test]
async fn test_approve_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the token account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Create a compressed mint with an ATA for the PDA owner with 1000 tokens
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) =
        setup_create_compressed_mint(&mut rpc, &payer, payer.pubkey(), 9, vec![(1000, pda_owner)])
            .await;

    let ata = ata_pubkeys[0];
    let delegate = Keypair::new();
    let approve_amount = 100u64;

    // Build approve instruction via wrapper program using invoke_signed
    let mut instruction_data = vec![InstructionType::ApproveInvokeSigned as u8];
    let approve_data = ApproveData {
        amount: approve_amount,
    };
    approve_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(C_TOKEN_PROGRAM_ID);
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                        // token_account
            AccountMeta::new_readonly(delegate.pubkey(), false), // delegate
            AccountMeta::new(pda_owner, false),                  // PDA owner (program signs)
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(ctoken_program, false),    // ctoken_program
        ],
        data: instruction_data,
    };

    // Execute the approve instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the delegate was set
    let ata_account = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken = CToken::deserialize(&mut &ata_account.data[..]).unwrap();

    assert_eq!(
        ctoken.delegate,
        Some(delegate.pubkey().to_bytes().into()),
        "Delegate should be set after approve"
    );
    assert_eq!(
        ctoken.delegated_amount, approve_amount,
        "Delegated amount should match"
    );
}

/// Test revoking delegation using RevokeCTokenCpi::invoke()
#[tokio::test]
async fn test_revoke_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint with an ATA for the payer with 1000 tokens
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_compressed_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];
    let delegate = Keypair::new();
    let approve_amount = 100u64;
    let ctoken_program = Pubkey::from(C_TOKEN_PROGRAM_ID);

    // First approve a delegate
    let mut approve_instruction_data = vec![InstructionType::ApproveInvoke as u8];
    let approve_data = ApproveData {
        amount: approve_amount,
    };
    approve_data
        .serialize(&mut approve_instruction_data)
        .unwrap();

    let approve_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(delegate.pubkey(), false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new_readonly(ctoken_program, false),
        ],
        data: approve_instruction_data,
    };

    rpc.create_and_send_transaction(&[approve_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify delegate was set
    let ata_account_after_approve = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_approve =
        CToken::deserialize(&mut &ata_account_after_approve.data[..]).unwrap();
    assert!(
        ctoken_after_approve.delegate.is_some(),
        "Delegate should be set"
    );

    // Now revoke the delegation
    let revoke_instruction_data = vec![InstructionType::RevokeInvoke as u8];

    let revoke_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                        // token_account
            AccountMeta::new(payer.pubkey(), true),              // owner (signer)
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(ctoken_program, false),    // ctoken_program
        ],
        data: revoke_instruction_data,
    };

    rpc.create_and_send_transaction(&[revoke_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the delegate was cleared
    let ata_account_after_revoke = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_revoke = CToken::deserialize(&mut &ata_account_after_revoke.data[..]).unwrap();

    assert_eq!(
        ctoken_after_revoke.delegate, None,
        "Delegate should be cleared after revoke"
    );
    assert_eq!(
        ctoken_after_revoke.delegated_amount, 0,
        "Delegated amount should be 0 after revoke"
    );
}

/// Test revoking delegation for a PDA-owned account using RevokeCTokenCpi::invoke_signed()
#[tokio::test]
async fn test_revoke_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the token account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Create a compressed mint with an ATA for the PDA owner with 1000 tokens
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) =
        setup_create_compressed_mint(&mut rpc, &payer, payer.pubkey(), 9, vec![(1000, pda_owner)])
            .await;

    let ata = ata_pubkeys[0];
    let delegate = Keypair::new();
    let approve_amount = 100u64;
    let ctoken_program = Pubkey::from(C_TOKEN_PROGRAM_ID);

    // First approve a delegate using invoke_signed
    let mut approve_instruction_data = vec![InstructionType::ApproveInvokeSigned as u8];
    let approve_data = ApproveData {
        amount: approve_amount,
    };
    approve_data
        .serialize(&mut approve_instruction_data)
        .unwrap();

    let approve_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(delegate.pubkey(), false),
            AccountMeta::new(pda_owner, false),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new_readonly(ctoken_program, false),
        ],
        data: approve_instruction_data,
    };

    rpc.create_and_send_transaction(&[approve_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify delegate was set
    let ata_account_after_approve = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_approve =
        CToken::deserialize(&mut &ata_account_after_approve.data[..]).unwrap();
    assert!(
        ctoken_after_approve.delegate.is_some(),
        "Delegate should be set"
    );

    // Now revoke the delegation using invoke_signed
    let revoke_instruction_data = vec![InstructionType::RevokeInvokeSigned as u8];

    let revoke_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                        // token_account
            AccountMeta::new(pda_owner, false),                  // PDA owner (program signs)
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(ctoken_program, false),    // ctoken_program
        ],
        data: revoke_instruction_data,
    };

    rpc.create_and_send_transaction(&[revoke_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the delegate was cleared
    let ata_account_after_revoke = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_revoke = CToken::deserialize(&mut &ata_account_after_revoke.data[..]).unwrap();

    assert_eq!(
        ctoken_after_revoke.delegate, None,
        "Delegate should be cleared after revoke"
    );
    assert_eq!(
        ctoken_after_revoke.delegated_amount, 0,
        "Delegated amount should be 0 after revoke"
    );
}
