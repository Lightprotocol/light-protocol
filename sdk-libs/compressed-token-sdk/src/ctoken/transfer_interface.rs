use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::transfer_ctoken::TransferCtokenAccountInfos;
use super::transfer_ctoken_spl::TransferCtokenToSplAccountInfos;
use super::transfer_spl_ctoken::TransferSplToCtokenAccountInfos;
use crate::{error::TokenSdkError, utils::is_ctoken_account};

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

    // pub fn with_spl_source_optional(
    //     mut self,
    //     mint: Option<AccountInfo<'info>>,
    //     spl_token_program: Option<AccountInfo<'info>>,
    //     token_pool_pda: Option<AccountInfo<'info>>,
    //     token_pool_pda_bump: Option<u8>,
    // ) -> Self {
    // TODO: add errors
    // TODO: check that source is owned by the program
    //     let mint = mint
    //         .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingMintAccount.into()))
    //         .unwrap();

    //     let spl_token_program = spl_token_program
    //         .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingSplTokenProgram.into()))
    //         .unwrap();

    //     let token_pool_pda = token_pool_pda
    //         .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingTokenPoolPda.into()))
    //         .unwrap();

    //     let token_pool_pda_bump = token_pool_pda_bump
    //         .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingTokenPoolPdaBump.into()))
    //         .unwrap();

    //     self.spl_bridge_config = Some(SplBridgeConfig {
    //         mint,
    //         spl_token_program,
    //         token_pool_pda,
    //         token_pool_pda_bump,
    //     });
    //     self
    // }

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
