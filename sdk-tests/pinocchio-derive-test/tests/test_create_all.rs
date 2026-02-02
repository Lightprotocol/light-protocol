mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_program_test::Rpc;
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use pinocchio_derive_test::{
    CreateAllParams, MINT_SIGNER_SEED_A, MINT_SIGNER_SEED_B, RECORD_SEED, VAULT_AUTH_SEED,
    VAULT_SEED,
};
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
    let (ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &ata_mint);

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

    let accounts = pinocchio_derive_test::accounts::CreateAll {
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

    let instruction_data = pinocchio_derive_test::instruction::CreateAll {
        params: CreateAllParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            ata_bump,
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

    // Verify PDA
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist");
    use pinocchio_derive_test::MinimalRecord;
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");
    assert_eq!(record.owner, owner, "Record owner should match");

    // Verify zero-copy
    let zc_account = rpc
        .get_account(zc_record_pda)
        .await
        .unwrap()
        .expect("Zero-copy record should exist");
    use pinocchio_derive_test::ZeroCopyRecord;
    let zc_record: &ZeroCopyRecord = bytemuck::from_bytes(&zc_account.data[8..]);
    assert_eq!(zc_record.owner, owner, "ZC record owner should match");
    assert_eq!(zc_record.counter, 0, "ZC record counter should be 0");

    // Verify ATA
    let ata_account = rpc
        .get_account(ata)
        .await
        .unwrap()
        .expect("ATA should exist");
    use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
    let ata_token: Token = borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..])
        .expect("Failed to deserialize ATA Token");
    use light_compressed_account::pubkey::Pubkey as LPubkey;
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

    // Verify vault
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

    // Verify mint A
    let mint_a_account = rpc
        .get_account(mint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist");
    use light_token_interface::state::Mint;
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    assert_eq!(mint_a.base.decimals, 9, "Mint A should have 9 decimals");

    // Verify mint B
    let mint_b_account = rpc
        .get_account(mint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_b_account.data[..])
        .expect("Failed to deserialize Mint B");
    assert_eq!(mint_b.base.decimals, 6, "Mint B should have 6 decimals");
}
