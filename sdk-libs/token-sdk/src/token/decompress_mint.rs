use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, traits::LightInstructionData,
};
use light_token_interface::instructions::mint_action::{
    CompressedMintWithContext, CpiContext, DecompressMintAction,
    MintActionCompressedInstructionData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::{config_pda, rent_sponsor_pda, SystemAccountInfos};
use crate::compressed_token::mint_action::MintActionMetaConfig;

/// Decompress a compressed mint to a CMint Solana account.
///
/// Creates an on-chain CMint PDA that becomes the source of truth.
/// The CMint is always compressible.
///
/// # Example
/// ```rust,ignore
/// let instruction = DecompressMint {
///     payer,
///     authority,
///     state_tree,
///     input_queue,
///     output_queue,
///     compressed_mint_with_context,
///     proof,
///     rent_payment: 16,       // epochs (~24 hours rent)
///     write_top_up: 766,      // lamports (~3 hours rent per write)
/// }.instruction()?;
/// ```
#[derive(Debug, Clone)]
pub struct DecompressMint {
    /// Fee payer
    pub payer: Pubkey,
    /// Mint authority (must sign)
    pub authority: Pubkey,
    /// State tree for the compressed mint
    pub state_tree: Pubkey,
    /// Input queue for reading compressed mint
    pub input_queue: Pubkey,
    /// Output queue for updated compressed mint
    pub output_queue: Pubkey,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}

impl DecompressMint {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Get CMint PDA from compressed mint metadata
        let mint_data = self
            .compressed_mint_with_context
            .mint
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?;
        let cmint_pda = Pubkey::from(mint_data.metadata.mint.to_bytes());

        // Build DecompressMintAction
        let action = DecompressMintAction {
            rent_payment: self.rent_payment,
            write_top_up: self.write_top_up,
        };

        // Build instruction data
        let instruction_data = MintActionCompressedInstructionData::new(
            self.compressed_mint_with_context,
            self.proof.0,
        )
        .with_decompress_mint(action);

        // Build account metas with compressible CMint
        let meta_config = MintActionMetaConfig::new(
            self.payer,
            self.authority,
            self.state_tree,
            self.input_queue,
            self.output_queue,
        )
        .with_compressible_mint(cmint_pda, config_pda(), rent_sponsor_pda());

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
// CPI Struct: DecompressMintCpi
// ============================================================================

/// Decompress a compressed mint to a CMint Solana account via CPI.
///
/// Creates an on-chain CMint PDA that becomes the source of truth.
/// The CMint is always compressible.
///
/// # Example
/// ```rust,ignore
/// DecompressMintCpi {
///     authority: authority_account,
///     payer: payer_account,
///     cmint: cmint_account,
///     compressible_config: config_account,
///     rent_sponsor: rent_sponsor_account,
///     state_tree: state_tree_account,
///     input_queue: input_queue_account,
///     output_queue: output_queue_account,
///     system_accounts,
///     compressed_mint_with_context,
///     proof,
///     rent_payment: 16,
///     write_top_up: 766,
/// }
/// .invoke()?;
/// ```
pub struct DecompressMintCpi<'info> {
    /// Mint authority (must sign)
    pub authority: AccountInfo<'info>,
    /// Fee payer
    pub payer: AccountInfo<'info>,
    /// CMint PDA account (writable)
    pub cmint: AccountInfo<'info>,
    /// CompressibleConfig account
    pub compressible_config: AccountInfo<'info>,
    /// Rent sponsor PDA account
    pub rent_sponsor: AccountInfo<'info>,
    /// State tree for the compressed mint
    pub state_tree: AccountInfo<'info>,
    /// Input queue for reading compressed mint
    pub input_queue: AccountInfo<'info>,
    /// Output queue for updated compressed mint
    pub output_queue: AccountInfo<'info>,
    /// System accounts for Light Protocol
    pub system_accounts: SystemAccountInfos<'info>,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}

impl<'info> DecompressMintCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        DecompressMint::try_from(self)?.instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match to_account_metas() from MintActionMetaConfig:
        // 1. light_system_program
        // 2. mint_signer (no sign for decompress)
        // 3. authority (signer)
        // 4. compressible_config
        // 5. cmint
        // 6. rent_sponsor
        // 7. fee_payer (signer)
        // 8. cpi_authority_pda
        // 9. registered_program_pda
        // 10. account_compression_authority
        // 11. account_compression_program
        // 12. system_program
        // 13. output_queue
        // 14. tree_pubkey (state_tree)
        // 15. input_queue
        let account_infos = self.build_account_infos();
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = self.build_account_infos();
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }

    fn build_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.system_accounts.light_system_program.clone(),
            self.authority.clone(),
            self.compressible_config.clone(),
            self.cmint.clone(),
            self.rent_sponsor.clone(),
            self.payer.clone(),
            self.system_accounts.cpi_authority_pda.clone(),
            self.system_accounts.registered_program_pda.clone(),
            self.system_accounts.account_compression_authority.clone(),
            self.system_accounts.account_compression_program.clone(),
            self.system_accounts.system_program.clone(),
            self.output_queue.clone(),
            self.state_tree.clone(),
            self.input_queue.clone(),
        ]
    }
}

