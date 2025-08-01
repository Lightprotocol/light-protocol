use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for mint_to_compressed instruction
#[derive(Debug, Copy, Clone)]
pub struct MintToCompressedMetaConfig {
    pub mint_authority: Option<Pubkey>,
    pub payer: Option<Pubkey>,
    pub state_merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub state_tree_pubkey: Pubkey,
    pub compressed_mint_tree: Pubkey,
    pub compressed_mint_queue: Pubkey,
    pub is_decompressed: bool,
    pub mint_pda: Option<Pubkey>,
    pub token_pool_pda: Option<Pubkey>,
    pub token_program: Option<Pubkey>,
    pub with_lamports: bool,
}

impl MintToCompressedMetaConfig {
    /// Create a new MintToCompressedMetaConfig for standard compressed mint operations
    pub fn new(
        mint_authority: Pubkey,
        payer: Pubkey,
        state_merkle_tree: Pubkey,
        output_queue: Pubkey,
        state_tree_pubkey: Pubkey,
        compressed_mint_tree: Pubkey,
        compressed_mint_queue: Pubkey,
        with_lamports: bool,
    ) -> Self {
        Self {
            mint_authority: Some(mint_authority),
            payer: Some(payer),
            state_merkle_tree,
            output_queue,
            state_tree_pubkey,
            compressed_mint_tree,
            compressed_mint_queue,
            is_decompressed: false,
            mint_pda: None,
            token_pool_pda: None,
            token_program: None,
            with_lamports,
        }
    }

    /// Create a new MintToCompressedMetaConfig for client use (excludes authority and payer accounts)
    pub fn new_client(
        state_merkle_tree: Pubkey,
        output_queue: Pubkey,
        state_tree_pubkey: Pubkey,
        compressed_mint_tree: Pubkey,
        compressed_mint_queue: Pubkey,
        with_lamports: bool,
    ) -> Self {
        Self {
            mint_authority: None, // Client mode - account provided by caller
            payer: None,          // Client mode - account provided by caller
            state_merkle_tree,
            output_queue,
            state_tree_pubkey,
            compressed_mint_tree,
            compressed_mint_queue,
            is_decompressed: false,
            mint_pda: None,
            token_pool_pda: None,
            token_program: None,
            with_lamports,
        }
    }

    /// Create a new MintToCompressedMetaConfig for decompressed mint operations
    pub fn new_decompressed(
        mint_authority: Pubkey,
        payer: Pubkey,
        state_merkle_tree: Pubkey,
        output_queue: Pubkey,
        state_tree_pubkey: Pubkey,
        compressed_mint_tree: Pubkey,
        compressed_mint_queue: Pubkey,
        mint_pda: Pubkey,
        token_pool_pda: Pubkey,
        token_program: Pubkey,
        with_lamports: bool,
    ) -> Self {
        Self {
            mint_authority: Some(mint_authority),
            payer: Some(payer),
            state_merkle_tree,
            output_queue,
            state_tree_pubkey,
            compressed_mint_tree,
            compressed_mint_queue,
            is_decompressed: true,
            mint_pda: Some(mint_pda),
            token_pool_pda: Some(token_pool_pda),
            token_program: Some(token_program),
            with_lamports,
        }
    }
}

/// Get the standard account metas for a mint_to_compressed instruction
pub fn get_mint_to_compressed_instruction_account_metas(
    config: MintToCompressedMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();

    // Calculate capacity based on configuration
    // Optional accounts: authority + payer + optional decompressed accounts (3) + light_system_program +
    //                   cpi accounts (6 without fee_payer) + optional SOL pool + system_program + merkle tree accounts (5)
    let base_capacity = 14; // light_system_program + 6 cpi accounts + system_program + 5 tree accounts
    let authority_capacity = if config.mint_authority.is_some() { 1 } else { 0 };
    let payer_capacity = if config.payer.is_some() { 1 } else { 0 };
    let decompressed_capacity = if config.is_decompressed { 3 } else { 0 };
    let sol_pool_capacity = if config.with_lamports { 1 } else { 0 };
    let total_capacity = base_capacity + authority_capacity + payer_capacity + decompressed_capacity + sol_pool_capacity;

    let mut metas = Vec::with_capacity(total_capacity);

    // authority (signer) - only add if provided
    if let Some(mint_authority) = config.mint_authority {
        metas.push(AccountMeta::new_readonly(mint_authority, true));
    }

    // Optional decompressed mint accounts
    if config.is_decompressed {
        metas.push(AccountMeta::new(config.mint_pda.unwrap(), false)); // mint
        metas.push(AccountMeta::new(config.token_pool_pda.unwrap(), false)); // token_pool_pda
        metas.push(AccountMeta::new_readonly(
            config.token_program.unwrap(),
            false,
        )); // token_program
    }

    // light_system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // CPI accounts in exact order expected by InvokeCpiWithReadOnly
    if let Some(payer) = config.payer {
        metas.push(AccountMeta::new(payer, true)); // fee_payer (signer, mutable)
    }
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    )); // cpi_authority_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.registered_program_pda,
        false,
    )); // registered_program_pda
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.noop_program,
        false,
    )); // noop_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_authority,
        false,
    )); // account_compression_authority
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_program,
        false,
    )); // account_compression_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.self_program,
        false,
    )); // self_program

    // Optional SOL pool
    if config.with_lamports {
        metas.push(AccountMeta::new(
            Pubkey::from(light_sdk::constants::SOL_POOL_PDA),
            false,
        )); // sol_pool_pda (mutable)
    }

    // system_program
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    // Merkle tree accounts - UpdateOneCompressedAccountTreeAccounts (3 accounts)
    metas.push(AccountMeta::new(config.state_merkle_tree, false)); // in_merkle_tree (mutable)
    metas.push(AccountMeta::new(config.compressed_mint_queue, false)); // in_output_queue (mutable)
    metas.push(AccountMeta::new(config.compressed_mint_queue, false)); // out_output_queue (mutable) - same as in_output_queue
    
    // Additional tokens_out_queue (separate from UpdateOneCompressedAccountTreeAccounts)
    metas.push(AccountMeta::new(config.output_queue, false)); // tokens_out_queue (mutable)

    // Compressed mint's address tree
    metas.push(AccountMeta::new(config.compressed_mint_tree, false));

    metas
}
