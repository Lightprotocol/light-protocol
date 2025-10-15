use anchor_lang::ToAccountMetas;
use light_compressed_token_sdk::instructions::{
    batch_compress::{get_batch_compress_instruction_account_metas, BatchCompressMetaConfig},
    transfer::account_metas::{get_transfer_instruction_account_metas, TokenAccountsMetaConfig},
    CTokenDefaultAccounts,
};
use light_compressed_token_types::constants::{
    ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID,
    PROGRAM_ID as COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::constants::REGISTERED_PROGRAM_PDA;
use solana_pubkey::Pubkey;

// TODO: Rewrite to use get_transfer_instruction_account_metas
#[test]
fn test_to_compressed_token_account_metas_compress() {
    // Create test accounts
    let fee_payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let default_pubkeys = CTokenDefaultAccounts::default();
    let reference = light_compressed_token::accounts::TransferInstruction {
        fee_payer,
        authority,
        registered_program_pda: default_pubkeys.registered_program_pda,
        noop_program: default_pubkeys.noop_program,
        account_compression_authority: default_pubkeys.account_compression_authority,
        account_compression_program: default_pubkeys.account_compression_program,
        self_program: default_pubkeys.self_program,
        cpi_authority_pda: default_pubkeys.cpi_authority_pda,
        light_system_program: default_pubkeys.light_system_program,
        token_pool_pda: None,
        compress_or_decompress_token_account: None,
        token_program: None,
        system_program: default_pubkeys.system_program,
    };

    // Test our function
    let meta_config = TokenAccountsMetaConfig::new(fee_payer, authority);
    let account_metas = get_transfer_instruction_account_metas(meta_config);
    let reference_metas = reference.to_account_metas(Some(true));

    assert_eq!(account_metas, reference_metas);
}

#[test]
fn test_to_compressed_token_account_metas_with_optional_accounts() {
    // Create test accounts
    let fee_payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    // Optional accounts
    let token_pool_pda = Pubkey::new_unique();
    let compress_or_decompress_token_account = Pubkey::new_unique();
    let spl_token_program = Pubkey::new_unique();

    let default_pubkeys = CTokenDefaultAccounts::default();
    let reference = light_compressed_token::accounts::TransferInstruction {
        fee_payer,
        authority,
        light_system_program: default_pubkeys.light_system_program,
        cpi_authority_pda: default_pubkeys.cpi_authority_pda,
        registered_program_pda: default_pubkeys.registered_program_pda,
        noop_program: default_pubkeys.noop_program,
        account_compression_authority: default_pubkeys.account_compression_authority,
        account_compression_program: default_pubkeys.account_compression_program,
        self_program: default_pubkeys.self_program,
        token_pool_pda: Some(token_pool_pda),
        compress_or_decompress_token_account: Some(compress_or_decompress_token_account),
        token_program: Some(spl_token_program),
        system_program: default_pubkeys.system_program,
    };

    let meta_config = TokenAccountsMetaConfig::compress(
        fee_payer,
        authority,
        reference.token_pool_pda.unwrap(),
        reference.compress_or_decompress_token_account.unwrap(),
        reference.token_program.unwrap(),
    );
    let account_metas = get_transfer_instruction_account_metas(meta_config);
    let reference_metas = reference.to_account_metas(Some(true));

    assert_eq!(account_metas, reference_metas);
}
#[ignore = "failing v1 tests"]
#[test]
fn test_get_batch_compress_instruction_account_metas() {
    let fee_payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_pool_pda = Pubkey::new_unique();
    let sender_token_account = Pubkey::new_unique();
    let token_program = Pubkey::new_unique();
    let merkle_tree = Pubkey::new_unique();

    let config = BatchCompressMetaConfig::new(
        fee_payer,
        authority,
        token_pool_pda,
        sender_token_account,
        token_program,
        merkle_tree,
        false,
    );
    let default_pubkeys = CTokenDefaultAccounts::default();

    let account_metas = get_batch_compress_instruction_account_metas(config);

    let reference = light_compressed_token::accounts::MintToInstruction {
        fee_payer,
        authority,
        cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
        mint: None,
        token_pool_pda,
        token_program,
        light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        registered_program_pda: Pubkey::from(REGISTERED_PROGRAM_PDA),
        noop_program: Pubkey::from(NOOP_PROGRAM_ID),
        account_compression_authority: default_pubkeys.account_compression_authority,
        account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        merkle_tree,
        self_program: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        system_program: Pubkey::default(),
        sol_pool_pda: None,
    };

    let reference_metas = reference.to_account_metas(Some(true));
    assert_eq!(account_metas, reference_metas);
}
