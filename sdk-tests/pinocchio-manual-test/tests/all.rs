//! Test create_all instruction - all account types in a single instruction.
//!
//! Creates:
//! - Borsh PDA (MinimalRecord)
//! - ZeroCopy PDA (ZeroCopyRecord)
//! - Compressed Mint
//! - Token Vault
//! - Associated Token Account (ATA)

mod shared;

use borsh::BorshDeserialize;
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_program_test::Rpc;
use light_token::instruction::{
    config_pda, derive_associated_token_account, find_mint_address, rent_sponsor_pda,
    LIGHT_TOKEN_PROGRAM_ID,
};
use light_token_interface::state::{
    AccountState, BaseMint, Mint, MintMetadata, Token, ACCOUNT_TYPE_MINT,
};
use pinocchio_manual_test::{
    all::accounts::{
        CreateAllParams, ALL_BORSH_SEED, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
        ALL_ZERO_COPY_SEED,
    },
    MinimalRecord, ZeroCopyRecord,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Test creating all account types in a single instruction.
#[tokio::test]
async fn test_create_all() {
    let (mut rpc, payer, config_pda_addr) = shared::setup_test_env().await;

    let program_id = Pubkey::new_from_array(pinocchio_manual_test::ID);
    let authority = Keypair::new();
    let owner = Keypair::new().pubkey();
    let value: u64 = 42;

    // ========== Derive all addresses ==========

    // PDAs (using ALL module-specific seeds)
    let (borsh_record_pda, _) =
        Pubkey::find_program_address(&[ALL_BORSH_SEED, owner.as_ref()], &program_id);
    let (zero_copy_record_pda, _) =
        Pubkey::find_program_address(&[ALL_ZERO_COPY_SEED, owner.as_ref()], &program_id);

    // Mint signer and mint
    let (mint_signer, mint_signer_bump) = Pubkey::find_program_address(
        &[ALL_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint, _mint_bump) = find_mint_address(&mint_signer);

    // Token vault
    let (token_vault, token_vault_bump) =
        Pubkey::find_program_address(&[ALL_TOKEN_VAULT_SEED, mint.as_ref()], &program_id);
    let vault_owner = Keypair::new();

    // ATA
    let ata_owner = Keypair::new();
    let (user_ata, _) = derive_associated_token_account(&ata_owner.pubkey(), &mint);

    // ========== Get proof for 2 PDAs + 1 Mint ==========
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(borsh_record_pda),
            CreateAccountsProofInput::pda(zero_copy_record_pda),
            CreateAccountsProofInput::mint(mint_signer),
        ],
    )
    .await
    .unwrap();

    // ========== Build and send instruction ==========
    let params = CreateAllParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        mint_signer_bump,
        token_vault_bump,
        owner: owner.to_bytes(),
        value,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(config_pda_addr, false),
        AccountMeta::new(borsh_record_pda, false),
        AccountMeta::new(zero_copy_record_pda, false),
        AccountMeta::new_readonly(mint_signer, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_vault, false),
        AccountMeta::new_readonly(vault_owner.pubkey(), false),
        AccountMeta::new_readonly(ata_owner.pubkey(), false),
        AccountMeta::new(user_ata, false),
        AccountMeta::new_readonly(config_pda(), false),
        AccountMeta::new(rent_sponsor_pda(), false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(
            Pubkey::new_from_array(light_token_types::CPI_AUTHORITY_PDA),
            false,
        ),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let ix = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: [
            pinocchio_manual_test::discriminators::CREATE_ALL.as_slice(),
            &borsh::to_vec(&params).unwrap(),
        ]
        .concat(),
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateAll should succeed");

    // ========== Verify all 5 accounts exist with correct data ==========

    // 1. Verify Borsh PDA
    let borsh_account = rpc
        .get_account(borsh_record_pda)
        .await
        .unwrap()
        .expect("Borsh PDA should exist");

    let borsh_record =
        MinimalRecord::deserialize(&mut &borsh_account.data[8..]).expect("Should deserialize");
    assert_eq!(
        borsh_record.owner,
        owner.to_bytes(),
        "Borsh PDA owner should match"
    );

    // 2. Verify ZeroCopy PDA
    let zero_copy_account = rpc
        .get_account(zero_copy_record_pda)
        .await
        .unwrap()
        .expect("ZeroCopy PDA should exist");

    let record_bytes = &zero_copy_account.data[8..8 + core::mem::size_of::<ZeroCopyRecord>()];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(record_bytes);
    assert_eq!(
        Pubkey::new_from_array(record.owner),
        owner,
        "ZeroCopy PDA owner should match"
    );
    assert_eq!(record.value, value, "ZeroCopy PDA value should match");

    // 3. Verify Mint
    let mint_account = rpc
        .get_account(mint)
        .await
        .unwrap()
        .expect("Mint should exist");

    let mint_data = Mint::deserialize(&mut &mint_account.data[..]).expect("Should deserialize");
    let compression = mint_data.compression;

    let expected_mint = Mint {
        base: BaseMint {
            mint_authority: Some(authority.pubkey().to_bytes().into()),
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 3,
            mint_decompressed: true,
            mint: mint.to_bytes().into(),
            mint_signer: mint_signer.to_bytes(),
            bump: _mint_bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression,
        extensions: None,
    };

    assert_eq!(mint_data, expected_mint, "Mint should match expected");

    // 4. Verify Token Vault
    let vault_account = rpc
        .get_account(token_vault)
        .await
        .unwrap()
        .expect("Token vault should exist");

    let token =
        Token::deserialize(&mut &vault_account.data[..]).expect("Should deserialize as Token");
    assert_eq!(token.mint.to_bytes(), mint.to_bytes());
    assert_eq!(token.owner.to_bytes(), vault_owner.pubkey().to_bytes());
    assert_eq!(token.amount, 0);
    assert_eq!(token.state, AccountState::Initialized);

    // 5. Verify ATA
    let ata_account = rpc
        .get_account(user_ata)
        .await
        .unwrap()
        .expect("ATA should exist");

    let ata_token =
        Token::deserialize(&mut &ata_account.data[..]).expect("Should deserialize as Token");
    assert_eq!(ata_token.mint.to_bytes(), mint.to_bytes());
    assert_eq!(ata_token.owner.to_bytes(), ata_owner.pubkey().to_bytes());
    assert_eq!(ata_token.amount, 0);
    assert_eq!(ata_token.state, AccountState::Initialized);
}
