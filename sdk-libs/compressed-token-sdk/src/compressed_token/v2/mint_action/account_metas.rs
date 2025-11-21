use light_program_profiler::profile;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::utils::CTokenDefaultAccounts;

#[derive(Debug, Clone)]
pub struct MintActionMetaConfig {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub tree_pubkey: Pubkey, // address tree when create_mint, input state tree when not
    pub output_queue: Pubkey,
    pub mint_signer: Option<Pubkey>,
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub tokens_out_queue: Option<Pubkey>, // Output queue for new token accounts
    pub cpi_context: Option<Pubkey>,
    pub ctoken_accounts: Vec<Pubkey>, // For mint_to_ctoken actions
}

impl MintActionMetaConfig {
    /// Create a new MintActionMetaConfig for creating a new compressed mint.
    pub fn new_create_mint(
        fee_payer: Pubkey,
        authority: Pubkey,
        mint_signer: Pubkey,
        address_tree: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            tree_pubkey: address_tree,
            output_queue,
            mint_signer: Some(mint_signer),
            input_queue: None,
            tokens_out_queue: None,
            cpi_context: None,
            ctoken_accounts: Vec::new(),
        }
    }

    /// Create a new MintActionMetaConfig for operations on an existing compressed mint.
    pub fn new(
        fee_payer: Pubkey,
        authority: Pubkey,
        state_tree: Pubkey,
        input_queue: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            tree_pubkey: state_tree,
            output_queue,
            mint_signer: None,
            input_queue: Some(input_queue),
            tokens_out_queue: None,
            cpi_context: None,
            ctoken_accounts: Vec::new(),
        }
    }

    /// Create a new MintActionMetaConfig for CPI context operations.
    pub fn new_cpi_context(
        instruction_data: &light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData,
        fee_payer: Pubkey,
        authority: Pubkey,
        cpi_context_pubkey: Pubkey,
    ) -> crate::error::Result<Self> {
        if instruction_data.cpi_context.is_none() {
            return Err(crate::error::TokenSdkError::CpiContextRequired);
        }

        Ok(Self {
            fee_payer,
            authority,
            tree_pubkey: Pubkey::default(),
            output_queue: Pubkey::default(),
            mint_signer: None,
            input_queue: None,
            tokens_out_queue: None,
            cpi_context: Some(cpi_context_pubkey),
            ctoken_accounts: Vec::new(),
        })
    }

    pub fn with_mint_compressed_tokens(mut self) -> Self {
        self.tokens_out_queue = Some(self.output_queue);
        self
    }

    pub fn with_ctoken_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.ctoken_accounts = accounts;
        self
    }

    /// Get the account metas for a mint action instruction
    #[profile]
    pub fn to_account_metas(self) -> Vec<AccountMeta> {
        let default_pubkeys = CTokenDefaultAccounts::default();
        let mut metas = Vec::new();

        metas.push(AccountMeta::new_readonly(
            default_pubkeys.light_system_program,
            false,
        ));

        // mint_signer is present when creating a new mint
        if let Some(mint_signer) = self.mint_signer {
            // mint signer always needs to sign when present
            metas.push(AccountMeta::new_readonly(mint_signer, true));
        }

        metas.push(AccountMeta::new_readonly(self.authority, true));

        metas.push(AccountMeta::new(self.fee_payer, true));

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

        if let Some(cpi_context) = self.cpi_context {
            metas.push(AccountMeta::new(cpi_context, false));
        }

        metas.push(AccountMeta::new(self.output_queue, false));

        metas.push(AccountMeta::new(self.tree_pubkey, false));

        // input_queue is present when NOT creating a new mint (mint_signer.is_none())
        if self.mint_signer.is_none() {
            if let Some(input_queue) = self.input_queue {
                metas.push(AccountMeta::new(input_queue, false));
            }
        }

        // tokens_out_queue is present when there are MintToCompressed actions
        if let Some(tokens_out_queue) = self.tokens_out_queue {
            metas.push(AccountMeta::new(tokens_out_queue, false));
        }

        for token_account in &self.ctoken_accounts {
            metas.push(AccountMeta::new(*token_account, false));
        }

        metas
    }
}
/// Account metadata configuration for mint action CPI write instruction
#[derive(Debug, Clone)]
pub struct MintActionMetaConfigCpiWrite {
    pub fee_payer: Pubkey,
    pub mint_signer: Option<Pubkey>, // Optional - only when creating mint
    pub authority: Pubkey,
    pub cpi_context: Pubkey,
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

    // mint signer always needs to sign when present
    if let Some(mint_signer) = config.mint_signer {
        metas.push(AccountMeta::new_readonly(mint_signer, true));
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
