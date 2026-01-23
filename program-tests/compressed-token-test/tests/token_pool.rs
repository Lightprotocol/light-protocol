#![cfg(feature = "test-sbf")]

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::{
    token::{Mint, TokenAccount},
    token_2022::spl_token_2022::{self, extension::ExtensionType},
};
use forester_utils::instructions::create_account_instruction;
use light_compressed_token::{
    constants::NUM_MAX_POOL_ACCOUNTS, get_token_pool_pda, get_token_pool_pda_with_index,
    mint_sdk::create_create_token_pool_instruction, process_transfer::get_cpi_authority_pda,
    spl_compression::check_spl_token_pool_derivation_with_index, ErrorCode,
};
use light_compressed_token_sdk::spl_interface::CreateSplInterfacePda;
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    spl::{create_additional_token_pools, create_mint_22_helper, create_mint_helper},
    Rpc, RpcError,
};
use light_token_interface::{
    find_spl_interface_pda, find_spl_interface_pda_with_index, has_restricted_extensions,
};
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use solana_system_interface::instruction as system_instruction;
use spl_token::instruction::initialize_mint;

#[serial]
#[tokio::test]
async fn test_create_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    create_additional_token_pools(&mut rpc, &payer, &mint, false, NUM_MAX_POOL_ACCOUNTS)
        .await
        .unwrap();
    let mint_22 = create_mint_22_helper(&mut rpc, &payer).await;
    create_additional_token_pools(&mut rpc, &payer, &mint_22, true, NUM_MAX_POOL_ACCOUNTS)
        .await
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_failing_create_token_pool() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let mint_1_keypair = Keypair::new();
    let mint_1_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        rent,
        &spl_token::ID,
        Some(&mint_1_keypair),
    );
    let create_mint_1_ix = initialize_mint(
        &spl_token::ID,
        &mint_1_keypair.pubkey(),
        &payer.pubkey(),
        Some(&payer.pubkey()),
        2,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[mint_1_account_create_ix, create_mint_1_ix],
        &payer.pubkey(),
        &[&payer, &mint_1_keypair],
    )
    .await
    .unwrap();
    let mint_1_pool_pda = get_token_pool_pda(&mint_1_keypair.pubkey());

    let mint_2_keypair = Keypair::new();
    let mint_2_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        rent,
        &spl_token::ID,
        Some(&mint_2_keypair),
    );
    let create_mint_2_ix = initialize_mint(
        &spl_token::ID,
        &mint_2_keypair.pubkey(),
        &payer.pubkey(),
        Some(&payer.pubkey()),
        2,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[mint_2_account_create_ix, create_mint_2_ix],
        &payer.pubkey(),
        &[&payer, &mint_2_keypair],
    )
    .await
    .unwrap();
    let mint_2_pool_pda = get_token_pool_pda(&mint_2_keypair.pubkey());

    // Try to create pool for `mint_1` while using seeds of `mint_2` for PDAs.
    {
        let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
        let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
            fee_payer: payer.pubkey(),
            token_pool_pda: mint_2_pool_pda,
            system_program: system_program::ID,
            mint: mint_1_keypair.pubkey(),
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // Invalid program id.
    {
        let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
        let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
            fee_payer: payer.pubkey(),
            token_pool_pda: mint_1_pool_pda,
            system_program: system_program::ID,
            mint: mint_1_keypair.pubkey(),
            token_program: light_system_program::ID, // invalid program id should be spl token program or token 2022 program
            cpi_authority_pda: get_cpi_authority_pda().0,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::InvalidProgramId.into(),
        )
        .unwrap();
    }
    // Try to create pool for `mint_2` while using seeds of `mint_1` for PDAs.
    {
        let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
        let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
            fee_payer: payer.pubkey(),
            token_pool_pda: mint_1_pool_pda,
            system_program: system_program::ID,
            mint: mint_2_keypair.pubkey(),
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // failing test try to create a token pool with mint with non-whitelisted token extension
    {
        let payer = rpc.get_payer().insecure_clone();
        let payer_pubkey = payer.pubkey();
        let mint = Keypair::new();
        let token_authority = payer.insecure_clone();
        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
            ExtensionType::NonTransferable,
        ])
        .unwrap();

        let mut instructions = vec![system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rpc.get_minimum_balance_for_rent_exemption(space)
                .await
                .unwrap(),
            space as u64,
            &spl_token_2022::ID,
        )];
        let invalid_token_extension_ix =
            spl_token_2022::instruction::initialize_non_transferable_mint(
                &spl_token_2022::ID,
                &mint.pubkey(),
            )
            .unwrap();
        instructions.push(invalid_token_extension_ix);
        instructions.push(
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::ID,
                &mint.pubkey(),
                &token_authority.pubkey(),
                None,
                2,
            )
            .unwrap(),
        );
        instructions.push(create_create_token_pool_instruction(
            &payer_pubkey,
            &mint.pubkey(),
            true,
        ));

        let result = rpc
            .create_and_send_transaction(&instructions, &payer_pubkey, &[&payer, &mint])
            .await;
        assert_rpc_error(result, 3, ErrorCode::MintWithInvalidExtension.into()).unwrap();
    }
    // functional create token pool account with token 2022 mint with allowed metadata pointer extension
    {
        let payer = rpc.get_payer().insecure_clone();
        // create_mint_helper(&mut rpc, &payer).await;
        let payer_pubkey = payer.pubkey();

        let mint = Keypair::new();
        let token_authority = payer.insecure_clone();
        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
            ExtensionType::MetadataPointer,
        ])
        .unwrap();

        let mut instructions = vec![system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rpc.get_minimum_balance_for_rent_exemption(space)
                .await
                .unwrap(),
            space as u64,
            &spl_token_2022::ID,
        )];
        let token_extension_ix =
            spl_token_2022::extension::metadata_pointer::instruction::initialize(
                &spl_token_2022::ID,
                &mint.pubkey(),
                Some(token_authority.pubkey()),
                None,
            )
            .unwrap();
        instructions.push(token_extension_ix);
        instructions.push(
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::ID,
                &mint.pubkey(),
                &token_authority.pubkey(),
                None,
                2,
            )
            .unwrap(),
        );
        instructions.push(create_create_token_pool_instruction(
            &payer_pubkey,
            &mint.pubkey(),
            true,
        ));
        rpc.create_and_send_transaction(&instructions, &payer_pubkey, &[&payer, &mint])
            .await
            .unwrap();

        let token_pool_pubkey = get_token_pool_pda(&mint.pubkey());
        let token_pool_account = rpc.get_account(token_pool_pubkey).await.unwrap().unwrap();
        check_spl_token_pool_derivation_with_index(
            &mint.pubkey().to_bytes(),
            &token_pool_pubkey,
            &[0],
        )
        .unwrap();
        // MetadataPointer is a mint-only extension, so token account has base size (165 bytes)
        assert_eq!(token_pool_account.data.len(), TokenAccount::LEN);
    }
}

