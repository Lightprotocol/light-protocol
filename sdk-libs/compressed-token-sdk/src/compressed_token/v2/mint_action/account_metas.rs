use light_program_profiler::profile;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::utils::CTokenDefaultAccounts;

#[derive(Debug, Clone)]
pub struct MintActionMetaConfig {
    pub fee_payer: Pubkey,
    pub mint_signer: Option<Pubkey>,
    pub authority: Pubkey,
    pub tree_pubkey: Pubkey, // address tree when create_mint, input state tree when not
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub output_queue: Pubkey,
    pub tokens_out_queue: Option<Pubkey>, // Output queue for new token accounts
    pub with_lamports: bool,
    pub spl_mint_initialized: bool,
    pub has_mint_to_compressed_actions: bool, // Whether we have MintToCompressed actions (not MintToCToken)
    pub with_cpi_context: Option<Pubkey>,
    pub create_mint: bool,
    pub with_mint_signer: bool,
    pub mint_needs_to_sign: bool, // Only true when creating new compressed mint
    pub ctoken_accounts: Vec<Pubkey>, // For mint_to_ctoken actions
}

impl MintActionMetaConfig {
    pub fn new_create_mint(
        instruction_data: &light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData,
        authority: Pubkey,
        mint_signer: Pubkey,
        fee_payer: Pubkey,
        address_tree: Pubkey,
        output_queue: Pubkey,
    ) -> crate::error::Result<Self> {
        if instruction_data.create_mint.is_none() {
            return Err(crate::error::TokenSdkError::CreateMintDataRequired);
        }

        let (has_mint_to_compressed_actions, ctoken_accounts) =
            Self::analyze_actions(&instruction_data.actions);
        let spl_mint_initialized = instruction_data.mint.metadata.spl_mint_initialized;

        Ok(Self {
            fee_payer,
            mint_signer: Some(mint_signer),
            authority,
            tree_pubkey: address_tree,
            input_queue: None,
            output_queue,
            tokens_out_queue: if has_mint_to_compressed_actions {
                Some(output_queue)
            } else {
                None
            },
            with_lamports: false,
            spl_mint_initialized,
            has_mint_to_compressed_actions,
            with_cpi_context: None,
            create_mint: true,
            with_mint_signer: true,
            mint_needs_to_sign: true,
            ctoken_accounts,
        })
    }

    pub fn new(
        instruction_data: &light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData,
        authority: Pubkey,
        fee_payer: Pubkey,
        state_tree: Pubkey,
        input_queue: Pubkey,
        output_queue: Pubkey,
    ) -> crate::error::Result<Self> {
        if instruction_data.create_mint.is_some() {
            return Err(crate::error::TokenSdkError::CreateMintMustBeNone);
        }

        let (has_mint_to_compressed_actions, ctoken_accounts) =
            Self::analyze_actions(&instruction_data.actions);

        Ok(Self {
            fee_payer,
            mint_signer: None,
            authority,
            tree_pubkey: state_tree,
            input_queue: Some(input_queue),
            output_queue,
            tokens_out_queue: if has_mint_to_compressed_actions {
                Some(output_queue)
            } else {
                None
            },
            with_lamports: false,
            spl_mint_initialized: false,
            has_mint_to_compressed_actions,
            with_cpi_context: None,
            create_mint: false,
            with_mint_signer: false,
            mint_needs_to_sign: false,
            ctoken_accounts,
        })
    }

    pub fn new_cpi_context(
        instruction_data: &light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData,
        authority: Pubkey,
        fee_payer: Pubkey,
        cpi_context_pubkey: Pubkey,
    ) -> crate::error::Result<Self> {
        if instruction_data.cpi_context.is_none() {
            return Err(crate::error::TokenSdkError::CpiContextRequired);
        }

        let (has_mint_to_compressed_actions, ctoken_accounts) =
            Self::analyze_actions(&instruction_data.actions);
        let spl_mint_initialized = instruction_data.mint.metadata.spl_mint_initialized;
        let create_mint = instruction_data.create_mint.is_some();

        Ok(Self {
            fee_payer,
            mint_signer: None,
            authority,
            tree_pubkey: Pubkey::default(),
            input_queue: None,
            output_queue: Pubkey::default(),
            tokens_out_queue: None,
            with_lamports: false,
            spl_mint_initialized,
            has_mint_to_compressed_actions,
            with_cpi_context: Some(cpi_context_pubkey),
            create_mint,
            with_mint_signer: create_mint,
            mint_needs_to_sign: create_mint,
            ctoken_accounts,
        })
    }

