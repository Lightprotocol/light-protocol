use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_ctoken_types::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
use light_program_profiler::profile;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::transfer_ctoken::TransferCtokenAccountInfos;
use crate::{
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    error::TokenSdkError,
    utils::is_ctoken_account,
};

pub struct TransferSplToCtoken {
    pub amount: u64,
    pub token_pool_pda_bump: u8,
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
    pub source_spl_token_account: AccountInfo<'info>,
    /// Destination ctoken account (writable)
    pub destination_ctoken_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub token_pool_pda: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
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
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}

pub struct TransferCtokenToSpl {
    pub source_ctoken_account: Pubkey,
    pub destination_spl_token_account: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub token_pool_pda: Pubkey,
    pub token_pool_pda_bump: u8,
    pub spl_token_program: Pubkey,
}

pub struct TransferCtokenToSplAccountInfos<'info> {
    pub source_ctoken_account: AccountInfo<'info>,
    pub destination_spl_token_account: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub token_pool_pda: AccountInfo<'info>,
    pub token_pool_pda_bump: u8,
    pub spl_token_program: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
}

impl<'info> TransferCtokenToSplAccountInfos<'info> {
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
            self.token_pool_pda,                     // Index 4: Token pool PDA
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
            self.token_pool_pda,                     // Index 4: Token pool PDA
            self.spl_token_program,                  // Index 5: SPL Token program
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCtokenToSplAccountInfos<'info>> for TransferCtokenToSpl {
    fn from(account_infos: &TransferCtokenToSplAccountInfos<'info>) -> Self {
        Self {
            source_ctoken_account: *account_infos.source_ctoken_account.key,
            destination_spl_token_account: *account_infos.destination_spl_token_account.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            mint: *account_infos.mint.key,
            payer: *account_infos.payer.key,
            token_pool_pda: *account_infos.token_pool_pda.key,
            token_pool_pda_bump: account_infos.token_pool_pda_bump,
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
            // Token pool PDA (index 4) - writable
            AccountMeta::new(self.token_pool_pda, false),
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
                self.token_pool_pda_bump,
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

pub struct SplBridgeConfig<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub token_pool_pda: AccountInfo<'info>,
    pub token_pool_pda_bump: u8,
}

pub struct TransferInterface<'info> {
    pub source_account: AccountInfo<'info>,
    pub destination_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
    pub amount: u64,
    pub spl_bridge_config: Option<SplBridgeConfig<'info>>,
}

impl<'info> TransferInterface<'info> {
    /// # Arguments
    /// * `source_account` - Source token account (can be ctoken or SPL)
    /// * `destination_account` - Destination token account (can be ctoken or SPL)
    /// * `authority` - Authority for the transfer (must be signer)
    /// * `amount` - Amount to transfer
    /// * `payer` - Payer for the transaction
    /// * `compressed_token_program_authority` - Compressed token program authority
    /// * `mint` - Optional mint account (required for SPL<->ctoken transfers)
    /// * `spl_token_program` - Optional SPL token program (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda` - Optional token pool PDA (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda_bump` - Optional bump seed for token pool PDA
    pub fn new(
        source_account: AccountInfo<'info>,
        destination_account: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        amount: u64,
        payer: AccountInfo<'info>,
        compressed_token_program_authority: AccountInfo<'info>,
    ) -> Self {
        Self {
            source_account,
            destination_account,
            authority,
            amount,
            payer,
            compressed_token_program_authority,
            spl_bridge_config: None,
        }
    }

    /// # Arguments
    /// * `mint` - mint account (required for SPL<->ctoken transfers)
    /// * `spl_token_program` - SPL token program (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda` - token pool PDA (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda_bump` - bump seed for token pool PDA
    pub fn with_spl_bridge(
        mut self,
        mint: AccountInfo<'info>,
        spl_token_program: AccountInfo<'info>,
        token_pool_pda: AccountInfo<'info>,
        token_pool_pda_bump: u8,
    ) -> Self {
        self.spl_bridge_config = Some(SplBridgeConfig {
            mint,
            spl_token_program,
            token_pool_pda,
            token_pool_pda_bump,
        });
        self
    }

