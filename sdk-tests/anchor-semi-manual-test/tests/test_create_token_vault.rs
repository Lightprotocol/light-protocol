mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_semi_manual_test::{
    CreateTokenVaultParams, LightAccountVariant, VaultSeeds, VAULT_AUTH_SEED, VAULT_SEED,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    ColdContext, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Token vault lifecycle: create -> verify on-chain -> warp -> verify compressed -> decompress -> verify restored.
#[tokio::test]
async fn test_create_token_vault_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let (mint, _mint_seed) = shared::setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    let (vault_authority, _auth_bump) =
        Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    let accounts = anchor_semi_manual_test::accounts::CreateTokenVault {
        fee_payer: payer.pubkey(),
        mint,
        vault_authority,
        vault,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = anchor_semi_manual_test::instruction::CreateTokenVault {
        params: CreateTokenVaultParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            vault_bump,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateTokenVault should succeed");

    // PHASE 1: Verify on-chain after creation
    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Token vault should exist on-chain");

    let token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Token");

    let expected_token = Token {
        mint: mint.to_bytes().into(),
        owner: vault_authority.to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: token.extensions.clone(),
    };

    assert_eq!(
        token, expected_token,
        "Token vault should match expected after creation"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &vault, "Vault").await;

    // PHASE 3: Decompress vault
    let vault_iface = rpc
        .get_token_account_interface(&vault, None)
        .await
        .expect("failed to get vault interface")
        .value
        .expect("vault interface should exist");
    assert!(vault_iface.is_cold(), "Vault should be cold");

    let token_data: Token =
        borsh::BorshDeserialize::deserialize(&mut &vault_iface.account.data[..])
            .expect("Failed to parse vault Token");
    let vault_variant = LightAccountVariant::Vault(light_account::token::TokenDataWithSeeds {
        seeds: VaultSeeds { mint },
        token_data,
    });
    let vault_compressed = vault_iface
        .compressed()
        .expect("cold vault must have compressed data");
    let vault_interface = AccountInterface {
        key: vault_iface.key,
        account: vault_iface.account.clone(),
        cold: Some(ColdContext::Account(vault_compressed.account.clone())),
    };
    let vault_spec = PdaSpec::new(vault_interface, vault_variant, program_id);

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(vault_spec)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Vault decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &vault, "Vault").await;

    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Vault should exist on-chain after decompression");

    let actual_token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Token after decompression");

    use light_compressed_account::pubkey::Pubkey as LPubkey;

    let expected_token = Token {
        mint: LPubkey::from(mint.to_bytes()),
        owner: LPubkey::from(vault_authority.to_bytes()),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: actual_token.extensions.clone(),
    };

    assert_eq!(
        actual_token, expected_token,
        "Token vault should match expected after decompression"
    );
}
