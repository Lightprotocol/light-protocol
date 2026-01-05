// Tests for TransferCTokenCheckedCpi with different mint types

mod shared;
use anchor_spl::token::{spl_token, Mint};
use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_ctoken_interface::state::CToken;
use light_ctoken_sdk::{
    ctoken::{derive_ctoken_ata, CreateAssociatedCTokenAccount, TransferSplToCtoken},
    spl_interface::{find_spl_interface_pda_with_index, CreateSplInterfacePda},
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    mint_2022::{create_mint_22_with_extensions, create_token_22_account, mint_spl_tokens_22},
    spl::{create_token_account, mint_spl_tokens},
};
use native_ctoken_examples::{InstructionType, TransferCheckedData, ID};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test transfer_checked with SPL Token mint
#[tokio::test]
async fn test_ctoken_transfer_checked_spl_mint() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create SPL mint
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        mint_rent,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let initialize_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &payer.pubkey(),
        Some(&payer.pubkey()),
        decimals,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_mint_account_ix, initialize_mint_ix],
        &payer.pubkey(),
        &[&payer, &mint_keypair],
    )
    .await
    .unwrap();

    // Create token pool for SPL interface
    let create_pool_ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint, anchor_spl::token::ID, false)
            .instruction();

    rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account_keypair, &payer)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        1000,
        false,
    )
    .await
    .unwrap();

    // Create cToken ATAs for source and destination
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let (source_ata, _) = derive_ctoken_ata(&source_owner, &mint);
    let (dest_ata, _) = derive_ctoken_ata(&dest_owner, &mint);

    let create_source_ata = CreateAssociatedCTokenAccount::new(payer.pubkey(), source_owner, mint)
        .instruction()
        .unwrap();
    let create_dest_ata = CreateAssociatedCTokenAccount::new(payer.pubkey(), dest_owner, mint)
        .instruction()
        .unwrap();

    rpc.create_and_send_transaction(
        &[create_source_ata, create_dest_ata],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .unwrap();

    // Transfer SPL tokens to source cToken ATA
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let transfer_to_ctoken = TransferSplToCtoken {
        amount: 1000,
        spl_interface_pda_bump,
        decimals,
        source_spl_token_account: spl_token_account_keypair.pubkey(),
        destination_ctoken_account: source_ata,
        authority: payer.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: anchor_spl::token::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_to_ctoken], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Execute transfer_checked via wrapper program
    let transfer_data = TransferCheckedData {
        amount: 500,
        decimals,
    };
    let mut instruction_data = vec![InstructionType::CTokenTransferCheckedInvoke as u8];
    transfer_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = light_ctoken_sdk::ctoken::CTOKEN_PROGRAM_ID;
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(source_owner, true),
            AccountMeta::new_readonly(ctoken_program, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = CToken::deserialize(&mut &source_data.data[..]).unwrap();
    assert_eq!(source_state.amount, 500);

    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = CToken::deserialize(&mut &dest_data.data[..]).unwrap();
    assert_eq!(dest_state.amount, 500);
}

/// Test transfer_checked with Token-2022 mint
#[tokio::test]
async fn test_ctoken_transfer_checked_t22_mint() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 2u8;

    // Create Token-2022 mint with extensions
    let (mint_keypair, _extension_config) =
        create_mint_22_with_extensions(&mut rpc, &payer, decimals).await;
    let mint = mint_keypair.pubkey();

    // Create T22 token account and mint tokens
    let t22_token_account = create_token_22_account(&mut rpc, &payer, &mint, &payer.pubkey()).await;
    mint_spl_tokens_22(&mut rpc, &payer, &mint, &t22_token_account, 1000).await;

    // Create cToken ATAs for source and destination with compression_only for T22 restricted extensions
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let (source_ata, _) = derive_ctoken_ata(&source_owner, &mint);
    let (dest_ata, _) = derive_ctoken_ata(&dest_owner, &mint);

    use light_ctoken_sdk::ctoken::CompressibleParams;
    let compressible_params = CompressibleParams {
        compression_only: true,
        ..Default::default()
    };

    let create_source_ata = CreateAssociatedCTokenAccount::new(payer.pubkey(), source_owner, mint)
        .with_compressible(compressible_params.clone())
        .instruction()
        .unwrap();
    let create_dest_ata = CreateAssociatedCTokenAccount::new(payer.pubkey(), dest_owner, mint)
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

    rpc.create_and_send_transaction(
        &[create_source_ata, create_dest_ata],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .unwrap();

    // Transfer T22 tokens to source cToken ATA (use restricted=true for mints with restricted extensions)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, true);
    let transfer_to_ctoken = TransferSplToCtoken {
        amount: 1000,
        spl_interface_pda_bump,
        decimals,
        source_spl_token_account: t22_token_account,
        destination_ctoken_account: source_ata,
        authority: payer.pubkey(),
        mint,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    rpc.create_and_send_transaction(&[transfer_to_ctoken], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Execute transfer_checked via wrapper program
    let transfer_data = TransferCheckedData {
        amount: 500,
        decimals,
    };
    let mut instruction_data = vec![InstructionType::CTokenTransferCheckedInvoke as u8];
    transfer_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = light_ctoken_sdk::ctoken::CTOKEN_PROGRAM_ID;
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(source_owner, true),
            AccountMeta::new_readonly(ctoken_program, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = CToken::deserialize(&mut &source_data.data[..]).unwrap();
    assert_eq!(source_state.amount, 500);

    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = CToken::deserialize(&mut &dest_data.data[..]).unwrap();
    assert_eq!(dest_state.amount, 500);
}

/// Test transfer_checked with decompressed CMint
#[tokio::test]
async fn test_ctoken_transfer_checked_cmint() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    // Create compressed mint and decompress it, then create ATAs with tokens
    let (mint, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint_with_freeze_authority(
            &mut rpc,
            &payer,
            payer.pubkey(),
            None, // no freeze authority needed for transfer
            decimals,
            vec![(1000, source_owner), (0, dest_owner)],
        )
        .await;

    let source_ata = ata_pubkeys[0];
    let dest_ata = ata_pubkeys[1];

    // Execute transfer_checked via wrapper program
    let transfer_data = TransferCheckedData {
        amount: 500,
        decimals,
    };
    let mut instruction_data = vec![InstructionType::CTokenTransferCheckedInvoke as u8];
    transfer_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = light_ctoken_sdk::ctoken::CTOKEN_PROGRAM_ID;
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(source_owner, true),
            AccountMeta::new_readonly(ctoken_program, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = CToken::deserialize(&mut &source_data.data[..]).unwrap();
    assert_eq!(source_state.amount, 500);

    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = CToken::deserialize(&mut &dest_data.data[..]).unwrap();
    assert_eq!(dest_state.amount, 500);
}
