mod shared;

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use anchor_semi_manual_test::{
    CreateAllParams, MinimalRecord, VaultSeeds, ZeroCopyRecord, MINT_SIGNER_SEED_A,
    MINT_SIGNER_SEED_B, RECORD_SEED, VAULT_AUTH_SEED, VAULT_SEED,
};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_all_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    // Setup pre-existing mints for ATA and vault
    let (ata_mint, _) = shared::setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;
    let (vault_mint, _) = shared::setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    let owner = Keypair::new().pubkey();
    let authority = Keypair::new();

    // PDA
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);

    // Zero-copy
    let (zc_record_pda, _) =
        Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    // ATA
    let ata_owner = payer.pubkey();
    let ata = light_token::instruction::derive_token_ata(&ata_owner, &ata_mint);

    // Token vault
    let (vault_authority, _) = Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, vault_mint.as_ref()], &program_id);

    // Mint A
    let (mint_signer_a, mint_signer_bump_a) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_a_pda, _) = light_token::instruction::find_mint_address(&mint_signer_a);

    // Mint B
    let (mint_signer_b, mint_signer_bump_b) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_B, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_b_pda, _) = light_token::instruction::find_mint_address(&mint_signer_b);

    // Build proof inputs for all accounts
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(record_pda),
            CreateAccountsProofInput::pda(zc_record_pda),
            CreateAccountsProofInput::mint(mint_signer_a),
            CreateAccountsProofInput::mint(mint_signer_b),
        ],
    )
    .await
    .unwrap();

    let accounts = anchor_semi_manual_test::accounts::CreateAll {
        fee_payer: payer.pubkey(),
        compression_config: env.config_pda,
        pda_rent_sponsor: env.rent_sponsor,
        record: record_pda,
        zero_copy_record: zc_record_pda,
        ata_mint,
        ata_owner,
        ata,
        vault_mint,
        vault_authority,
        vault,
        authority: authority.pubkey(),
        mint_signer_a,
        mint_a: mint_a_pda,
        mint_signer_b,
        mint_b: mint_b_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = anchor_semi_manual_test::instruction::CreateAll {
        params: CreateAllParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            vault_bump,
            mint_signer_bump_a,
            mint_signer_bump_b,
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
    assert_eq!(record.owner, owner, "Record owner should match");

    let zc_account = rpc
        .get_account(zc_record_pda)
        .await
        .unwrap()
        .expect("Zero-copy record should exist");
    let zc_record: &ZeroCopyRecord = bytemuck::from_bytes(&zc_account.data[8..]);
    assert_eq!(zc_record.owner, owner, "ZC record owner should match");
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
        LPubkey::from(ata_mint.to_bytes()),
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
        LPubkey::from(vault_mint.to_bytes()),
        "Vault mint should match"
    );
    assert_eq!(
        vault_token.owner,
        LPubkey::from(vault_authority.to_bytes()),
        "Vault owner should match"
    );

    use light_token_interface::state::Mint;

    let mint_a_account = rpc
        .get_account(mint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist");
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    assert_eq!(mint_a.base.decimals, 9, "Mint A should have 9 decimals");

    let mint_b_account = rpc
        .get_account(mint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_b_account.data[..])
        .expect("Failed to deserialize Mint B");
    assert_eq!(mint_b.base.decimals, 6, "Mint B should have 6 decimals");

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();

    shared::assert_onchain_closed(&mut rpc, &record_pda, "MinimalRecord").await;
    shared::assert_onchain_closed(&mut rpc, &zc_record_pda, "ZeroCopyRecord").await;
    shared::assert_onchain_closed(&mut rpc, &ata, "ATA").await;
    shared::assert_onchain_closed(&mut rpc, &vault, "Vault").await;
    shared::assert_onchain_closed(&mut rpc, &mint_a_pda, "MintA").await;
    shared::assert_onchain_closed(&mut rpc, &mint_b_pda, "MintB").await;

    // PHASE 3: Decompress all accounts via create_load_instructions.
    use anchor_semi_manual_test::{LightAccountVariant, MinimalRecordSeeds, ZeroCopyRecordSeeds};

    // PDA: MinimalRecord
    let record_interface = rpc
        .get_account_interface(&record_pda, None)
        .await
        .expect("failed to get MinimalRecord interface")
        .value
        .expect("MinimalRecord interface should exist");
    assert!(record_interface.is_cold(), "MinimalRecord should be cold");

    let record_data = MinimalRecord::deserialize(&mut &record_interface.account.data[8..])
        .expect("Failed to parse MinimalRecord");
    let record_variant = LightAccountVariant::MinimalRecord {
        seeds: MinimalRecordSeeds { owner },
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

    let zc_data = ZeroCopyRecord::deserialize(&mut &zc_interface.account.data[8..])
        .expect("Failed to parse ZeroCopyRecord");
    let zc_variant = LightAccountVariant::ZeroCopyRecord {
        seeds: ZeroCopyRecordSeeds { owner },
        data: zc_data,
    };
    let zc_spec = PdaSpec::new(zc_interface, zc_variant, program_id);

    // ATA
    let ata_interface = rpc
        .get_associated_token_account_interface(&ata_owner, &ata_mint, None)
        .await
        .expect("failed to get ATA interface")
        .value
        .expect("ATA interface should exist");
    assert!(ata_interface.is_cold(), "ATA should be cold");

    // Mint A
    let mint_a_iface = rpc
        .get_mint_interface(&mint_a_pda, None)
        .await
        .expect("failed to get mint A interface")
        .value
        .expect("mint A interface should exist");
    assert!(mint_a_iface.is_cold(), "Mint A should be cold");
    let mint_a_ai = AccountInterface::from(mint_a_iface);

    // Mint B
    let mint_b_iface = rpc
        .get_mint_interface(&mint_b_pda, None)
        .await
        .expect("failed to get mint B interface")
        .value
        .expect("mint B interface should exist");
    assert!(mint_b_iface.is_cold(), "Mint B should be cold");
    let mint_b_ai = AccountInterface::from(mint_b_iface);

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
    let vault_variant = LightAccountVariant::Vault(light_account::token::TokenDataWithSeeds {
        seeds: VaultSeeds { mint: vault_mint },
        token_data: vault_token_data,
    });
    let vault_compressed = vault_iface
        .compressed()
        .expect("cold vault must have compressed data");
    let vault_interface = AccountInterface {
        key: vault_iface.key,
        account: vault_iface.account.clone(),
        cold: Some(vault_compressed.account.clone()),
    };
    let vault_spec = PdaSpec::new(vault_interface, vault_variant, program_id);

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![
        AccountSpec::Pda(record_spec),
        AccountSpec::Pda(zc_spec),
        AccountSpec::Ata(Box::new(ata_interface)),
        AccountSpec::Pda(vault_spec),
        AccountSpec::Mint(mint_a_ai),
        AccountSpec::Mint(mint_b_ai),
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
    shared::assert_onchain_exists(&mut rpc, &mint_a_pda, "MintA").await;
    shared::assert_onchain_exists(&mut rpc, &mint_b_pda, "MintB").await;

    // MinimalRecord
    let account = rpc.get_account(record_pda).await.unwrap().unwrap();
    let actual_record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[8..]).unwrap();
    let expected_record = MinimalRecord {
        compression_info: shared::expected_compression_info(&actual_record.compression_info),
        owner,
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
        owner,
        counter: 0,
    };
    assert_eq!(
        *actual_zc, expected_zc,
        "ZeroCopyRecord should match after decompression"
    );

    // ATA
    let actual_ata: Token = shared::parse_token(&rpc.get_account(ata).await.unwrap().unwrap().data);
    let expected_ata = Token {
        mint: LPubkey::from(ata_mint.to_bytes()),
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
        mint: LPubkey::from(vault_mint.to_bytes()),
        owner: LPubkey::from(vault_authority.to_bytes()),
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

    // Mints
    let actual_ma: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_a_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_ma.base.decimals, 9,
        "Mint A decimals should be preserved"
    );
    assert_eq!(
        actual_ma.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint A authority should be preserved"
    );

    let actual_mb: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_b_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_mb.base.decimals, 6,
        "Mint B decimals should be preserved"
    );
    assert_eq!(
        actual_mb.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint B authority should be preserved"
    );
}
