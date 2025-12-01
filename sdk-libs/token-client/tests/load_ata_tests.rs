#![cfg(feature = "test-sbf")]

use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use light_test_utils::spl::{create_mint_helper, mint_spl_tokens};
use light_token_client::actions::transfer2::{load_ata, load_ata_instructions};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

fn get_ata_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(owner, mint, token_program)
}

async fn create_ata_at_derived_address(
    rpc: &mut LightProgramTest,
    mint: &Pubkey,
    owner: &Pubkey,
    payer: &Keypair,
    is_t22: bool,
) -> Pubkey {
    let token_program_id = if is_t22 {
        anchor_spl::token_2022::ID
    } else {
        anchor_spl::token::ID
    };
    let ata = get_ata_address(owner, mint, &token_program_id);

    let create_ata_ix = solana_sdk::instruction::Instruction {
        program_id: anchor_spl::associated_token::ID,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(payer.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new(ata, false),
            solana_sdk::instruction::AccountMeta::new_readonly(*owner, false),
            solana_sdk::instruction::AccountMeta::new_readonly(*mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                solana_sdk::system_program::ID,
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(token_program_id, false),
        ],
        data: vec![],
    };

    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    ata
}

#[tokio::test]
async fn test_load_ata_empty_returns_no_instructions() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();
    rpc.airdrop_lamports(&owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let ctoken_ata = Keypair::new().pubkey();

    let instructions =
        load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, owner.pubkey(), mint)
            .await
            .unwrap();

    assert!(
        instructions.is_empty(),
        "Expected no instructions when no balances exist"
    );
}

#[tokio::test]
async fn test_load_ata_spl_only() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let spl_balance = 1_000_000u64;

    // Create standard ATA at derived address
    let spl_ata =
        create_ata_at_derived_address(&mut rpc, &mint, &payer.pubkey(), &payer, false).await;

    // Mint tokens to the ATA
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_ata,
        &payer.pubkey(),
        &payer,
        spl_balance,
        false,
    )
    .await
    .unwrap();

    let ctoken_ata = Keypair::new().pubkey();

    let instructions =
        load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, payer.pubkey(), mint)
            .await
            .unwrap();

    assert_eq!(instructions.len(), 1, "Expected 1 instruction for SPL wrap");

    // Verify instruction references correct accounts
    let ix = &instructions[0];
    assert!(
        ix.accounts.iter().any(|acc| acc.pubkey == spl_ata),
        "Instruction should reference SPL ATA"
    );
    assert!(
        ix.accounts.iter().any(|acc| acc.pubkey == ctoken_ata),
        "Instruction should reference ctoken ATA"
    );
}

#[tokio::test]
async fn test_load_ata_zero_balance_spl_no_instruction() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL ATA but don't mint any tokens (zero balance)
    let _spl_ata =
        create_ata_at_derived_address(&mut rpc, &mint, &payer.pubkey(), &payer, false).await;

    let ctoken_ata = Keypair::new().pubkey();

    let instructions =
        load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, payer.pubkey(), mint)
            .await
            .unwrap();

    assert!(
        instructions.is_empty(),
        "Expected no instructions when SPL ATA has zero balance"
    );
}

#[tokio::test]
async fn test_load_ata_returns_none_when_empty() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();
    rpc.airdrop_lamports(&owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let ctoken_ata = Keypair::new().pubkey();

    let result = load_ata(&mut rpc, &payer, ctoken_ata, &owner, mint).await;

    assert!(result.is_ok());
    assert!(
        result.unwrap().is_none(),
        "Expected None when no balances to load"
    );
}

#[tokio::test]
async fn test_spl_and_t22_atas_are_different() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let spl_ata = get_ata_address(&owner, &mint, &spl_token::ID);
    let t22_ata = get_ata_address(&owner, &mint, &spl_token_2022::ID);

    assert_ne!(
        spl_ata, t22_ata,
        "SPL and T22 ATAs should be different addresses for same owner/mint"
    );
}

#[tokio::test]
async fn test_load_ata_spl_balance_creates_wrap_instruction() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create and fund SPL ATA with specific amount
    let spl_ata =
        create_ata_at_derived_address(&mut rpc, &mint, &payer.pubkey(), &payer, false).await;
    let balance = 500_000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_ata,
        &payer.pubkey(),
        &payer,
        balance,
        false,
    )
    .await
    .unwrap();

    let ctoken_ata = Keypair::new().pubkey();

    let instructions =
        load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, payer.pubkey(), mint)
            .await
            .unwrap();

    // Verify we get exactly 1 instruction
    assert_eq!(instructions.len(), 1);

    // Verify instruction accounts include the mint
    let ix = &instructions[0];
    assert!(
        ix.accounts.iter().any(|acc| acc.pubkey == mint),
        "Wrap instruction should reference the mint"
    );
}

#[tokio::test]
async fn test_load_ata_different_owner_than_payer() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();
    rpc.airdrop_lamports(&owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL ATA for owner (not payer)
    let spl_ata =
        create_ata_at_derived_address(&mut rpc, &mint, &owner.pubkey(), &payer, false).await;
    let balance = 100_000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_ata,
        &payer.pubkey(),
        &payer,
        balance,
        false,
    )
    .await
    .unwrap();

    let ctoken_ata = Keypair::new().pubkey();

    let instructions =
        load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, owner.pubkey(), mint)
            .await
            .unwrap();

    // Should find the owner's ATA
    assert_eq!(instructions.len(), 1, "Should find owner's SPL ATA balance");

    // Verify instruction references owner's ATA
    let ix = &instructions[0];
    assert!(
        ix.accounts.iter().any(|acc| acc.pubkey == spl_ata),
        "Instruction should reference owner's SPL ATA"
    );
}