#[serial]
#[tokio::test]
async fn failing_tests_add_token_pool() {
    for is_token_22 in [false, true] {
        let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
            .await
            .unwrap();
        let payer = rpc.get_payer().insecure_clone();

        let mint = if !is_token_22 {
            create_mint_helper(&mut rpc, &payer).await
        } else {
            create_mint_22_helper(&mut rpc, &payer).await
        };
        let invalid_mint = if !is_token_22 {
            create_mint_helper(&mut rpc, &payer).await
        } else {
            create_mint_22_helper(&mut rpc, &payer).await
        };
        let mut current_token_pool_bump = 1;
        create_additional_token_pools(&mut rpc, &payer, &mint, is_token_22, 2)
            .await
            .unwrap();
        create_additional_token_pools(&mut rpc, &payer, &invalid_mint, is_token_22, 2)
            .await
            .unwrap();
        current_token_pool_bump += 2;
        // 1. failing invalid existing token pool pda
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidExistingTokenPoolPda,
            )
            .await;
            assert_rpc_error(result, 0, ErrorCode::InvalidTokenPoolPda.into()).unwrap();
        }
        // 2. failing InvalidTokenPoolPda
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidTokenPoolPda,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
            )
            .unwrap();
        }
        // 3. failing invalid system program id
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidSystemProgramId,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                anchor_lang::error::ErrorCode::InvalidProgramId.into(),
            )
            .unwrap();
        }
        // 4. failing invalid mint - now fails with ConstraintSeeds because mint validation
        // happens after PDA derivation (mint changed from InterfaceAccount to AccountInfo)
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidMint,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
            )
            .unwrap();
        }
        // 5. failing inconsistent mints
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                Some(invalid_mint),
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InconsistentMints,
            )
            .await;
            assert_rpc_error(result, 0, ErrorCode::InvalidTokenPoolPda.into()).unwrap();
        }
        // 6. failing invalid program id
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidTokenProgramId,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                anchor_lang::error::ErrorCode::InvalidProgramId.into(),
            )
            .unwrap();
        }
        // 7. failing invalid cpi authority pda
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                current_token_pool_bump,
                is_token_22,
                FailingTestsAddTokenPool::InvalidCpiAuthorityPda,
            )
            .await;
            assert_rpc_error(
                result,
                0,
                anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
            )
            .unwrap();
        }
        // create all remaining token pools
        create_additional_token_pools(&mut rpc, &payer, &mint, is_token_22, 5)
            .await
            .unwrap();
        // 8. failing invalid token pool bump (too large)
        {
            let result = add_token_pool(
                &mut rpc,
                &payer,
                &mint,
                None,
                NUM_MAX_POOL_ACCOUNTS,
                is_token_22,
                FailingTestsAddTokenPool::Functional,
            )
            .await;
            assert_rpc_error(result, 0, ErrorCode::InvalidTokenPoolBump.into()).unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailingTestsAddTokenPool {
    Functional,
    InvalidMint,
    InconsistentMints,
    InvalidTokenPoolPda,
    InvalidSystemProgramId,
    InvalidExistingTokenPoolPda,
    InvalidCpiAuthorityPda,
    InvalidTokenProgramId,
}

pub async fn add_token_pool<R: Rpc>(
    rpc: &mut R,
    fee_payer: &Keypair,
    mint: &Pubkey,
    invalid_mint: Option<Pubkey>,
    token_pool_index: u8,
    is_token_22: bool,
    mode: FailingTestsAddTokenPool,
) -> Result<Signature, RpcError> {
    let token_pool_pda = if mode == FailingTestsAddTokenPool::InvalidTokenPoolPda {
        Pubkey::new_unique()
    } else {
        get_token_pool_pda_with_index(mint, token_pool_index)
    };
    let existing_token_pool_pda = if mode == FailingTestsAddTokenPool::InvalidExistingTokenPoolPda {
        get_token_pool_pda_with_index(mint, token_pool_index.saturating_sub(2))
    } else if let Some(invalid_mint) = invalid_mint {
        get_token_pool_pda_with_index(&invalid_mint, token_pool_index.saturating_sub(1))
    } else {
        get_token_pool_pda_with_index(mint, token_pool_index.saturating_sub(1))
    };
    let instruction_data = light_compressed_token::instruction::AddTokenPool { token_pool_index };

    let token_program: Pubkey = if mode == FailingTestsAddTokenPool::InvalidTokenProgramId {
        Pubkey::new_unique()
    } else if is_token_22 {
        anchor_spl::token_2022::ID
    } else {
        anchor_spl::token::ID
    };
    let cpi_authority_pda = if mode == FailingTestsAddTokenPool::InvalidCpiAuthorityPda {
        Pubkey::new_unique()
    } else {
        get_cpi_authority_pda().0
    };
    let system_program = if mode == FailingTestsAddTokenPool::InvalidSystemProgramId {
        Pubkey::new_unique()
    } else {
        system_program::ID
    };
    let mint = if mode == FailingTestsAddTokenPool::InvalidMint {
        Pubkey::new_unique()
    } else {
        *mint
    };

    let accounts = light_compressed_token::accounts::AddTokenPoolInstruction {
        fee_payer: fee_payer.pubkey(),
        token_pool_pda,
        system_program,
        mint,
        token_program,
        cpi_authority_pda,
        existing_token_pool_pda,
    };

    let instruction = Instruction {
        program_id: light_compressed_token::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    };
    rpc.create_and_send_transaction(&[instruction], &fee_payer.pubkey(), &[fee_payer])
        .await
}

/// Test that restricted extensions are properly detected and use different pool derivations.
///
/// This test verifies:
/// 1. Mints with restricted extensions (Pausable, PermanentDelegate, TransferFeeConfig, TransferHook)
///    are detected by `has_restricted_extensions()`
/// 2. Restricted and non-restricted pool PDAs are different
/// 3. The anchor `create_token_pool` instruction still works for restricted mints
///    (uses normal derivation, which is intentional for backward compatibility)
#[serial]
#[tokio::test]
async fn test_restricted_mint_pool_derivation() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test PermanentDelegate (a restricted extension)
    let extension_type = ExtensionType::PermanentDelegate;
    println!("Testing restricted extension: {:?}", extension_type);

    // Create mint with restricted extension
    let mint = Keypair::new();
    let space =
        ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[extension_type])
            .unwrap();

    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rpc.get_minimum_balance_for_rent_exemption(space)
                .await
                .unwrap(),
            space as u64,
            &spl_token_2022::ID,
        ),
        spl_token_2022::instruction::initialize_permanent_delegate(
            &spl_token_2022::ID,
            &mint.pubkey(),
            &payer.pubkey(),
        )
        .unwrap(),
        spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            2,
        )
        .unwrap(),
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer, &mint])
        .await
        .unwrap();

    // Fetch mint account and verify restricted extensions are detected
    let mint_account = rpc.get_account(mint.pubkey()).await.unwrap().unwrap();
    assert!(
        has_restricted_extensions(&mint_account.data),
        "Mint with PermanentDelegate should be detected as restricted"
    );

    // Verify that restricted and non-restricted PDAs are different
    let (regular_pda, _) = find_spl_interface_pda(&mint.pubkey(), false);
    let (restricted_pda, _) = find_spl_interface_pda(&mint.pubkey(), true);
    assert_ne!(
        regular_pda, restricted_pda,
        "Regular and restricted PDAs should be different"
    );

    // Verify with index derivation as well
    for index in 0..NUM_MAX_POOL_ACCOUNTS {
        let (regular_pda_idx, _) = find_spl_interface_pda_with_index(&mint.pubkey(), index, false);
        let (restricted_pda_idx, _) =
            find_spl_interface_pda_with_index(&mint.pubkey(), index, true);
        assert_ne!(
            regular_pda_idx, restricted_pda_idx,
            "Regular and restricted PDAs for index {} should be different",
            index
        );
    }

    // The anchor create_token_pool instruction automatically uses restricted derivation
    // for mints with restricted extensions (detected via restricted_seed() function)
    let create_pool_ix = CreateSplInterfacePda::new(
        payer.pubkey(),
        mint.pubkey(),
        spl_token_2022::ID,
        true, // restricted = true for mints with restricted extensions
    )
    .instruction();

    rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify pool was created at restricted derivation
    let token_pool_account = rpc.get_account(restricted_pda).await.unwrap();
    assert!(
        token_pool_account.is_some(),
        "Token pool should exist at restricted derivation"
    );

    println!(
        "Successfully tested PermanentDelegate: regular_pda={}, restricted_pda={}",
        regular_pda, restricted_pda
    );
}

