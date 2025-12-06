use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_ctoken_interface::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
use light_program_profiler::profile;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compressed_token::{
    transfer2::{
        create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config, Transfer2Inputs,
    },
    CTokenAccount2,
};

/// # Create a transfer ctoken to SPL instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::TransferCtokenToSpl;
/// # let source_ctoken_account = Pubkey::new_unique();
/// # let destination_spl_token_account = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let spl_interface_pda = Pubkey::new_unique();
/// # let spl_token_program = Pubkey::new_unique();
/// let instruction = TransferCtokenToSpl {
///     source_ctoken_account,
///     destination_spl_token_account,
///     amount: 100,
///     authority,
///     mint,
///     payer,
///     spl_interface_pda,
///     spl_interface_pda_bump: 255,
///     spl_token_program,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferCtokenToSpl {
    pub source_ctoken_account: Pubkey,
    pub destination_spl_token_account: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub spl_interface_pda: Pubkey,
    pub spl_interface_pda_bump: u8,
    pub spl_token_program: Pubkey,
}

/// # Transfer ctoken to SPL via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::TransferCtokenToSplCpi;
/// # use solana_account_info::AccountInfo;
/// # let source_ctoken_account: AccountInfo = todo!();
/// # let destination_spl_token_account: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let spl_interface_pda: AccountInfo = todo!();
/// # let spl_token_program: AccountInfo = todo!();
/// # let compressed_token_program_authority: AccountInfo = todo!();
/// TransferCtokenToSplCpi {
///     source_ctoken_account,
///     destination_spl_token_account,
///     amount: 100,
///     authority,
///     mint,
///     payer,
///     spl_interface_pda,
///     spl_interface_pda_bump: 255,
///     spl_token_program,
///     compressed_token_program_authority,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferCtokenToSplCpi<'info> {
    pub source_ctoken_account: AccountInfo<'info>,
    pub destination_spl_token_account: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_interface_pda_bump: u8,
    pub spl_token_program: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
}

impl<'info> TransferCtokenToSplCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferCtokenToSpl::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferCtokenToSpl::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.source_ctoken_account,              // Index 1: Source ctoken account
            self.destination_spl_token_account,      // Index 2: Destination SPL token account
            self.authority,                          // Index 3: Authority (signer)
            self.spl_interface_pda,                  // Index 4: SPL interface PDA
            self.spl_token_program,                  // Index 5: SPL Token program
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferCtokenToSpl::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.source_ctoken_account,              // Index 1: Source ctoken account
            self.destination_spl_token_account,      // Index 2: Destination SPL token account
            self.authority,                          // Index 3: Authority (signer)
            self.spl_interface_pda,                  // Index 4: SPL interface PDA
            self.spl_token_program,                  // Index 5: SPL Token program
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCtokenToSplCpi<'info>> for TransferCtokenToSpl {
    fn from(account_infos: &TransferCtokenToSplCpi<'info>) -> Self {
        Self {
            source_ctoken_account: *account_infos.source_ctoken_account.key,
            destination_spl_token_account: *account_infos.destination_spl_token_account.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            mint: *account_infos.mint.key,
            payer: *account_infos.payer.key,
            spl_interface_pda: *account_infos.spl_interface_pda.key,
            spl_interface_pda_bump: account_infos.spl_interface_pda_bump,
            spl_token_program: *account_infos.spl_token_program.key,
        }
    }
}

impl TransferCtokenToSpl {
    #[profile]
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let packed_accounts = vec![
            // Mint (index 0)
            AccountMeta::new_readonly(self.mint, false),
            // Source ctoken account (index 1) - writable
            AccountMeta::new(self.source_ctoken_account, false),
            // Destination SPL token account (index 2) - writable
            AccountMeta::new(self.destination_spl_token_account, false),
            // Authority (index 3) - signer
            AccountMeta::new_readonly(self.authority, true),
            // SPL interface PDA (index 4) - writable
            AccountMeta::new(self.spl_interface_pda, false),
            // SPL Token program (index 5) - needed for CPI
            AccountMeta::new_readonly(self.spl_token_program, false),
        ];

        // First operation: compress from ctoken account to pool using compress_spl
        let compress_to_pool = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::compress_ctoken(
                self.amount,
                0, // mint index
                1, // source ctoken account index
                3, // authority index
            )),
            delegate_is_set: false,
            method_used: true,
        };

        // Second operation: decompress from pool to SPL token account using decompress_spl
        let decompress_to_spl = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::decompress_spl(
                self.amount,
                0, // mint index
                2, // destination SPL token account index
                4, // pool_account_index
                0, // pool_index (TODO: make dynamic)
                self.spl_interface_pda_bump,
            )),
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
            token_accounts: vec![compress_to_pool, decompress_to_spl],
            output_queue: 0, // Decompressed accounts only, no output queue needed
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