impl<'info> TryFrom<&DecompressMintCpi<'info>> for DecompressMint {
    type Error = ProgramError;

    fn try_from(cpi: &DecompressMintCpi<'info>) -> Result<Self, Self::Error> {
        Ok(Self {
            payer: *cpi.payer.key,
            authority: *cpi.authority.key,
            state_tree: *cpi.state_tree.key,
            input_queue: *cpi.input_queue.key,
            output_queue: *cpi.output_queue.key,
            compressed_mint_with_context: cpi.compressed_mint_with_context.clone(),
            proof: cpi.proof,
            rent_payment: cpi.rent_payment,
            write_top_up: cpi.write_top_up,
        })
    }
}

/// Decompress a compressed mint with CPI context support.
///
/// For use in multi-operation ixns where mints are decompressed
/// along with PDAs and token accounts using a single proof.
#[derive(Debug, Clone)]
pub struct DecompressCMintWithCpiContext {
    /// Mint seed pubkey (used to derive CMint PDA)
    pub mint_seed_pubkey: Pubkey,
    /// Fee payer
    pub payer: Pubkey,
    /// Mint authority (must sign)
    pub authority: Pubkey,
    /// State tree for the compressed mint
    pub state_tree: Pubkey,
    /// Input queue for reading compressed mint
    pub input_queue: Pubkey,
    /// Output queue for updated compressed mint
    pub output_queue: Pubkey,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
    /// CPI context account
    pub cpi_context_pubkey: Pubkey,
    /// CPI context flags
    pub cpi_context: CpiContext,
    /// Compressible config account (ctoken's config)
    pub compressible_config: Pubkey,
    /// Rent sponsor account (ctoken's rent sponsor)
    pub rent_sponsor: Pubkey,
}

impl DecompressCMintWithCpiContext {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Derive CMint PDA
        let (cmint_pda, _cmint_bump) = crate::token::find_mint_address(&self.mint_seed_pubkey);

        // Build DecompressMintAction
        let action = DecompressMintAction {
            rent_payment: self.rent_payment,
            write_top_up: self.write_top_up,
        };

        // Build instruction data with CPI context
        let instruction_data = MintActionCompressedInstructionData::new(
            self.compressed_mint_with_context,
            self.proof.0,
        )
        .with_decompress_mint(action)
        .with_cpi_context(self.cpi_context.clone());

        // Build account metas with compressible CMint and CPI context
        // Use provided config/rent_sponsor instead of hardcoded defaults
        let mut meta_config = MintActionMetaConfig::new(
            self.payer,
            self.authority,
            self.state_tree,
            self.input_queue,
            self.output_queue,
        )
        .with_compressible_mint(cmint_pda, self.compressible_config, self.rent_sponsor);

