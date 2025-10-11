use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for create compressed mint instruction
#[derive(Debug, Copy, Clone)]
pub struct CreateCompressedMintMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub mint_signer: Option<Pubkey>,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
}

impl CreateCompressedMintMetaConfig {
    /// Create a new CreateCompressedMintMetaConfig for direct invocation
    pub fn new(
        fee_payer: Pubkey,
        mint_signer: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            mint_signer: Some(mint_signer),
            address_tree_pubkey,
            output_queue,
        }
    }

    /// Create a new CreateCompressedMintMetaConfig for client-side (CPI) usage
    pub fn new_client(
        mint_seed: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            mint_signer: Some(mint_seed),
            address_tree_pubkey,
            output_queue,
        }
    }
}

/// Get the standard account metas for a create compressed mint instruction
pub fn get_create_compressed_mint_instruction_account_metas(
    config: CreateCompressedMintMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    // Calculate capacity based on configuration
    // Static accounts: mint_signer + light_system_program (2)
    // LightSystemAccounts: fee_payer + cpi_authority_pda + registered_program_pda +
    //                      account_compression_authority + account_compression_program + system_program (6)
    // Tree accounts: address_merkle_tree + output_queue (2)
    let base_capacity = 9; // 2 static + 5 LightSystemAccounts (excluding fee_payer since it's counted separately) + 2 tree

    // Optional fee_payer account
    let fee_payer_capacity = if config.fee_payer.is_some() { 1 } else { 0 };

    let total_capacity = base_capacity + fee_payer_capacity;

    let mut metas = Vec::with_capacity(total_capacity);

    // First two accounts are static non-CPI accounts as expected by CPI_ACCOUNTS_OFFSET = 2
    // mint_signer (always required)
    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(mint_signer, true));
    }

    // light_system_program (always required)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // CPI accounts start here (matching system program expectations)
    // fee_payer (signer, mutable) - only add if provided
    if let Some(fee_payer) = config.fee_payer {
        metas.push(AccountMeta::new(fee_payer, true));
    }

    // cpi_authority_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    ));

    // registered_program_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.registered_program_pda,
        false,
    ));

    // account_compression_authority
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_authority,
        false,
    ));

    // account_compression_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_program,
        false,
    ));

    // system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    // Tree accounts (mutable) - these are parsed by CreateCompressedAccountTreeAccounts
    // address_merkle_tree (mutable)
    metas.push(AccountMeta::new(config.address_tree_pubkey, false));

    // output_queue (mutable)
    metas.push(AccountMeta::new(config.output_queue, false));

    metas
}

#[derive(Debug, Copy, Clone)]
pub struct CreateCompressedMintMetaConfigCpiWrite {
    pub fee_payer: Pubkey,
    pub mint_signer: Pubkey,
    pub cpi_context: Pubkey,
}
pub fn get_create_compressed_mint_instruction_account_metas_cpi_write(
    config: CreateCompressedMintMetaConfigCpiWrite,
) -> [AccountMeta; 5] {
    let default_pubkeys = CTokenDefaultAccounts::default();
    [
        AccountMeta::new_readonly(config.mint_signer, true),
        AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
        AccountMeta::new(config.fee_payer, true),
        AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
        AccountMeta::new(config.cpi_context, false),
    ]
}