/// Test that non-restricted mints are correctly identified.
#[serial]
#[tokio::test]
async fn test_non_restricted_mint_detection() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a regular SPL token mint (not Token-2022)
    let spl_mint = create_mint_helper(&mut rpc, &payer).await;
    let spl_mint_account = rpc.get_account(spl_mint).await.unwrap().unwrap();
    assert!(
        !has_restricted_extensions(&spl_mint_account.data),
        "Regular SPL mint should not be restricted"
    );

    // Create a Token-2022 mint with only non-restricted extensions
    let mint = Keypair::new();
    let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
        ExtensionType::MetadataPointer,
    ])
    .unwrap();

    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rpc.get_minimum_balance_for_rent_exemption(space)
                .await
                .unwrap(),
            space as u64,
            &spl_token_2022::ID,
        ),
        spl_token_2022::extension::metadata_pointer::instruction::initialize(
            &spl_token_2022::ID,
            &mint.pubkey(),
            Some(payer.pubkey()),
            None,
        )
        .unwrap(),
        spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            2,
        )
        .unwrap(),
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer, &mint])
        .await
        .unwrap();

    // Verify non-restricted extension is not detected as restricted
    let mint_account = rpc.get_account(mint.pubkey()).await.unwrap().unwrap();
    assert!(
        !has_restricted_extensions(&mint_account.data),
        "Mint with MetadataPointer should not be restricted"
    );
}

