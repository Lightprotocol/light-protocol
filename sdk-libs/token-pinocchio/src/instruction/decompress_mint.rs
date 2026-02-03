//! Decompress mint CPI for pinocchio.

use alloc::{vec, vec::Vec};

use light_account_checks::{AccountInfoTrait, CpiMeta};
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, traits::LightInstructionData,
};
use light_token_interface::{
    instructions::mint_action::{
        CpiContext, DecompressMintAction, MintActionCompressedInstructionData, MintWithContext,
    },
    LIGHT_TOKEN_PROGRAM_ID,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use crate::instruction::SystemAccountInfos;

/// Decompress a compressed mint to a Mint Solana account via CPI.
///
/// Creates an on-chain Mint PDA that becomes the source of truth.
/// The Mint is always compressible.
///
/// # Example
/// ```rust,ignore
/// DecompressMintCpi {
///     authority: &authority_account,
///     payer: &payer_account,
///     mint: &mint_account,
///     compressible_config: &config_account,
///     rent_sponsor: &rent_sponsor_account,
///     state_tree: &state_tree_account,
///     input_queue: &input_queue_account,
///     output_queue: &output_queue_account,
///     system_accounts: &system_accounts,
///     compressed_mint_with_context,
///     proof,
///     rent_payment: 16,
///     write_top_up: 766,
/// }
/// .invoke()?;
/// ```
pub struct DecompressMintCpi<'info> {
    /// Mint authority (must sign)
    pub authority: &'info AccountInfo,
    /// Fee payer
    pub payer: &'info AccountInfo,
    /// Mint PDA account (writable)
    pub mint: &'info AccountInfo,
    /// CompressibleConfig account
    pub compressible_config: &'info AccountInfo,
    /// Rent sponsor PDA account
    pub rent_sponsor: &'info AccountInfo,
    /// State tree for the compressed mint
    pub state_tree: &'info AccountInfo,
    /// Input queue for reading compressed mint
    pub input_queue: &'info AccountInfo,
    /// Output queue for updated compressed mint
    pub output_queue: &'info AccountInfo,
    /// System accounts for Light Protocol
    pub system_accounts: SystemAccountInfos<'info>,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: MintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}

impl<'info> DecompressMintCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        let (ix_data, metas, account_infos) = self.build_instruction_inner()?;
        AccountInfo::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            &ix_data,
            &metas,
            &account_infos,
            &[],
        )
        .map_err(|_| ProgramError::Custom(0))
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let (ix_data, metas, account_infos) = self.build_instruction_inner()?;
        AccountInfo::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            &ix_data,
            &metas,
            &account_infos,
            signer_seeds,
        )
        .map_err(|_| ProgramError::Custom(0))
    }

    #[allow(clippy::type_complexity)]
    fn build_instruction_inner(
        &self,
    ) -> Result<(Vec<u8>, Vec<CpiMeta>, Vec<AccountInfo>), ProgramError> {
        // Build DecompressMintAction
        let action = DecompressMintAction {
            rent_payment: self.rent_payment,
            write_top_up: self.write_top_up,
        };

        // Build instruction data
        let instruction_data = MintActionCompressedInstructionData::new(
            self.compressed_mint_with_context.clone(),
            self.proof.0,
        )
        .with_decompress_mint(action);

        let ix_data = instruction_data
            .data()
            .map_err(|_| ProgramError::BorshIoError)?;

        // Build account metas and account infos in matching order
        // Order matches MintActionMetaConfig::to_account_metas:
        // 1. light_system_program
        // 2. authority (signer)
        // 3. compressible_config
        // 4. mint (writable)
        // 5. rent_sponsor (writable)
        // 6. fee_payer (signer, writable)
        // 7. cpi_authority_pda
        // 8. registered_program_pda
        // 9. account_compression_authority
        // 10. account_compression_program
        // 11. system_program
        // 12. output_queue (writable)
        // 13. tree_pubkey (state_tree, writable)
        // 14. input_queue (writable)

        let metas = vec![
            CpiMeta {
                pubkey: *self.system_accounts.light_system_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.authority.key(),
                is_signer: true,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.compressible_config.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.mint.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.rent_sponsor.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.payer.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.system_accounts.cpi_authority_pda.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.system_accounts.registered_program_pda.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.system_accounts.account_compression_authority.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.system_accounts.account_compression_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.system_accounts.system_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.output_queue.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.state_tree.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.input_queue.key(),
                is_signer: false,
                is_writable: true,
            },
        ];

        let account_infos = vec![
            *self.system_accounts.light_system_program,
            *self.authority,
            *self.compressible_config,
            *self.mint,
            *self.rent_sponsor,
            *self.payer,
            *self.system_accounts.cpi_authority_pda,
            *self.system_accounts.registered_program_pda,
            *self.system_accounts.account_compression_authority,
            *self.system_accounts.account_compression_program,
            *self.system_accounts.system_program,
            *self.output_queue,
            *self.state_tree,
            *self.input_queue,
        ];

        Ok((ix_data, metas, account_infos))
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
