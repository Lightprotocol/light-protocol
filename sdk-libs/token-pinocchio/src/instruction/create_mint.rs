//! Create compressed mint CPI builder for pinocchio.
//!
//! Provides `CreateMintParams`, `CreateMintCpi`, and helper functions
//! for creating compressed mints via CPI from pinocchio-based programs.

use alloc::{vec, vec::Vec};

use light_account_checks::AccountInfoTrait;
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CpiContext, DecompressMintAction, MintActionCompressedInstructionData},
    },
    state::MintMetadata,
    COMPRESSED_MINT_SEED,
};
use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{constants::LIGHT_TOKEN_PROGRAM_ID, instruction::SystemAccountInfos};

/// Parameters for creating a mint.
///
/// Creates both a compressed mint AND a decompressed Mint Solana account
/// in a single instruction.
#[derive(Debug, Clone)]
pub struct CreateMintParams {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: [u8; 32],
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: [u8; 32],
    pub bump: u8,
    pub freeze_authority: Option<[u8; 32]>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    /// Rent payment in epochs for the Mint account (must be 0 or >= 2).
    /// Default: 16 (~24 hours)
    pub rent_payment: u8,
    /// Lamports allocated for future write operations.
    /// Default: 766 (~3 hours per write)
    pub write_top_up: u32,
}

impl Default for CreateMintParams {
    fn default() -> Self {
        Self {
            decimals: 9,
            address_merkle_tree_root_index: 0,
            mint_authority: [0u8; 32],
            proof: CompressedProof::default(),
            compression_address: [0u8; 32],
            mint: [0u8; 32],
            bump: 0,
            freeze_authority: None,
            extensions: None,
            rent_payment: 16,
            write_top_up: 766,
        }
    }
}

/// Create a mint via CPI.
///
/// Creates both a compressed mint AND a decompressed Mint Solana account
/// in a single instruction.
///
/// # Example
/// ```rust,ignore
/// CreateMintCpi {
///     mint_seed: &mint_seed_account,
///     authority: &authority_account,
///     payer: &payer_account,
///     address_tree: &address_tree_account,
///     output_queue: &output_queue_account,
///     compressible_config: &config_account,
///     mint: &mint_account,
///     rent_sponsor: &rent_sponsor_account,
///     system_accounts: &system_accounts,
///     cpi_context: None,
///     cpi_context_account: None,
///     params: CreateMintParams {
///         decimals: 9,
///         mint_authority: authority_pubkey,
///         // ... other params from validity proof
///         ..Default::default()
///     },
/// }
/// .invoke()?;
/// ```
pub struct CreateMintCpi<'info> {
    /// Used as seed for the mint address (must be a signer).
    pub mint_seed: &'info AccountInfo,
    /// The authority for the mint (will be stored as mint_authority).
    pub authority: &'info AccountInfo,
    /// The fee payer for the transaction.
    pub payer: &'info AccountInfo,
    pub address_tree: &'info AccountInfo,
    pub output_queue: &'info AccountInfo,
    /// CompressibleConfig account (required for Mint creation)
    pub compressible_config: &'info AccountInfo,
    /// Mint PDA account (writable, will be initialized)
    pub mint: &'info AccountInfo,
    /// Rent sponsor PDA (required for Mint creation)
    pub rent_sponsor: &'info AccountInfo,
    pub system_accounts: SystemAccountInfos<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<&'info AccountInfo>,
    pub params: CreateMintParams,
}