    pub fn with_spl_bridge_config(mut self, config: Option<SplBridgeConfig<'info>>) -> Self {
        self.spl_bridge_config = config;
        self
    }

    /// # Errors
    /// * `SplBridgeConfigRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    /// * `CannotDetermineAccountType` - If account type cannot be determined
    pub fn invoke(self) -> Result<(), ProgramError> {
        let source_is_ctoken = is_ctoken_account(&self.source_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let dest_is_ctoken = is_ctoken_account(&self.destination_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        match (source_is_ctoken, dest_is_ctoken) {
            (true, true) => TransferCtokenAccountInfos {
                source: self.source_account.clone(),
                destination: self.destination_account.clone(),
                amount: self.amount,
                authority: self.authority.clone(),
            }
            .invoke(),

            (true, false) => {
                let config = self.spl_bridge_config.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::IncompleteSplBridgeConfig.into())
                })?;

                TransferCtokenToSplAccountInfos {
                    source_ctoken_account: self.source_account.clone(),
                    destination_spl_token_account: self.destination_account.clone(),
                    amount: self.amount,
                    authority: self.authority.clone(),
                    mint: config.mint.clone(),
                    payer: self.payer.clone(),
                    token_pool_pda: config.token_pool_pda.clone(),
                    token_pool_pda_bump: config.token_pool_pda_bump,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke()
            }

            (false, true) => {
                let config = self.spl_bridge_config.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::IncompleteSplBridgeConfig.into())
                })?;

                TransferSplToCtokenAccountInfos {
                    source_spl_token_account: self.source_account.clone(),
                    destination_ctoken_account: self.destination_account.clone(),
                    amount: self.amount,
                    authority: self.authority.clone(),
                    mint: config.mint.clone(),
                    payer: self.payer.clone(),
                    token_pool_pda: config.token_pool_pda.clone(),
                    token_pool_pda_bump: config.token_pool_pda_bump,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke()
            }

            (false, false) => Err(ProgramError::Custom(
                TokenSdkError::UseRegularSplTransfer.into(),
            )),
        }
    }

    /// # Errors
    /// * `SplBridgeConfigRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    /// * `CannotDetermineAccountType` - If account type cannot be determined
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let source_is_ctoken = is_ctoken_account(&self.source_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let dest_is_ctoken = is_ctoken_account(&self.destination_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        match (source_is_ctoken, dest_is_ctoken) {
            (true, true) => TransferCtokenAccountInfos {
                source: self.source_account.clone(),
                destination: self.destination_account.clone(),
                amount: self.amount,
                authority: self.authority.clone(),
            }
            .invoke_signed(signer_seeds),

            (true, false) => {
                let config = self.spl_bridge_config.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::IncompleteSplBridgeConfig.into())
                })?;

                TransferCtokenToSplAccountInfos {
                    source_ctoken_account: self.source_account.clone(),
                    destination_spl_token_account: self.destination_account.clone(),
                    amount: self.amount,
                    authority: self.authority.clone(),
                    mint: config.mint.clone(),
                    payer: self.payer.clone(),
                    token_pool_pda: config.token_pool_pda.clone(),
                    token_pool_pda_bump: config.token_pool_pda_bump,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke_signed(signer_seeds)
            }

            (false, true) => {
                let config = self.spl_bridge_config.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::IncompleteSplBridgeConfig.into())
                })?;

                TransferSplToCtokenAccountInfos {
                    source_spl_token_account: self.source_account.clone(),
                    destination_ctoken_account: self.destination_account.clone(),
                    amount: self.amount,
                    authority: self.authority.clone(),
                    mint: config.mint.clone(),
                    payer: self.payer.clone(),
                    token_pool_pda: config.token_pool_pda.clone(),
                    token_pool_pda_bump: config.token_pool_pda_bump,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke_signed(signer_seeds)
            }

            (false, false) => Err(ProgramError::Custom(
                TokenSdkError::UseRegularSplTransfer.into(),
            )),
        }
    }
}
