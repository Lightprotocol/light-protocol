use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_compressed_token_sdk::compressed_token::{
    transfer2::{
        create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config, Transfer2Inputs,
    },
    CTokenAccount2,
};
use light_token_interface::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Create a transfer SPL to cToken instruction
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::TransferFromSpl;
/// # let source_spl_token_account = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let spl_interface_pda = Pubkey::new_unique();
/// # let spl_token_program = Pubkey::new_unique();
/// let instruction = TransferFromSpl {
///     amount: 100,
///     spl_interface_pda_bump: 255,
///     decimals: 9,
///     source_spl_token_account,
///     destination,
///     authority,
///     mint,
///     payer,
///     spl_interface_pda,
///     spl_token_program,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferFromSpl {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
    pub source_spl_token_account: Pubkey,
    /// Destination ctoken account (writable)
    pub destination: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub spl_interface_pda: Pubkey,
    pub spl_token_program: Pubkey,
}

/// # Transfer SPL to ctoken via CPI:
/// ```rust,no_run
/// # use light_token::instruction::TransferFromSplCpi;
/// # use solana_account_info::AccountInfo;
/// # let source_spl_token_account: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let spl_interface_pda: AccountInfo = todo!();
/// # let spl_token_program: AccountInfo = todo!();
/// # let compressed_token_program_authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// TransferFromSplCpi {
///     amount: 100,
///     spl_interface_pda_bump: 255,
///     decimals: 9,
///     source_spl_token_account,
///     destination,
///     authority,
///     mint,
///     payer,
///     spl_interface_pda,
///     spl_token_program,
///     compressed_token_program_authority,
///     system_program,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferFromSplCpi<'info> {
    pub amount: u64,
    pub spl_interface_pda_bump: u8,
    pub decimals: u8,
    pub source_spl_token_account: AccountInfo<'info>,
    /// Destination ctoken account (writable)
    pub destination: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: AccountInfo<'info>,
}

impl<'info> TransferFromSplCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferFromSpl::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferFromSpl::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.destination,                        // Index 1: Destination ctoken account
            self.authority,                          // Index 2: Authority (signer)
            self.source_spl_token_account,           // Index 3: Source SPL token account
            self.spl_interface_pda,                  // Index 4: SPL interface PDA
            self.spl_token_program,                  // Index 5: SPL Token program
            self.system_program,                     // Index 6: System program
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferFromSpl::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.destination,                        // Index 1: Destination ctoken account
            self.authority,                          // Index 2: Authority (signer)
            self.source_spl_token_account,           // Index 3: Source SPL token account
            self.spl_interface_pda,                  // Index 4: SPL interface PDA
            self.spl_token_program,                  // Index 5: SPL Token program
            self.system_program,                     // Index 6: System program
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferFromSplCpi<'info>> for TransferFromSpl {
    fn from(account_infos: &TransferFromSplCpi<'info>) -> Self {
        Self {
            source_spl_token_account: *account_infos.source_spl_token_account.key,
            destination: *account_infos.destination.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            mint: *account_infos.mint.key,
            payer: *account_infos.payer.key,
            spl_interface_pda: *account_infos.spl_interface_pda.key,
            spl_interface_pda_bump: account_infos.spl_interface_pda_bump,
            decimals: account_infos.decimals,
            spl_token_program: *account_infos.spl_token_program.key,
        }
    }
}

impl TransferFromSpl {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let packed_accounts = vec![
            // Mint (index 0)
            AccountMeta::new_readonly(self.mint, false),
            // Destination ctoken account (index 1) - writable
            AccountMeta::new(self.destination, false),
            // Authority for compression (index 2) - signer
            AccountMeta::new_readonly(self.authority, true),
            // Source SPL token account (index 3) - writable
            AccountMeta::new(self.source_spl_token_account, false),
            // SPL interface PDA (index 4) - writable
            AccountMeta::new(self.spl_interface_pda, false),
            // SPL Token program (index 5) - needed for CPI
            AccountMeta::new_readonly(self.spl_token_program, false),
            // System program (index 6) - needed for compressible account lamport top-ups
            AccountMeta::new_readonly(Pubkey::default(), false),
        ];

        let wrap_from_spl = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::compress_spl(
                self.amount,
                0, // mint
                3, // source or recipient
                2, // authority
                4, // pool_account_index:
                0, // pool_index
                self.spl_interface_pda_bump,
                self.decimals,
            )),
            delegate_is_set: false,
            method_used: true,
        };

        let unwrap_to_destination = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::decompress(self.amount, 0, 1)),
            delegate_is_set: false,
            method_used: true,
        };

        let inputs = Transfer2Inputs {
            validity_proof: ValidityProof::new(None),
            transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
            meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
                self.payer,
                packed_accounts,
            ),
            in_lamports: None,
            out_lamports: None,
            token_accounts: vec![wrap_from_spl, unwrap_to_destination],
            output_queue: 0, // Decompressed accounts only, no output queue needed
            in_tlv: None,
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
