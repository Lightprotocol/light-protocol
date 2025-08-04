use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use spl_token_2022;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for mint action instruction
#[derive(Debug, Copy, Clone)]
pub struct MintActionMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub mint_signer: Option<Pubkey>,
    pub authority: Pubkey,
    pub tree_pubkey: Pubkey, // address tree when create_mint, input state tree when not
    pub output_queue: Pubkey,
    pub with_lamports: bool,
    pub is_decompressed: bool,
    pub with_cpi_context: bool,
    pub create_mint: bool,
    pub with_mint_signer: bool,
}

impl MintActionMetaConfig {
    /// Create a new MintActionMetaConfig for direct invocation
    pub fn new(
        fee_payer: Pubkey,
        mint_signer: Pubkey,
        authority: Pubkey,
        tree_pubkey: Pubkey,
        output_queue: Pubkey,
        with_lamports: bool,
        is_decompressed: bool,
        with_cpi_context: bool,
        create_mint: bool,
        with_mint_signer: bool,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            mint_signer: Some(mint_signer),
            authority,
            tree_pubkey,
            output_queue,
            with_lamports,
            is_decompressed,
            with_cpi_context,
            create_mint,
            with_mint_signer,
        }
    }
}

/// Get the account metas for a mint action instruction
pub fn get_mint_action_instruction_account_metas(
    config: MintActionMetaConfig,
    compressed_mint_inputs: &light_ctoken_types::instructions::create_compressed_mint::CompressedMintWithContext,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut metas = Vec::new();

    // Static accounts (before CPI accounts offset)
    // light_system_program (always required)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // mint_signer (conditional) - matches onchain logic: with_mint_signer = create_mint() | has_CreateSplMint_action
    if config.with_mint_signer {
        if let Some(mint_signer) = config.mint_signer {
            metas.push(AccountMeta::new_readonly(mint_signer, true));
        }
    }

    // authority (signer)
    metas.push(AccountMeta::new_readonly(config.authority, true));

    // For decompressed mints, add SPL mint and token program accounts
    // These need to come right after authority to match processor expectations
    if config.is_decompressed {
        // mint - either derived from mint_signer (for creation) or from existing mint data
        if let Some(mint_signer) = config.mint_signer {
            // For mint creation - derive from mint_signer
            let (spl_mint_pda, _) = crate::instructions::find_spl_mint_address(&mint_signer);
            metas.push(AccountMeta::new(spl_mint_pda, false)); // mutable: true, signer: false
            
            // token_pool_pda (derived from mint)
            let (token_pool_pda, _) = crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pda, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        } else {
            // For existing mint operations - use the spl_mint from compressed mint inputs
            let spl_mint_pubkey = solana_pubkey::Pubkey::from(compressed_mint_inputs.mint.spl_mint.to_bytes());
            metas.push(AccountMeta::new(spl_mint_pubkey, false)); // mutable: true, signer: false
            
            // token_pool_pda (derived from the spl_mint)
            let (token_pool_pda, _) = crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pubkey, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        }

        // token_program (use spl_token_2022 program ID)
        metas.push(AccountMeta::new_readonly(
            spl_token_2022::ID,
            false,
        ));
    }

    // LightSystemAccounts in exact order expected by validate_and_parse:
    
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

    // sol_pool_pda (optional for lamports operations)
    if config.with_lamports {
        metas.push(AccountMeta::new(
            Pubkey::new_from_array(light_sdk::constants::SOL_POOL_PDA),
            false,
        ));
    }

    // sol_decompression_recipient (optional - not used in mint_action, but needed for account order)
    // Skip this as decompress_sol is false in mint_action

    // cpi_context (optional)
    if config.with_cpi_context {
        // CPI context account would be added here
        // For now, we'll handle this in the client layer
    }

    // After LightSystemAccounts, add the remaining accounts:
    
    // out_output_queue (mutable)
    metas.push(AccountMeta::new(config.output_queue, false));

    // Add address tree only if creating a new mint (for address creation)
    if config.create_mint {
        metas.push(AccountMeta::new(config.tree_pubkey, false));
    }

    // in_merkle_tree (optional if is_decompressed) - the state tree containing the existing compressed mint
    if config.is_decompressed && !config.create_mint {
        // For existing mints, we need the state merkle tree where the compressed mint is stored
        metas.push(AccountMeta::new(config.tree_pubkey, false));
    }

    // in_output_queue (optional if is_decompressed)
    if config.is_decompressed {
        metas.push(AccountMeta::new(config.output_queue, false));
    }

    // tokens_out_queue (optional if is_decompressed)
    if config.is_decompressed {
        metas.push(AccountMeta::new(config.output_queue, false));
    }

    metas
}

/// Account metadata configuration for mint action CPI write instruction
#[derive(Debug, Copy, Clone)]
pub struct MintActionMetaConfigCpiWrite {
    pub fee_payer: Pubkey,
    pub mint_signer: Option<Pubkey>, // Optional - only when creating mint and when creating SPL mint
    pub authority: Pubkey,
    pub cpi_context: Pubkey,
}

/// Get the account metas for a mint action CPI write instruction
pub fn get_mint_action_instruction_account_metas_cpi_write(
    config: MintActionMetaConfigCpiWrite,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut metas = Vec::new();

    // The order must match mint_action on-chain program expectations:
    // [light_system_program, mint_signer, authority, fee_payer, cpi_authority_pda, cpi_context]

    // light_system_program (always required) - index 0
    metas.push(AccountMeta::new_readonly(default_pubkeys.light_system_program, false));

    // mint_signer (optional signer - only when creating mint and creating SPL mint) - index 1
    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(mint_signer, true));
    }

    // authority (signer) - index 2
    metas.push(AccountMeta::new_readonly(config.authority, true));

    // fee_payer (signer, mutable) - index 3 (this is what the program checks for)
    metas.push(AccountMeta::new(config.fee_payer, true));

    // cpi_authority_pda - index 4
    metas.push(AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false));

    // cpi_context (mutable) - index 5
    metas.push(AccountMeta::new(config.cpi_context, false));

    metas
}