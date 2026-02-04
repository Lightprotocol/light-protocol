//! Transfer from SPL token account to CToken account via CPI.

use alloc::{vec, vec::Vec};

use borsh::BorshSerialize;
use light_token_interface::{
    instructions::transfer2::{CompressedTokenInstructionDataTransfer2, Compression},
    LIGHT_TOKEN_PROGRAM_ID,
};
use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Discriminator for Transfer2 instruction
const TRANSFER2_DISCRIMINATOR: u8 = 101;

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
        // Build compressions:
        // 1. Wrap SPL tokens to Light Token pool
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
        // Note: out_token_data must be empty for compressions-only (Path A) operations.
        // The program determines the path based on: no_compressed_accounts = in_token_data.is_empty() && out_token_data.is_empty()
        // If out_token_data is non-empty, the program expects Path B accounts (with light_system_program, registered_program_pda, etc.)
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
            out_token_data: vec![],
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
        let account_metas = vec![
            AccountMeta::readonly(self.compressed_token_program_authority.key()),
            AccountMeta::writable_signer(self.payer.key()),
            AccountMeta::readonly(self.mint.key()),
            AccountMeta::writable(self.destination.key()),
            AccountMeta::readonly_signer(self.authority.key()),
            AccountMeta::writable(self.source_spl_token_account.key()),
            AccountMeta::writable(self.spl_interface_pda.key()),
            AccountMeta::readonly(self.spl_token_program.key()),
            AccountMeta::readonly(self.system_program.key()),
        ];

        let account_infos = vec![
            self.compressed_token_program_authority,
            self.payer,
            self.mint,
            self.destination,
            self.authority,
            self.source_spl_token_account,
            self.spl_interface_pda,
            self.spl_token_program,
            self.system_program,
        ];

        Ok((ix_data, account_metas, account_infos))
    }
}
