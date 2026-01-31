//! Test derived mint pattern - minimal params, program derives everything.

mod shared;

use borsh::BorshDeserialize;
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_program_test::Rpc;
use light_token::instruction::{
    config_pda, find_mint_address, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID,
};
use light_token_interface::state::{BaseMint, Mint, MintMetadata, ACCOUNT_TYPE_MINT};
use manual_test_pinocchio::two_mints::accounts::{
    CreateDerivedMintsParams, MINT_SIGNER_0_SEED, MINT_SIGNER_1_SEED,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Test creating two compressed mints using derived PDA mint signers.
#[tokio::test]
async fn test_create_derived_mints() {
    let (mut rpc, payer, _) = shared::setup_test_env().await;

    let program_id = Pubkey::new_from_array(manual_test_pinocchio::ID);
    let authority = Keypair::new();

    // Derive mint signer PDAs from authority (like macro would)
    let (mint_signer_0, mint_signer_0_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_0_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_signer_1, mint_signer_1_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_1_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDAs from mint signers (light-token derives these)
    let (mint_0, mint_0_bump) = find_mint_address(&mint_signer_0);
    let (mint_1, mint_1_bump) = find_mint_address(&mint_signer_1);

    // Get proof for the mints
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::mint(mint_signer_0),
            CreateAccountsProofInput::mint(mint_signer_1),
        ],
    )
    .await
    .unwrap();

    // Minimal params - only proof + bumps
    let params = CreateDerivedMintsParams {
        create_accounts_proof: proof_result.create_accounts_proof.clone(),
        mint_signer_0_bump,
        mint_signer_1_bump,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(mint_signer_0, false),
        AccountMeta::new_readonly(mint_signer_1, false),
        AccountMeta::new(mint_0, false),
        AccountMeta::new(mint_1, false),
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
            manual_test_pinocchio::discriminators::CREATE_DERIVED_MINTS.as_slice(),
            &borsh::to_vec(&params).unwrap(),
        ]
        .concat(),
    };

    // Sign with payer and authority
    let signers: Vec<&Keypair> = vec![&payer, &authority];

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
        .await
        .expect("CreateDerivedMints should succeed");

    // Verify mints exist on-chain
    let mint_0_account = rpc
        .get_account(mint_0)
        .await
        .unwrap()
        .expect("Mint 0 should exist");
    let mint_1_account = rpc
        .get_account(mint_1)
        .await
        .unwrap()
        .expect("Mint 1 should exist");

    // Deserialize and verify mint 0
    let mint_0_data = Mint::deserialize(&mut &mint_0_account.data[..]).unwrap();
    let compression_0 = mint_0_data.compression;

    let expected_mint_0 = Mint {
        base: BaseMint {
            mint_authority: Some(authority.pubkey().to_bytes().into()),
            supply: 0,
            decimals: 6, // mint::decimals = 6
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 3,
            mint_decompressed: true,
            mint: mint_0.to_bytes().into(),
            mint_signer: mint_signer_0.to_bytes(),
            bump: mint_0_bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: compression_0,
        extensions: None,
    };

    assert_eq!(mint_0_data, expected_mint_0, "Mint 0 should match expected");

    // Deserialize and verify mint 1
    let mint_1_data = Mint::deserialize(&mut &mint_1_account.data[..]).unwrap();
    let compression_1 = mint_1_data.compression;

    let expected_mint_1 = Mint {
        base: BaseMint {
            mint_authority: Some(authority.pubkey().to_bytes().into()),
            supply: 0,
            decimals: 9, // mint::decimals = 9
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 3,
            mint_decompressed: true,
            mint: mint_1.to_bytes().into(),
            mint_signer: mint_signer_1.to_bytes(),
            bump: mint_1_bump,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: compression_1,
        extensions: None,
    };

    assert_eq!(mint_1_data, expected_mint_1, "Mint 1 should match expected");
}
