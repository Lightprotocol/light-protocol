use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CompressedMintInstructionData, CpiContext},
    },
    COMPRESSED_MINT_SEED,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{compressed_token::mint_action::MintActionMetaConfig, token::SystemAccountInfos};
// TODO: modify so that it creates a decompressed mint, if you want a compressed mint use light_token_sdk::compressed_token::create_cmint
/// Parameters for creating a compressed mint.
#[derive(Debug, Clone)]
pub struct CreateMintParams {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub bump: u8,
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

/// # Create a compressed mint instruction:
/// ```rust,no_run
/// # use solana_pubkey::Pubkey;
/// use light_token_sdk::token::{
///     CreateMint, CreateMintParams, derive_mint_compressed_address, find_mint_address,
/// };
/// # use light_token_sdk::CompressedProof;
/// # let mint_seed_pubkey = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let address_tree = Pubkey::new_unique();
/// # let output_queue = Pubkey::new_unique();
/// # let mint_authority = Pubkey::new_unique();
/// # let address_merkle_tree_root_index: u16 = 0;
/// # let proof: CompressedProof = todo!();
///
/// // Derive addresses
/// let compression_address = derive_mint_compressed_address(&mint_seed_pubkey, &address_tree);
/// let mint = find_mint_address(&mint_seed_pubkey).0;
///
/// let params = CreateMintParams {
///     decimals: 9,
///     address_merkle_tree_root_index, // from rpc.get_validity_proof
///     mint_authority,
///     proof, // from rpc.get_validity_proof
///     compression_address,
///     mint,
///     freeze_authority: None,
///     extensions: None,
/// };
/// let instruction = CreateMint::new(
///     params,
///     mint_seed_pubkey,
///     payer,
///     address_tree,
///     output_queue,
/// ).instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CreateMint {
    /// Used as seed for the mint address.
    /// The mint seed account must be a signer.
    pub mint_seed_pubkey: Pubkey,
    pub payer: Pubkey,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub params: CreateMintParams,
}

impl CreateMint {
    pub fn new(
        params: CreateMintParams,
        mint_seed_pubkey: Pubkey,
        payer: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
    ) -> Self {
        Self {
            mint_seed_pubkey,
            payer,
            address_tree_pubkey,
            output_queue,
            cpi_context: None,
            cpi_context_pubkey: None,
            params,
        }
    }

    pub fn with_cpi_context(mut self, cpi_context: CpiContext, cpi_context_pubkey: Pubkey) -> Self {
        self.cpi_context = Some(cpi_context);
        self.cpi_context_pubkey = Some(cpi_context_pubkey);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let compressed_mint_instruction_data = CompressedMintInstructionData {
            supply: 0,
            decimals: self.params.decimals,
            metadata: light_token_interface::state::CompressedMintMetadata {
                version: 3,
                mint: self.params.mint.to_bytes().into(),
                cmint_decompressed: false,
                mint_signer: self.mint_seed_pubkey.to_bytes(),
                bump: self.params.bump,
            },
            mint_authority: Some(self.params.mint_authority.to_bytes().into()),
            freeze_authority: self
                .params
                .freeze_authority
                .map(|auth| auth.to_bytes().into()),
            extensions: self.params.extensions,
        };

        let mut instruction_data =
            light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                self.params.address_merkle_tree_root_index,
                self.params.proof,
                compressed_mint_instruction_data,
            );

        if let Some(ctx) = self.cpi_context {
            instruction_data = instruction_data.with_cpi_context(ctx);
        }

        let mut meta_config = MintActionMetaConfig::new_create_mint(
            self.payer,
            self.params.mint_authority,
            self.mint_seed_pubkey,
            self.address_tree_pubkey,
            self.output_queue,
        );
        if let Some(cpi_context_pubkey) = self.cpi_context_pubkey {
            meta_config.cpi_context = Some(cpi_context_pubkey);
        }

        let account_metas = meta_config.to_account_metas();

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}

// ============================================================================
// AccountInfos Struct: CreateCMintCpi (for CPI usage)
// ============================================================================

/// # Create a compressed mint via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::{CreateMintCpi, CreateMintParams, SystemAccountInfos};
/// # use solana_account_info::AccountInfo;
/// # let mint_seed: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let address_tree: AccountInfo = todo!();
/// # let output_queue: AccountInfo = todo!();
/// # let system_accounts: SystemAccountInfos = todo!();
/// # let params: CreateMintParams = todo!();
/// CreateMintCpi {
///     mint_seed,
///     authority,
///     payer,
///     address_tree,
///     output_queue,
///     system_accounts,
///     cpi_context: None,
///     cpi_context_account: None,
///     params,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CreateMintCpi<'info> {
    pub mint_seed: AccountInfo<'info>,
    /// The authority for the mint (will be stored as mint_authority).
    pub authority: AccountInfo<'info>,
    /// The fee payer for the transaction.
    pub payer: AccountInfo<'info>,
    pub address_tree: AccountInfo<'info>,
    pub output_queue: AccountInfo<'info>,
    pub system_accounts: SystemAccountInfos<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub params: CreateMintParams,
}

impl<'info> CreateMintCpi<'info> {
    pub fn new(
        mint_seed: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        payer: AccountInfo<'info>,
        address_tree: AccountInfo<'info>,
        output_queue: AccountInfo<'info>,
        system_accounts: SystemAccountInfos<'info>,
        params: CreateMintParams,
    ) -> Self {
        Self {
            mint_seed,
            authority,
            payer,
            address_tree,
            output_queue,
            system_accounts,
            cpi_context: None,
            cpi_context_account: None,
            params,
        }
    }

    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateMint::try_from(self)?.instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from get_mint_action_instruction_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program, // Index 0
            self.mint_seed,                            // Index 1
            self.authority,                            // Index 2 (authority)
            self.payer,                                // Index 3 (fee_payer)
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
            self.output_queue,
            self.address_tree,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from get_mint_action_instruction_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program, // Index 0
            self.mint_seed,                            // Index 1
            self.authority,                            // Index 2 (authority)
            self.payer,                                // Index 3 (fee_payer)
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
            self.output_queue,
            self.address_tree,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> TryFrom<&CreateMintCpi<'info>> for CreateMint {
    type Error = ProgramError;

    fn try_from(account_infos: &CreateMintCpi<'info>) -> Result<Self, Self::Error> {
        if account_infos.params.mint_authority != *account_infos.authority.key {
            solana_msg::msg!(
                "CreateMintCpi: params.mint_authority ({}) does not match authority account ({})",
                account_infos.params.mint_authority,
                account_infos.authority.key
            );
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self {
            mint_seed_pubkey: *account_infos.mint_seed.key,
            payer: *account_infos.payer.key,
            address_tree_pubkey: *account_infos.address_tree.key,
            output_queue: *account_infos.output_queue.key,
            cpi_context: account_infos.cpi_context.clone(),
            cpi_context_pubkey: account_infos
                .cpi_context_account
                .as_ref()
                .map(|acc| *acc.key),
            params: account_infos.params.clone(),
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Derives the compressed mint address from the mint seed and address tree
pub fn derive_mint_compressed_address(
    mint_seed: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &find_mint_address(mint_seed).0.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Derives the compressed mint address from an SPL mint address
pub fn derive_mint_from_spl_mint(mint: &Pubkey, address_tree_pubkey: &Pubkey) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &mint.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Finds the compressed mint address from a mint seed.
pub fn find_mint_address(mint_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
    )
}
