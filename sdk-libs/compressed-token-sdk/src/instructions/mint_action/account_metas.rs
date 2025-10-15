use light_program_profiler::profile;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use spl_token_2022;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for mint action instruction
#[derive(Debug, Clone, Default)]
pub struct MintActionMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub mint_signer: Option<Pubkey>,
    pub authority: Pubkey,
    pub tree_pubkey: Pubkey, // address tree when create_mint, input state tree when not
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub output_queue: Pubkey,
    pub tokens_out_queue: Option<Pubkey>, // Output queue for new token accounts
    pub with_lamports: bool,
    pub spl_mint_initialized: bool,
    pub has_mint_to_actions: bool, // Whether we have MintTo actions
    pub with_cpi_context: Option<Pubkey>,
    pub create_mint: bool,
    pub with_mint_signer: bool,
    pub mint_needs_to_sign: bool, // Only true when creating new compressed mint
    pub ctoken_accounts: Vec<Pubkey>, // For mint_to_ctoken actions
}

/// Get the account metas for a mint action instruction
#[profile]
pub fn get_mint_action_instruction_account_metas(
    config: MintActionMetaConfig,
    compressed_mint_inputs: &light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
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
            metas.push(AccountMeta::new_readonly(
                mint_signer,
                config.mint_needs_to_sign,
            ));
        }
    }

    // authority (always signer as per program requirement)
    metas.push(AccountMeta::new_readonly(config.authority, true));

    // For decompressed mints, add SPL mint and token program accounts
    // These need to come right after authority to match processor expectations
    if config.spl_mint_initialized {
        // mint - either derived from mint_signer (for creation) or from existing mint data
        if let Some(mint_signer) = config.mint_signer {
            // For mint creation - derive from mint_signer
            let (spl_mint_pda, _) = crate::instructions::find_spl_mint_address(&mint_signer);
            metas.push(AccountMeta::new(spl_mint_pda, false)); // mutable: true, signer: false

            // token_pool_pda (derived from mint)
            let (token_pool_pda, _) =
                crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pda, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        } else {
            // For existing mint operations - use the mint from compressed mint inputs
            let spl_mint_pubkey =
                solana_pubkey::Pubkey::from(compressed_mint_inputs.mint.metadata.mint.to_bytes());
            metas.push(AccountMeta::new(spl_mint_pubkey, false)); // mutable: true, signer: false

            // token_pool_pda (derived from the mint)
            let (token_pool_pda, _) =
                crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pubkey, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        }

        // token_program (use spl_token_2022 program ID)
        metas.push(AccountMeta::new_readonly(spl_token_2022::ID, false));
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
    if let Some(cpi_context) = config.with_cpi_context {
        metas.push(AccountMeta::new(cpi_context, false));
    }

    // After LightSystemAccounts, add the remaining accounts to match onchain expectations:

    // out_output_queue (mutable) - always required
    metas.push(AccountMeta::new(config.output_queue, false));

    // in_merkle_tree (always required)
    // When create_mint=true: this is the address tree for creating new mint addresses
    // When create_mint=false: this is the state tree containing the existing compressed mint
    metas.push(AccountMeta::new(config.tree_pubkey, false));

    // in_output_queue - only when NOT creating mint
    if !config.create_mint {
        if let Some(input_queue) = config.input_queue {
            metas.push(AccountMeta::new(input_queue, false));
        }
    }

    // tokens_out_queue - only when we have MintTo actions
    if config.has_mint_to_actions {
        let tokens_out_queue = config.tokens_out_queue.unwrap_or(config.output_queue);
        metas.push(AccountMeta::new(tokens_out_queue, false));
    }

    // Add decompressed token accounts as remaining accounts for MintToCToken actions
    for token_account in &config.ctoken_accounts {
        metas.push(AccountMeta::new(*token_account, false));
    }

    metas
}

/// Account metadata configuration for mint action CPI write instruction
#[derive(Debug, Clone)]
pub struct MintActionMetaConfigCpiWrite {
    pub fee_payer: Pubkey,
    pub mint_signer: Option<Pubkey>, // Optional - only when creating mint and when creating SPL mint
    pub authority: Pubkey,
    pub cpi_context: Pubkey,
    pub mint_needs_to_sign: bool, // Only true when creating new compressed mint
}

/// Get the account metas for a mint action CPI write instruction
#[profile]
pub fn get_mint_action_instruction_account_metas_cpi_write(
    config: MintActionMetaConfigCpiWrite,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut metas = Vec::new();

    // The order must match mint_action on-chain program expectations:
    // [light_system_program, mint_signer, authority, fee_payer, cpi_authority_pda, cpi_context]

    // light_system_program (always required) - index 0
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    // mint_signer (optional signer - only when creating mint and creating SPL mint) - index 1
    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(
            mint_signer,
            config.mint_needs_to_sign,
        ));
    }

    // authority (signer) - index 2
    metas.push(AccountMeta::new_readonly(config.authority, true));

    // fee_payer (signer, mutable) - index 3 (this is what the program checks for)
    metas.push(AccountMeta::new(config.fee_payer, true));

    // cpi_authority_pda - index 4
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    ));

    // cpi_context (mutable) - index 5
    metas.push(AccountMeta::new(config.cpi_context, false));

    metas
}
