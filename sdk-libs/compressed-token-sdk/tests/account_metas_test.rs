use anchor_lang::ToAccountMetas;
use light_account_checks::account_info::test_account_info::solana_program::TestAccount;
use light_compressed_token_sdk::cpi::accounts::to_compressed_token_account_metas;
use light_compressed_token_types::{
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID,
        NOOP_PROGRAM_ID, PROGRAM_ID as COMPRESSED_TOKEN_PROGRAM_ID,
    },
    cpi_accounts::{CpiAccounts, CpiAccountsConfig},
};
use solana_pubkey::Pubkey;

#[test]
fn test_to_compressed_token_account_metas_compress() {
    // Create test accounts
    let fee_payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let registered_program_pda = Pubkey::new_unique();
    let account_compression_authority = Pubkey::new_unique();
    let system_program = Pubkey::default();

    // Create TestAccounts and get AccountInfo references
    let mut fee_payer_account = TestAccount::new(fee_payer, Pubkey::default(), 0);
    fee_payer_account.writable = true;

    let mut light_system_account =
        TestAccount::new(Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID), Pubkey::default(), 0);
    let mut authority_account = TestAccount::new(authority, Pubkey::default(), 0);
    let mut registered_program_account =
        TestAccount::new(registered_program_pda, Pubkey::default(), 0);
    let mut noop_account = TestAccount::new(Pubkey::from(NOOP_PROGRAM_ID), Pubkey::default(), 0);
    let mut compression_authority_account =
        TestAccount::new(account_compression_authority, Pubkey::default(), 0);
    let mut compression_program_account = TestAccount::new(
        Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        Pubkey::default(),
        0,
    );
    let mut token_program_account = TestAccount::new(
        Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        Pubkey::default(),
        0,
    );
    let mut system_program_account = TestAccount::new(system_program, Pubkey::default(), 0);

    let fee_payer_info = fee_payer_account.get_account_info();

    // Create account infos in the correct order for CpiAccounts
    let account_infos = vec![
        light_system_account.get_account_info(), // 0: light_system_program
        authority_account.get_account_info(),    // 1: authority
        registered_program_account.get_account_info(), // 2: registered_program_pda
        noop_account.get_account_info(),         // 3: noop_program
        compression_authority_account.get_account_info(), // 4: account_compression_authority
        compression_program_account.get_account_info(), // 5: account_compression_program
        token_program_account.get_account_info(), // 6: invoking_program (self_program)
        system_program_account.get_account_info(), // 7: system_program
    ];
    let reference = light_compressed_token::accounts::TransferInstruction {
        fee_payer,
        authority,
        registered_program_pda,
        noop_program: Pubkey::from(NOOP_PROGRAM_ID),
        account_compression_authority,
        account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        self_program: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
        light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        token_pool_pda: None,
        compress_or_decompress_token_account: None,
        token_program: None,
        system_program,
    };

    // Create CpiAccounts with default config (no optional accounts)
    let config = CpiAccountsConfig::default();
    let cpi_accounts = CpiAccounts::new_with_config(&fee_payer_info, &account_infos, config);

    // Test our function
    let result = to_compressed_token_account_metas(&cpi_accounts);
    assert!(result.is_ok(), "Function should succeed with valid inputs");

    let account_metas = result.unwrap();
    let reference_metas = reference.to_account_metas(Some(true));

    assert_eq!(account_metas, reference_metas);
}

#[test]
fn test_to_compressed_token_account_metas_with_optional_accounts() {
    // Create test accounts
    let fee_payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let registered_program_pda = Pubkey::new_unique();
    let account_compression_authority = Pubkey::new_unique();
    let system_program = Pubkey::default();

    // Optional accounts
    let token_pool_pda = Pubkey::new_unique();
    let compress_or_decompress_token_account = Pubkey::new_unique();
    let token_program = Pubkey::new_unique();

    // Create TestAccounts and get AccountInfo references
    let mut fee_payer_account = TestAccount::new(fee_payer, Pubkey::default(), 0);
    fee_payer_account.writable = true;

    let mut light_system_account =
        TestAccount::new(Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID), Pubkey::default(), 0);
    let mut authority_account = TestAccount::new(authority, Pubkey::default(), 0);
    let mut registered_program_account =
        TestAccount::new(registered_program_pda, Pubkey::default(), 0);
    let mut noop_account = TestAccount::new(Pubkey::from(NOOP_PROGRAM_ID), Pubkey::default(), 0);
    let mut compression_authority_account =
        TestAccount::new(account_compression_authority, Pubkey::default(), 0);
    let mut compression_program_account = TestAccount::new(
        Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        Pubkey::default(),
        0,
    );
    let mut token_program_account = TestAccount::new(
        Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        Pubkey::default(),
        0,
    );
    let mut system_program_account = TestAccount::new(system_program, Pubkey::default(), 0);

    // Optional accounts
    let mut token_pool_account = TestAccount::new(token_pool_pda, Pubkey::default(), 0);
    token_pool_account.writable = true;
    let mut compress_decompress_account =
        TestAccount::new(compress_or_decompress_token_account, Pubkey::default(), 0);
    compress_decompress_account.writable = true;
    let mut token_program_ctx_account = TestAccount::new(token_program, Pubkey::default(), 0);

    let fee_payer_info = fee_payer_account.get_account_info();

    // Create account infos in the correct order for CpiAccounts with all optional accounts
    let account_infos = vec![
        light_system_account.get_account_info(), // 0: light_system_program
        authority_account.get_account_info(),    // 1: authority
        registered_program_account.get_account_info(), // 2: registered_program_pda
        noop_account.get_account_info(),         // 3: noop_program
        compression_authority_account.get_account_info(), // 4: account_compression_authority
        compression_program_account.get_account_info(), // 5: account_compression_program
        token_program_account.get_account_info(), // 6: invoking_program (self_program)
        token_pool_account.get_account_info(),   // 7: token_pool_pda
        compress_decompress_account.get_account_info(), // 8: decompression_recipient (compress_or_decompress_token_account)
        system_program_account.get_account_info(),      // 9: system_program
        token_program_ctx_account.get_account_info(),   // 10: cpi_context (token_program)
    ];

    let reference = light_compressed_token::accounts::TransferInstruction {
        fee_payer,
        authority,
        registered_program_pda,
        noop_program: Pubkey::from(NOOP_PROGRAM_ID),
        account_compression_authority,
        account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        self_program: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
        light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        token_pool_pda: Some(token_pool_pda),
        compress_or_decompress_token_account: Some(compress_or_decompress_token_account),
        token_program: Some(token_program),
        system_program,
    };

    // Create CpiAccounts with config that enables all optional accounts
    let config = CpiAccountsConfig {
        cpi_context: true,
        compress_or_decompress_token_account: true,
        token_pool_pda: true,
    };
    let cpi_accounts = CpiAccounts::new_with_config(&fee_payer_info, &account_infos, config);

    // Test our function
    let result = to_compressed_token_account_metas(&cpi_accounts);
    assert!(result.is_ok(), "Function should succeed with valid inputs");

    let account_metas = result.unwrap();
    let reference_metas = reference.to_account_metas(Some(true));

    assert_eq!(account_metas, reference_metas);
}
