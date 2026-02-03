//! Transfer from SPL token account to CToken account via CPI.

use alloc::{vec, vec::Vec};

use borsh::BorshSerialize;
use light_account_checks::{AccountInfoTrait, CpiMeta};
use light_token_interface::{
    instructions::transfer2::{
        Compression, CompressedTokenInstructionDataTransfer2, MultiTokenTransferOutputData,
    },
    LIGHT_TOKEN_PROGRAM_ID,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

/// Discriminator for Transfer2 instruction
const TRANSFER2_DISCRIMINATOR: u8 = 4;

/// Transfer from SPL token account to CToken account via CPI.
///
/// # Example
/// ```rust,ignore
/// TransferFromSplCpi {
///     amount: 100,
///     spl_interface_pda_bump: 255,
///     decimals: 9,
///     source_spl_token_account: &source,
///     destination: &destination,
///     authority: &authority,
///     mint: &mint,
///     payer: &payer,
///     spl_interface_pda: &spl_interface,
///     spl_token_program: &spl_token,
///     compressed_token_program_authority: &cpi_authority,
///     system_program: &system,
/// }
/// .invoke()?;
/// ```
pub struct TransferFromSplCpi<'info> {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
    pub source_spl_token_account: &'info AccountInfo,
    /// Destination ctoken account (writable)
    pub destination: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub payer: &'info AccountInfo,
    pub spl_interface_pda: &'info AccountInfo,
    pub spl_token_program: &'info AccountInfo,
    pub compressed_token_program_authority: &'info AccountInfo,
    /// System program - required for compressible account lamport top-ups
    pub system_program: &'info AccountInfo,
}

impl<'info> TransferFromSplCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        let (ix_data, metas, account_infos) = self.build_instruction_inner()?;
        AccountInfo::invoke_cpi(&LIGHT_TOKEN_PROGRAM_ID, &ix_data, &metas, &account_infos, &[])
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
        // Build compressions:
        // 1. Wrap SPL tokens to compressed pool
        // 2. Unwrap from pool to destination ctoken account
        let wrap_from_spl = Compression::compress_spl(
            self.amount,
            0, // mint index
            3, // source index
            2, // authority index
            4, // pool_account_index
            0, // pool_index
            self.spl_interface_pda_bump,
            self.decimals,
        );

        let unwrap_to_destination = Compression::decompress(
            self.amount,
            0, // mint index
            1, // destination index
        );

        // Build instruction data
        let instruction_data = CompressedTokenInstructionDataTransfer2 {
            with_transaction_hash: false,
            with_lamports_change_account_merkle_tree_index: false,
            lamports_change_account_merkle_tree_index: 0,
            lamports_change_account_owner_index: 0,
            output_queue: 0,
            max_top_up: 0,
            cpi_context: None,
            compressions: Some(vec![wrap_from_spl, unwrap_to_destination]),
            proof: None,
            in_token_data: vec![],
            out_token_data: vec![
                MultiTokenTransferOutputData::default(),
                MultiTokenTransferOutputData::default(),
            ],
            in_lamports: None,
            out_lamports: None,
            in_tlv: None,
            out_tlv: None,
        };

        let mut ix_data = vec![TRANSFER2_DISCRIMINATOR];
        instruction_data
            .serialize(&mut ix_data)
            .map_err(|_| ProgramError::BorshIoError)?;

        // Build account metas matching transfer2 format:
        // [0] cpi_authority_pda (readonly)
        // [1] fee_payer (signer, writable)
        // [2..] packed_accounts:
        //   - [0] mint (readonly)
        //   - [1] destination ctoken account (writable)
        //   - [2] authority (signer, readonly)
        //   - [3] source SPL token account (writable)
        //   - [4] SPL interface PDA (writable)
        //   - [5] SPL Token program (readonly)
        //   - [6] System program (readonly)
        let metas = vec![
            CpiMeta {
                pubkey: *self.compressed_token_program_authority.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.payer.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.mint.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.destination.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.authority.key(),
                is_signer: true,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.source_spl_token_account.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.spl_interface_pda.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.spl_token_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: *self.system_program.key(),
                is_signer: false,
                is_writable: false,
            },
        ];

        let account_infos = vec![
            *self.compressed_token_program_authority,
            *self.payer,
            *self.mint,
            *self.destination,
            *self.authority,
            *self.source_spl_token_account,
            *self.spl_interface_pda,
            *self.spl_token_program,
            *self.system_program,
        ];

        Ok((ix_data, metas, account_infos))
    }
}
