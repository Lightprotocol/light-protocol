//! Transfer from CToken account to SPL token account via CPI.

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

/// Transfer from CToken account to SPL token account via CPI.
///
/// # Example
/// ```rust,ignore
/// TransferToSplCpi {
///     source: &source_ctoken,
///     destination_spl_token_account: &destination_spl,
///     amount: 100,
///     authority: &authority,
///     mint: &mint,
///     payer: &payer,
///     spl_interface_pda: &spl_interface,
///     spl_interface_pda_bump: 255,
///     decimals: 9,
///     spl_token_program: &spl_token,
///     compressed_token_program_authority: &cpi_authority,
/// }
/// .invoke()?;
/// ```
pub struct TransferToSplCpi<'info> {
    pub source: &'info AccountInfo,
    pub destination_spl_token_account: &'info AccountInfo,
    pub amount: u64,
    pub authority: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub payer: &'info AccountInfo,
    pub spl_interface_pda: &'info AccountInfo,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
    pub spl_token_program: &'info AccountInfo,
    pub compressed_token_program_authority: &'info AccountInfo,
}

impl<'info> TransferToSplCpi<'info> {
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
        // 1. Compress from ctoken account to pool
        // 2. Decompress from pool to SPL token account
        let compress_to_pool = Compression::compress(
            self.amount,
            0, // mint index
            1, // source ctoken account index
            3, // authority index
        );

        let decompress_to_spl = Compression::decompress_spl(
            self.amount,
            0, // mint index
            2, // destination SPL token account index
            4, // pool_account_index
            0, // pool_index
            self.spl_interface_pda_bump,
            self.decimals,
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
            compressions: Some(vec![compress_to_pool, decompress_to_spl]),
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
        //   - [1] source ctoken account (writable)
        //   - [2] destination SPL token account (writable)
        //   - [3] authority (signer, readonly)
        //   - [4] SPL interface PDA (writable)
        //   - [5] SPL Token program (readonly)
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
                pubkey: *self.source.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.destination_spl_token_account.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: *self.authority.key(),
                is_signer: true,
                is_writable: false,
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
        ];

        let account_infos = vec![
            *self.compressed_token_program_authority,
            *self.payer,
            *self.mint,
            *self.source,
            *self.destination_spl_token_account,
            *self.authority,
            *self.spl_interface_pda,
            *self.spl_token_program,
        ];

        Ok((ix_data, metas, account_infos))
    }
}