impl<'info> CreateMintCpi<'info> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mint_seed: &'info AccountInfo,
        authority: &'info AccountInfo,
        payer: &'info AccountInfo,
        address_tree: &'info AccountInfo,
        output_queue: &'info AccountInfo,
        compressible_config: &'info AccountInfo,
        mint: &'info AccountInfo,
        rent_sponsor: &'info AccountInfo,
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

    pub fn with_cpi_context(
        mut self,
        cpi_context: CpiContext,
        cpi_context_account: &'info AccountInfo,
    ) -> Self {
        self.cpi_context = Some(cpi_context);
        self.cpi_context_account = Some(cpi_context_account);
        self
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let (ix_data, account_metas, account_infos) = self.build_instruction_inner()?;

        let program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &ix_data,
        };

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }

    #[allow(clippy::type_complexity)]
    fn build_instruction_inner(
        &self,
    ) -> Result<(Vec<u8>, Vec<AccountMeta<'_>>, Vec<&AccountInfo>), ProgramError> {
        // Validate mint_authority matches authority account
        if self.params.mint_authority != *self.authority.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Build MintInstructionData
        let mint_instruction_data =
            light_token_interface::instructions::mint_action::MintInstructionData {
                supply: 0,
                decimals: self.params.decimals,
                metadata: MintMetadata {
                    version: 3,
                    mint: self.params.mint.into(),
                    mint_decompressed: false,
                    mint_signer: *self.mint_seed.key(),
                    bump: self.params.bump,
                },
                mint_authority: Some(self.params.mint_authority.into()),
                freeze_authority: self.params.freeze_authority.map(|auth| auth.into()),
                extensions: self.params.extensions.clone(),
            };

        // Build instruction data
        let mut instruction_data = MintActionCompressedInstructionData::new_mint(
            self.params.address_merkle_tree_root_index,
            self.params.proof,
            mint_instruction_data,
        );

        // Always add decompress action to create Mint Solana account
        instruction_data = instruction_data.with_decompress_mint(DecompressMintAction {
            rent_payment: self.params.rent_payment,
            write_top_up: self.params.write_top_up,
        });

        if let Some(ctx) = &self.cpi_context {
            instruction_data = instruction_data.with_cpi_context(ctx.clone());
        }

        let ix_data = instruction_data
            .data()
            .map_err(|_| ProgramError::BorshIoError)?;

        // Build account metas and account infos in matching order
        // Order matches MintActionMetaConfig::to_account_metas:
        // 1. light_system_program
        // 2. mint_seed (signer)
        // 3. authority (signer)
        // 4. compressible_config
        // 5. mint (writable)
        // 6. rent_sponsor (writable)
        // 7. fee_payer (signer, writable)
        // 8. cpi_authority_pda
        // 9. registered_program_pda
        // 10. account_compression_authority
        // 11. account_compression_program
        // 12. system_program
        // [optional: cpi_context_account]
        // 13. output_queue (writable)
        // 14. address_tree (writable)

        let mut account_metas = vec![
            AccountMeta::readonly(self.system_accounts.light_system_program.key()),
            AccountMeta::readonly_signer(self.mint_seed.key()),
            AccountMeta::readonly_signer(self.authority.key()),
            AccountMeta::readonly(self.compressible_config.key()),
            AccountMeta::writable(self.mint.key()),
            AccountMeta::writable(self.rent_sponsor.key()),
            AccountMeta::writable_signer(self.payer.key()),
            AccountMeta::readonly(self.system_accounts.cpi_authority_pda.key()),
            AccountMeta::readonly(self.system_accounts.registered_program_pda.key()),
            AccountMeta::readonly(self.system_accounts.account_compression_authority.key()),
            AccountMeta::readonly(self.system_accounts.account_compression_program.key()),
            AccountMeta::readonly(self.system_accounts.system_program.key()),
        ];

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

        // Add optional cpi_context_account
        if let Some(cpi_ctx_acc) = self.cpi_context_account {
            account_metas.push(AccountMeta::writable(cpi_ctx_acc.key()));
            account_infos.push(cpi_ctx_acc);
        }

        // Add output_queue and address_tree
        account_metas.push(AccountMeta::writable(self.output_queue.key()));
        account_metas.push(AccountMeta::writable(self.address_tree.key()));

        account_infos.push(self.output_queue);
        account_infos.push(self.address_tree);

        Ok((ix_data, account_metas, account_infos))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Derives the compressed mint address from the mint seed and address tree.
pub fn derive_mint_compressed_address(
    mint_seed: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
) -> [u8; 32] {
    let (mint_pda, _) = find_mint_address(mint_seed);
    light_compressed_account::address::derive_address(
        &mint_pda,
        address_tree_pubkey,
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Derives the compressed mint address from an SPL mint address.
pub fn derive_mint_from_spl_mint(mint: &[u8; 32], address_tree_pubkey: &[u8; 32]) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        mint,
        address_tree_pubkey,
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Finds the compressed mint PDA address from a mint seed.
pub fn find_mint_address(mint_seed: &[u8; 32]) -> ([u8; 32], u8) {
    AccountInfo::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}