/// Test creating all 5 SPL interface PDAs (index 0-4) using the SDK.
/// Tests both regular and restricted mints.
#[serial]
#[tokio::test]
async fn test_create_all_spl_interface_pdas_with_sdk() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test 1: Regular SPL mint (non-restricted)
    {
        // Create mint without a pool
        let mint = Keypair::new();
        let rent = rpc
            .get_minimum_balance_for_rent_exemption(Mint::LEN)
            .await
            .unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rent,
                Mint::LEN as u64,
                &spl_token::ID,
            ),
            initialize_mint(&spl_token::ID, &mint.pubkey(), &payer.pubkey(), None, 2).unwrap(),
        ];

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer, &mint])
            .await
            .unwrap();

        println!("Testing regular SPL mint: {}", mint.pubkey());

        // Create all 5 pools using SDK
        for index in 0..NUM_MAX_POOL_ACCOUNTS {
            let create_pool_ix = CreateSplInterfacePda::new_with_index(
                payer.pubkey(),
                mint.pubkey(),
                spl_token::ID,
                index,
                false, // not restricted
            )
            .instruction();

            rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
                .await
                .unwrap();

            // Verify pool was created at correct derivation
            let (expected_pda, _) = find_spl_interface_pda_with_index(&mint.pubkey(), index, false);
            let pool_account = rpc.get_account(expected_pda).await.unwrap();
            assert!(
                pool_account.is_some(),
                "Pool at index {} should exist for regular mint",
                index
            );
            println!("Created pool at index {}: {}", index, expected_pda);
        }
    }

    // Test 2: Token-2022 mint with restricted extension (PermanentDelegate)
    {
        let mint = Keypair::new();
        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&[
            ExtensionType::PermanentDelegate,
        ])
        .unwrap();

        let instructions = vec![
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rpc.get_minimum_balance_for_rent_exemption(space)
                    .await
                    .unwrap(),
                space as u64,
                &spl_token_2022::ID,
            ),
            spl_token_2022::instruction::initialize_permanent_delegate(
                &spl_token_2022::ID,
                &mint.pubkey(),
                &payer.pubkey(),
            )
            .unwrap(),
            spl_token_2022::instruction::initialize_mint(
                &spl_token_2022::ID,
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                2,
            )
            .unwrap(),
        ];

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer, &mint])
            .await
            .unwrap();

        println!(
            "Testing restricted Token-2022 mint (PermanentDelegate): {}",
            mint.pubkey()
        );

        // Verify it's detected as restricted
        let mint_account = rpc.get_account(mint.pubkey()).await.unwrap().unwrap();
        assert!(
            has_restricted_extensions(&mint_account.data),
            "Mint should be detected as restricted"
        );

        // Create all 5 pools using SDK with restricted = true
        for index in 0..NUM_MAX_POOL_ACCOUNTS {
            let create_pool_ix = CreateSplInterfacePda::new_with_index(
                payer.pubkey(),
                mint.pubkey(),
                spl_token_2022::ID,
                index,
                true, // restricted
            )
            .instruction();

            rpc.create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
                .await
                .unwrap();

            // Verify pool was created at restricted derivation
            let (restricted_pda, _) =
                find_spl_interface_pda_with_index(&mint.pubkey(), index, true);
            let pool_account = rpc.get_account(restricted_pda).await.unwrap();
            assert!(
                pool_account.is_some(),
                "Pool at index {} should exist for restricted mint",
                index
            );

            // Verify it's NOT at the regular derivation
            let (regular_pda, _) = find_spl_interface_pda_with_index(&mint.pubkey(), index, false);
            let regular_account = rpc.get_account(regular_pda).await.unwrap();
            assert!(
                regular_account.is_none(),
                "Pool at index {} should NOT exist at regular derivation",
                index
            );

            println!(
                "Created restricted pool at index {}: {}",
                index, restricted_pda
            );
        }
    }

    println!("Successfully created all SPL interface PDAs for both regular and restricted mints");
}
