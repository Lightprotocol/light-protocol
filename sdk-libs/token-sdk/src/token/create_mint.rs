use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CpiContext, DecompressMintAction, MintInstructionData},
    },
    COMPRESSED_MINT_SEED,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::{config_pda, rent_sponsor_pda};
use crate::{compressed_token::mint_action::MintActionMetaConfig, token::SystemAccountInfos};
/// Parameters for creating a mint.
///
/// Creates both a compressed mint AND a decompressed Mint Solana account
/// in a single instruction.
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
    /// Rent payment in epochs for the Mint account (must be 0 or >= 2).
    /// Default: 16 (~24 hours)
    pub rent_payment: u8,
    /// Lamports allocated for future write operations.
    /// Default: 766 (~3 hours per write)
    pub write_top_up: u32,
}

/// Create a mint instruction that creates both a compressed mint AND a Mint Solana account.
///
/// # Example
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
/// let (mint, bump) = find_mint_address(&mint_seed_pubkey);
///
/// let params = CreateMintParams {
///     decimals: 9,
///     address_merkle_tree_root_index, // from rpc.get_validity_proof
///     mint_authority,
///     proof, // from rpc.get_validity_proof
///     compression_address,
///     mint,
///     bump,
///     freeze_authority: None,
///     extensions: None,
///     rent_payment: 16,  // ~24 hours rent
///     write_top_up: 766, // ~3 hours per write
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
        let compressed_mint_instruction_data = MintInstructionData {
            supply: 0,
            decimals: self.params.decimals,
            metadata: light_token_interface::state::MintMetadata {
                version: 3,
                mint: self.params.mint.to_bytes().into(),
                mint_decompressed: false,
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

        // Always add decompress action to create Mint Solana account
        instruction_data = instruction_data.with_decompress_mint(DecompressMintAction {
            rent_payment: self.params.rent_payment,
            write_top_up: self.params.write_top_up,
        });

        if let Some(ctx) = self.cpi_context {
            instruction_data = instruction_data.with_cpi_context(ctx);
        }

        let mut meta_config = MintActionMetaConfig::new_create_mint(
            self.payer,
            self.params.mint_authority,
            self.mint_seed_pubkey,
            self.address_tree_pubkey,
            self.output_queue,
        )
        // Always include compressible accounts for Mint creation
        .with_compressible_mint(self.params.mint, config_pda(), rent_sponsor_pda());

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
// AccountInfos Struct: CreateMintCpi (for CPI usage)
// ============================================================================

/// # Create a mint via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::{CreateMintCpi, CreateMintParams, SystemAccountInfos};
/// # use solana_account_info::AccountInfo;
/// # let mint_seed: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let address_tree: AccountInfo = todo!();
/// # let output_queue: AccountInfo = todo!();
/// # let compressible_config: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let rent_sponsor: AccountInfo = todo!();
/// # let system_accounts: SystemAccountInfos = todo!();
/// # let params: CreateMintParams = todo!();
/// CreateMintCpi {
///     mint_seed,
///     authority,
///     payer,
///     address_tree,
///     output_queue,
///     compressible_config,
///     mint,
///     rent_sponsor,
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
    /// CompressibleConfig account (required for Mint creation)
    pub compressible_config: AccountInfo<'info>,
    /// Mint PDA account (writable, will be initialized)
    pub mint: AccountInfo<'info>,
    /// Rent sponsor PDA (required for Mint creation)
    pub rent_sponsor: AccountInfo<'info>,
    pub system_accounts: SystemAccountInfos<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub params: CreateMintParams,
}

impl<'info> CreateMintCpi<'info> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mint_seed: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        payer: AccountInfo<'info>,
        address_tree: AccountInfo<'info>,
        output_queue: AccountInfo<'info>,
        compressible_config: AccountInfo<'info>,
        mint: AccountInfo<'info>,
        rent_sponsor: AccountInfo<'info>,
        system_accounts: SystemAccountInfos<'info>,
        params: CreateMintParams,
    ) -> Self {
        Self {
            mint_seed,
            authority,
            payer,
            address_tree,
            output_queue,
            compressible_config,
            mint,
            rent_sponsor,
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

        // Account order must match MintActionMetaConfig::to_account_metas()
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.mint_seed,
            self.authority,
            self.compressible_config,
            self.mint,
            self.rent_sponsor,
            self.payer,
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        account_infos.push(self.output_queue);
        account_infos.push(self.address_tree);

        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match MintActionMetaConfig::to_account_metas()
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.mint_seed,
            self.authority,
            self.compressible_config,
            self.mint,
            self.rent_sponsor,
            self.payer,
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        account_infos.push(self.output_queue);
        account_infos.push(self.address_tree);

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
