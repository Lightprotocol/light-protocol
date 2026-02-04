mod shared;

use light_account_pinocchio::token::TokenDataWithSeeds;
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    ColdContext, CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
use pinocchio_light_program_test::{
    all::accounts::CreateAllParams, discriminators, LightAccountVariant, MinimalRecord,
    MinimalRecordSeeds, VaultSeeds, ZeroCopyRecord, ZeroCopyRecordSeeds, MINT_SIGNER_SEED_A,
    RECORD_SEED, VAULT_AUTH_SEED, VAULT_SEED,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_all_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let owner = Keypair::new().pubkey();
    let authority = Keypair::new();

    // PDA: MinimalRecord
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);

    // PDA: ZeroCopyRecord
    let (zc_record_pda, _) =
        Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    // Mint signer PDA
    let (mint_signer, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_pda, _) = light_token::instruction::find_mint_address(&mint_signer);

    // Token vault PDA (uses the mint we're creating)
    let (vault_owner, _) = Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint_pda.as_ref()], &program_id);

    // ATA (uses the mint we're creating)
    let ata_owner = payer.pubkey();
    let (ata, _) = light_token::instruction::derive_token_ata(&ata_owner, &mint_pda);

    // Build proof inputs for PDA accounts and the mint
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(record_pda),
            CreateAccountsProofInput::pda(zc_record_pda),
            CreateAccountsProofInput::mint(mint_signer),
        ],
    )
    .await
    .unwrap();

    let params = CreateAllParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        owner: owner.to_bytes(),
        mint_signer_bump,
        token_vault_bump: vault_bump,
    };

    // Account order per all/accounts.rs:
    // [0] payer (signer, writable)
    // [1] authority (signer)
    // [2] compression_config
    // [3] borsh_record (writable)
    // [4] zero_copy_record (writable)
    // [5] mint_signer
    // [6] mint (writable)
    // [7] token_vault (writable)
    // [8] vault_owner
    // [9] ata_owner
    // [10] user_ata (writable)
    // [11] compressible_config (LIGHT_TOKEN_CONFIG)
    // [12] rent_sponsor (LIGHT_TOKEN_RENT_SPONSOR, writable)
    // [13] light_token_program
    // [14] cpi_authority
    // [15] system_program
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(env.config_pda, false),
        AccountMeta::new(record_pda, false),
        AccountMeta::new(zc_record_pda, false),
        AccountMeta::new_readonly(mint_signer, false),
        AccountMeta::new(mint_pda, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(vault_owner, false),
        AccountMeta::new_readonly(ata_owner, false),
        AccountMeta::new(ata, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token_types::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_ALL, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateAll should succeed");

    // PHASE 1: Verify all accounts on-chain after creation
    use light_compressed_account::pubkey::Pubkey as LPubkey;

    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist");
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");
    assert_eq!(record.owner, owner.to_bytes(), "Record owner should match");

    let zc_account = rpc
        .get_account(zc_record_pda)
        .await
        .unwrap()
        .expect("Zero-copy record should exist");
    let zc_record: &ZeroCopyRecord = bytemuck::from_bytes(&zc_account.data[8..]);
    assert_eq!(
        zc_record.owner,
        owner.to_bytes(),
        "ZC record owner should match"
    );
    assert_eq!(zc_record.counter, 0, "ZC record counter should be 0");

    let ata_account = rpc
        .get_account(ata)
        .await
        .unwrap()
        .expect("ATA should exist");
    let ata_token: Token = borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..])
        .expect("Failed to deserialize ATA Token");
    assert_eq!(
        ata_token.mint,
        LPubkey::from(mint_pda.to_bytes()),
        "ATA mint should match"
    );
    assert_eq!(
        ata_token.owner,
        LPubkey::from(ata_owner.to_bytes()),
        "ATA owner should match"
    );

    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Vault should exist");
    let vault_token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Vault Token");
    assert_eq!(
        vault_token.mint,
        LPubkey::from(mint_pda.to_bytes()),
        "Vault mint should match"
    );
    assert_eq!(
        vault_token.owner,
        LPubkey::from(vault_owner.to_bytes()),
        "Vault owner should match"
    );

    use light_token_interface::state::Mint;

    let mint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist");
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_account.data[..])
        .expect("Failed to deserialize Mint");
    assert_eq!(mint.base.decimals, 9, "Mint should have 9 decimals");

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    shared::assert_onchain_closed(&mut rpc, &record_pda, "MinimalRecord").await;
    shared::assert_onchain_closed(&mut rpc, &zc_record_pda, "ZeroCopyRecord").await;
    shared::assert_onchain_closed(&mut rpc, &ata, "ATA").await;
    shared::assert_onchain_closed(&mut rpc, &vault, "Vault").await;
    shared::assert_onchain_closed(&mut rpc, &mint_pda, "Mint").await;

    // PHASE 3: Decompress all accounts via create_load_instructions

    // PDA: MinimalRecord
    let record_interface = rpc
        .get_account_interface(&record_pda, None)
        .await
        .expect("failed to get MinimalRecord interface")
        .value
        .expect("MinimalRecord interface should exist");
    assert!(record_interface.is_cold(), "MinimalRecord should be cold");

    let record_data: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_interface.account.data[8..])
            .expect("Failed to parse MinimalRecord");
    let record_variant = LightAccountVariant::MinimalRecord {
        seeds: MinimalRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: record_data,
    };
    let record_spec = PdaSpec::new(record_interface, record_variant, program_id);

    // PDA: ZeroCopyRecord
    let zc_interface = rpc
        .get_account_interface(&zc_record_pda, None)
        .await
        .expect("failed to get ZeroCopyRecord interface")
        .value
        .expect("ZeroCopyRecord interface should exist");
    assert!(zc_interface.is_cold(), "ZeroCopyRecord should be cold");

    let zc_data: ZeroCopyRecord =
        borsh::BorshDeserialize::deserialize(&mut &zc_interface.account.data[8..])
            .expect("Failed to parse ZeroCopyRecord");
    let zc_variant = LightAccountVariant::ZeroCopyRecord {
        seeds: ZeroCopyRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: zc_data,
    };
    let zc_spec = PdaSpec::new(zc_interface, zc_variant, program_id);

    // ATA
    let ata_interface = rpc
        .get_ata_interface(&ata_owner, &mint_pda, None)
        .await
        .expect("failed to get ATA interface")
        .value
        .expect("ATA interface should exist");
    assert!(ata_interface.is_cold(), "ATA should be cold");

    // Token PDA: Vault
    let vault_iface = rpc
        .get_token_account_interface(&vault, None)
        .await
        .expect("failed to get vault interface")
        .value
        .expect("vault interface should exist");
    assert!(vault_iface.is_cold(), "Vault should be cold");

    let vault_token_data: Token =
        borsh::BorshDeserialize::deserialize(&mut &vault_iface.account.data[..])
            .expect("Failed to parse vault Token");
    let vault_variant = LightAccountVariant::Vault(TokenDataWithSeeds {
        seeds: VaultSeeds {
            mint: mint_pda.to_bytes(),
        },
        token_data: vault_token_data,
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

    // Mint
    let mint_iface = rpc
        .get_mint_interface(&mint_pda, None)
        .await
        .expect("failed to get mint interface")
        .value
        .expect("mint interface should exist");
    assert!(mint_iface.is_cold(), "Mint should be cold");
    let (compressed_mint, _) = mint_iface
        .compressed()
        .expect("cold mint must have compressed data");
    let mint_ai = AccountInterface {
        key: mint_pda,
        account: solana_account::Account {
            lamports: 0,
            data: vec![],
            owner: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
        cold: Some(ColdContext::Account(compressed_mint.clone())),
    };

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![
        AccountSpec::Pda(record_spec),
        AccountSpec::Pda(zc_spec),
        AccountSpec::Ata(ata_interface),
        AccountSpec::Pda(vault_spec),
        AccountSpec::Mint(mint_ai),
    ];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &record_pda, "MinimalRecord").await;
    shared::assert_onchain_exists(&mut rpc, &zc_record_pda, "ZeroCopyRecord").await;
    shared::assert_onchain_exists(&mut rpc, &ata, "ATA").await;
    shared::assert_onchain_exists(&mut rpc, &vault, "Vault").await;
    shared::assert_onchain_exists(&mut rpc, &mint_pda, "Mint").await;

    // MinimalRecord
    let account = rpc.get_account(record_pda).await.unwrap().unwrap();
    let actual_record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    let expected_record = MinimalRecord {
        compression_info: shared::expected_compression_info(&actual_record.compression_info),
        owner: owner.to_bytes(),
    };
    assert_eq!(
        actual_record, expected_record,
        "MinimalRecord should match after decompression"
    );

    // ZeroCopyRecord
    let account = rpc.get_account(zc_record_pda).await.unwrap().unwrap();
    let actual_zc: &ZeroCopyRecord = bytemuck::from_bytes(&account.data[8..]);
    let expected_zc = ZeroCopyRecord {
        compression_info: shared::expected_compression_info(&actual_zc.compression_info),
        owner: owner.to_bytes(),
        counter: 0,
    };
    assert_eq!(
        *actual_zc, expected_zc,
        "ZeroCopyRecord should match after decompression"
    );

    // ATA
    let actual_ata: Token = shared::parse_token(&rpc.get_account(ata).await.unwrap().unwrap().data);
    let expected_ata = Token {
        mint: LPubkey::from(mint_pda.to_bytes()),
        owner: LPubkey::from(ata_owner.to_bytes()),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: actual_ata.extensions.clone(),
    };
    assert_eq!(
        actual_ata, expected_ata,
        "ATA should match after decompression"
    );

    // Vault
    let actual_vault: Token =
        shared::parse_token(&rpc.get_account(vault).await.unwrap().unwrap().data);
    let expected_vault = Token {
        mint: LPubkey::from(mint_pda.to_bytes()),
        owner: LPubkey::from(vault_owner.to_bytes()),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: actual_vault.extensions.clone(),
    };
    assert_eq!(
        actual_vault, expected_vault,
        "Vault should match after decompression"
    );

    // Mint
    let actual_mint: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_mint.base.decimals, 9,
        "Mint decimals should be preserved"
    );
    assert_eq!(
        actual_mint.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint authority should be preserved"
    );
}