        meta_config.cpi_context = Some(self.cpi_context_pubkey);

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

/// CPI struct for decompressing a mint with CPI context.
pub struct DecompressCMintCpiWithContext<'info> {
    /// Mint seed account (used to derive CMint PDA, does not sign)
    pub mint_seed: AccountInfo<'info>,
    /// Mint authority (must sign)
    pub authority: AccountInfo<'info>,
    /// Fee payer
    pub payer: AccountInfo<'info>,
    /// CMint PDA account (writable)
    pub cmint: AccountInfo<'info>,
    /// CompressibleConfig account
    pub compressible_config: AccountInfo<'info>,
    /// Rent sponsor PDA account
    pub rent_sponsor: AccountInfo<'info>,
    /// State tree for the compressed mint
    pub state_tree: AccountInfo<'info>,
    /// Input queue for reading compressed mint
    pub input_queue: AccountInfo<'info>,
    /// Output queue for updated compressed mint
    pub output_queue: AccountInfo<'info>,
    /// CPI context account
    pub cpi_context_account: AccountInfo<'info>,
    /// System accounts for Light Protocol
    pub system_accounts: SystemAccountInfos<'info>,
    /// CToken program's CPI authority (GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy)
    /// This is separate from system_accounts.cpi_authority_pda which is the calling program's authority
    pub ctoken_cpi_authority: AccountInfo<'info>,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
    /// CPI context flags
    pub cpi_context: CpiContext,
}

impl<'info> DecompressCMintCpiWithContext<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        DecompressCMintWithCpiContext {
            mint_seed_pubkey: *self.mint_seed.key,
            payer: *self.payer.key,
            authority: *self.authority.key,
            state_tree: *self.state_tree.key,
            input_queue: *self.input_queue.key,
            output_queue: *self.output_queue.key,
            compressed_mint_with_context: self.compressed_mint_with_context.clone(),
            proof: self.proof,
            rent_payment: self.rent_payment,
            write_top_up: self.write_top_up,
            cpi_context_pubkey: *self.cpi_context_account.key,
            cpi_context: self.cpi_context.clone(),
            compressible_config: *self.compressible_config.key,
            rent_sponsor: *self.rent_sponsor.key,
        }
        .instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = self.build_account_infos();
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = self.build_account_infos();
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }

    fn build_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.system_accounts.light_system_program.clone(),
            self.mint_seed.clone(),
            self.authority.clone(),
            self.compressible_config.clone(),
            self.cmint.clone(),
            self.rent_sponsor.clone(),
            self.payer.clone(),
            // Use ctoken's CPI authority for the CPI, not the calling program's authority
            self.ctoken_cpi_authority.clone(),
            self.system_accounts.registered_program_pda.clone(),
            self.system_accounts.account_compression_authority.clone(),
            self.system_accounts.account_compression_program.clone(),
            self.system_accounts.system_program.clone(),
            self.cpi_context_account.clone(),
            self.output_queue.clone(),
            self.state_tree.clone(),
            self.input_queue.clone(),
        ]
    }
}

/// Helper to create CPI context for first write (first_set_context = true)
pub fn create_decompress_mint_cpi_context_first(
    address_tree_pubkey: [u8; 32],
    tree_index: u8,
    queue_index: u8,
) -> CpiContext {
    CpiContext {
        first_set_context: true,
        set_context: false,
        in_tree_index: tree_index,
        in_queue_index: queue_index,
        out_queue_index: queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey,
    }
}

/// Helper to create CPI context for subsequent writes (set_context = true)
pub fn create_decompress_mint_cpi_context_set(
    address_tree_pubkey: [u8; 32],
    tree_index: u8,
    queue_index: u8,
) -> CpiContext {
    CpiContext {
        first_set_context: false,
        set_context: true,
        in_tree_index: tree_index,
        in_queue_index: queue_index,
        out_queue_index: queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey,
    }
}

/// Helper to create CPI context for execution (both false - consumes context)
pub fn create_decompress_mint_cpi_context_execute(
    address_tree_pubkey: [u8; 32],
    tree_index: u8,
    queue_index: u8,
) -> CpiContext {
    CpiContext {
        first_set_context: false,
        set_context: false,
        in_tree_index: tree_index,
        in_queue_index: queue_index,
        out_queue_index: queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey,
    }
}
