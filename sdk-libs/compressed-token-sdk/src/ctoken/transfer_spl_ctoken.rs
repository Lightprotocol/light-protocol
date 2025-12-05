use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_ctoken_types::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
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

pub struct TransferSplToCtoken {
    pub amount: u64,
    pub token_pool_pda_bump: u8,
    pub decimals: u8,
    pub source_spl_token_account: Pubkey,
    /// Destination ctoken account (writable)
    pub destination_ctoken_account: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub token_pool_pda: Pubkey,
    pub spl_token_program: Pubkey,
}

pub struct TransferSplToCtokenAccountInfos<'info> {
    pub amount: u64,
    pub token_pool_pda_bump: u8,
    pub decimals: u8,
    pub source_spl_token_account: AccountInfo<'info>,
    /// Destination ctoken account (writable)
    pub destination_ctoken_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub token_pool_pda: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: AccountInfo<'info>,
}

impl<'info> TransferSplToCtokenAccountInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferSplToCtoken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferSplToCtoken::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.destination_ctoken_account,         // Index 1: Destination ctoken account
            self.authority,                          // Index 2: Authority (signer)
            self.source_spl_token_account,           // Index 3: Source SPL token account
            self.token_pool_pda,                     // Index 4: Token pool PDA
            self.spl_token_program,                  // Index 5: SPL Token program
            self.system_program,                     // Index 6: System program
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferSplToCtoken::from(&self).instruction()?;
        // Account order must match instruction metas: cpi_authority_pda, fee_payer, packed_accounts...
        let account_infos = [
            self.compressed_token_program_authority, // CPI authority PDA (first)
            self.payer,                              // Fee payer (second)
            self.mint,                               // Index 0: Mint
            self.destination_ctoken_account,         // Index 1: Destination ctoken account
            self.authority,                          // Index 2: Authority (signer)
            self.source_spl_token_account,           // Index 3: Source SPL token account
            self.token_pool_pda,                     // Index 4: Token pool PDA
            self.spl_token_program,                  // Index 5: SPL Token program
            self.system_program,                     // Index 6: System program
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferSplToCtokenAccountInfos<'info>> for TransferSplToCtoken {
    fn from(account_infos: &TransferSplToCtokenAccountInfos<'info>) -> Self {
        Self {
            source_spl_token_account: *account_infos.source_spl_token_account.key,
            destination_ctoken_account: *account_infos.destination_ctoken_account.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            mint: *account_infos.mint.key,
            payer: *account_infos.payer.key,
            token_pool_pda: *account_infos.token_pool_pda.key,
            token_pool_pda_bump: account_infos.token_pool_pda_bump,
            decimals: account_infos.decimals,
            spl_token_program: *account_infos.spl_token_program.key,
        }
    }
}

impl TransferSplToCtoken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let packed_accounts = vec![
            // Mint (index 0)
            AccountMeta::new_readonly(self.mint, false),
            // Destination ctoken account (index 1) - writable
            AccountMeta::new(self.destination_ctoken_account, false),
            // Authority for compression (index 2) - signer
            AccountMeta::new_readonly(self.authority, true),
            // Source SPL token account (index 3) - writable
            AccountMeta::new(self.source_spl_token_account, false),
            // Token pool PDA (index 4) - writable
            AccountMeta::new(self.token_pool_pda, false),
            // SPL Token program (index 5) - needed for CPI
            AccountMeta::new_readonly(self.spl_token_program, false),
            // System program (index 6) - needed for compressible account lamport top-ups
            AccountMeta::new_readonly(Pubkey::default(), false),
        ];

        let wrap_spl_to_ctoken_account = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::compress_spl(
                self.amount,
                0, // mint
                3, // source or recipient
                2, // authority
                4, // pool_account_index:
                0, // pool_index
                self.token_pool_pda_bump,
                self.decimals,
            )),
            delegate_is_set: false,
            method_used: true,
        };

        let ctoken_account = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::decompress_ctoken(self.amount, 0, 1)),
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
            token_accounts: vec![wrap_spl_to_ctoken_account, ctoken_account],
            output_queue: 0, // Decompressed accounts only, no output queue needed
            in_tlv: None,
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