    pub fn with_tokens_out_queue(mut self, queue: Pubkey) -> Self {
        self.tokens_out_queue = Some(queue);
        self
    }

    pub fn with_ctoken_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.ctoken_accounts = accounts;
        self
    }

    fn analyze_actions(
        actions: &[light_ctoken_types::instructions::mint_action::Action],
    ) -> (bool, Vec<Pubkey>) {
        let mut has_mint_to_compressed_actions = false;
        let ctoken_accounts = Vec::new();

        for action in actions {
            match action {
                light_ctoken_types::instructions::mint_action::Action::MintToCompressed(_) => {
                    has_mint_to_compressed_actions = true;
                }
                light_ctoken_types::instructions::mint_action::Action::MintToCToken(_) => {
                    // MintToCToken doesn't need tokens_out_queue - it mints to existing decompressed accounts
                }
                _ => {}
            }
        }

        (has_mint_to_compressed_actions, ctoken_accounts)
    }
}

/// Get the account metas for a mint action instruction
#[profile]
pub fn get_mint_action_instruction_account_metas(
    config: MintActionMetaConfig,
    compressed_mint_inputs: &light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut metas = Vec::new();

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    if config.with_mint_signer {
        if let Some(mint_signer) = config.mint_signer {
            metas.push(AccountMeta::new_readonly(
                mint_signer,
                config.mint_needs_to_sign,
            ));
        }
    }

    metas.push(AccountMeta::new_readonly(config.authority, true));

    if config.spl_mint_initialized {
        if let Some(mint_signer) = config.mint_signer {
            let (spl_mint_pda, _) =
                crate::compressed_token::create_compressed_mint::find_spl_mint_address(
                    &mint_signer,
                );
            metas.push(AccountMeta::new(spl_mint_pda, false));

            let (token_pool_pda, _) =
                crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pda, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        } else {
            let spl_mint_pubkey =
                solana_pubkey::Pubkey::from(compressed_mint_inputs.mint.metadata.mint.to_bytes());
            metas.push(AccountMeta::new(spl_mint_pubkey, false));

            let (token_pool_pda, _) =
                crate::token_pool::find_token_pool_pda_with_index(&spl_mint_pubkey, 0);
            metas.push(AccountMeta::new(token_pool_pda, false));
        }

        metas.push(AccountMeta::new_readonly(spl_token_2022::ID, false));
    }

    metas.push(AccountMeta::new(config.fee_payer, true));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    ));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.registered_program_pda,
        false,
    ));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_authority,
        false,
    ));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.account_compression_program,
        false,
    ));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    if config.with_lamports {
        metas.push(AccountMeta::new(
            Pubkey::new_from_array(light_sdk::constants::SOL_POOL_PDA),
            false,
        ));
    }

    if let Some(cpi_context) = config.with_cpi_context {
        metas.push(AccountMeta::new(cpi_context, false));
    }

    metas.push(AccountMeta::new(config.output_queue, false));

    metas.push(AccountMeta::new(config.tree_pubkey, false));

    if !config.create_mint {
        if let Some(input_queue) = config.input_queue {
            metas.push(AccountMeta::new(input_queue, false));
        }
    }

    if config.has_mint_to_compressed_actions {
        let tokens_out_queue = config.tokens_out_queue.unwrap_or(config.output_queue);
        metas.push(AccountMeta::new(tokens_out_queue, false));
    }

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

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.light_system_program,
        false,
    ));

    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(
            mint_signer,
            config.mint_needs_to_sign,
        ));
    }

    metas.push(AccountMeta::new_readonly(config.authority, true));

    metas.push(AccountMeta::new(config.fee_payer, true));

    metas.push(AccountMeta::new_readonly(
        default_pubkeys.cpi_authority_pda,
        false,
    ));

    metas.push(AccountMeta::new(config.cpi_context, false));

    metas
}
