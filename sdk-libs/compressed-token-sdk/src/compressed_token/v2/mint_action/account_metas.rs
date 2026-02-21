use light_program_profiler::profile;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::utils::TokenDefaultAccounts;

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
    pub token_accounts: Vec<Pubkey>, // For mint_to_ctoken actions
    pub mint: Option<Pubkey>,        // Mint PDA account for DecompressMint action
    pub compressible_config: Option<Pubkey>, // CompressibleConfig account (when creating Mint)
    pub rent_sponsor: Option<Pubkey>, // Rent sponsor PDA (when creating Mint)
    pub mint_signer_must_sign: bool, // true for create_mint, false for decompress_mint
}

impl MintActionMetaConfig {
    /// Create a new MintActionMetaConfig for creating a new compressed mint.
    /// `rent_sponsor` is required because mint creation charges a creation fee
    /// transferred to the rent sponsor PDA.
    pub fn new_create_mint(
        fee_payer: Pubkey,
        authority: Pubkey,
        mint_signer: Pubkey,
        address_tree: Pubkey,
        output_queue: Pubkey,
        rent_sponsor: Pubkey,
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
            token_accounts: Vec::new(),
            mint: None,
            compressible_config: None,
            rent_sponsor: Some(rent_sponsor),
            mint_signer_must_sign: true,
        }
    }

    /// Create a new MintActionMetaConfig for operations on an existing compressed mint.
    #[inline(never)]
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
            token_accounts: Vec::new(),
            mint: None,
            compressible_config: None,
            rent_sponsor: None,
            mint_signer_must_sign: false,
        }
    }

    /// Create a new MintActionMetaConfig for CPI context operations.
    pub fn new_cpi_context(
        instruction_data: &light_token_interface::instructions::mint_action::MintActionCompressedInstructionData,
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
            token_accounts: Vec::new(),
            mint: None,
            compressible_config: None,
            rent_sponsor: None,
            mint_signer_must_sign: false,
        })
    }

    pub fn with_mint_compressed_tokens(mut self) -> Self {
        self.tokens_out_queue = Some(self.output_queue);
        self
    }

    pub fn with_token_accounts(mut self, accounts: Vec<Pubkey>) -> Self {
        self.token_accounts = accounts;
        self
    }

    pub fn with_mint(mut self, mint: Pubkey) -> Self {
        self.mint = Some(mint);
        self
    }

    /// Set the mint_signer account with signing required.
    /// Use for create_mint actions.
    pub fn with_mint_signer(mut self, mint_signer: Pubkey) -> Self {
        self.mint_signer = Some(mint_signer);
        self.mint_signer_must_sign = true;
        self
    }

    /// Set only the rent_sponsor account (without compressible_config or mint/cmint).
    /// Required for create_mint operations to receive the mint creation fee.
    pub fn with_rent_sponsor(mut self, rent_sponsor: Pubkey) -> Self {
        self.rent_sponsor = Some(rent_sponsor);
        self
    }

    /// Configure compressible Mint with config and rent sponsor.
    /// Mint is always compressible - this sets all required accounts.
    pub fn with_compressible_mint(
        mut self,
        mint: Pubkey,
        compressible_config: Pubkey,
        rent_sponsor: Pubkey,
    ) -> Self {
        self.mint = Some(mint);
        self.compressible_config = Some(compressible_config);
        self.rent_sponsor = Some(rent_sponsor);
        self
    }

    /// Get the account metas for a mint action instruction
    #[profile]
    #[inline(never)]
    pub fn to_account_metas(self) -> Vec<AccountMeta> {
        let default_pubkeys = TokenDefaultAccounts::default();
        let mut metas = Vec::new();

        metas.push(AccountMeta::new_readonly(
            default_pubkeys.light_system_program,
            false,
        ));

        // mint_signer is present when creating a new mint or decompressing
        if let Some(mint_signer) = self.mint_signer {
            // mint_signer needs to sign for create_mint, not for decompress_mint
            metas.push(AccountMeta::new_readonly(
                mint_signer,
                self.mint_signer_must_sign,
            ));
        }

        metas.push(AccountMeta::new_readonly(self.authority, true));

        // CompressibleConfig account (when creating compressible Mint)
        if let Some(config) = self.compressible_config {
            metas.push(AccountMeta::new_readonly(config, false));
        }

        // Mint account is present when decompressing the mint (DecompressMint action) or syncing
        if let Some(mint) = self.mint {
            metas.push(AccountMeta::new(mint, false));
        }

        // Rent sponsor PDA (when creating compressible Mint)
        if let Some(rent_sponsor) = self.rent_sponsor {
            metas.push(AccountMeta::new(rent_sponsor, false));
        }

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

        // input_queue is present when operating on an existing compressed mint
        // (input_queue is set via new() for existing mints, None via new_create_mint() for new mints)
        if let Some(input_queue) = self.input_queue {
            metas.push(AccountMeta::new(input_queue, false));
        }

        // tokens_out_queue is present when there are MintToCompressed actions
        if let Some(tokens_out_queue) = self.tokens_out_queue {
            metas.push(AccountMeta::new(tokens_out_queue, false));
        }

        for token_account in &self.token_accounts {
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
    let default_pubkeys = TokenDefaultAccounts::default();
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
