use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::{
    transfer_ctoken::TransferCtokenAccountInfos,
    transfer_ctoken_spl::TransferCtokenToSplAccountInfos,
    transfer_spl_ctoken::TransferSplToCtokenAccountInfos,
};
use crate::{error::TokenSdkError, utils::is_ctoken_account};

/// Required accounts to interface between ctoken and SPL token accounts.
pub struct SplInterface<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub token_pool_pda: AccountInfo<'info>,
    pub token_pool_pda_bump: u8,
}

pub struct TransferInterface<'info> {
    pub amount: u64,
    pub decimals: u8,
    pub source_account: AccountInfo<'info>,
    pub destination_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
    pub spl_interface: Option<SplInterface<'info>>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: AccountInfo<'info>,
}

impl<'info> TransferInterface<'info> {
    /// # Arguments
    /// * `amount` - Amount to transfer
    /// * `decimals` - Token decimals (required for SPL transfers)
    /// * `source_account` - Source token account (can be ctoken or SPL)
    /// * `destination_account` - Destination token account (can be ctoken or SPL)
    /// * `authority` - Authority for the transfer (must be signer)
    /// * `payer` - Payer for the transaction
    /// * `compressed_token_program_authority` - Compressed token program authority
    /// * `system_program` - System program (required for compressible account lamport top-ups)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        amount: u64,
        decimals: u8,
        source_account: AccountInfo<'info>,
        destination_account: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        payer: AccountInfo<'info>,
        compressed_token_program_authority: AccountInfo<'info>,
        system_program: AccountInfo<'info>,
    ) -> Self {
        Self {
            source_account,
            destination_account,
            authority,
            amount,
            decimals,
            payer,
            compressed_token_program_authority,
            spl_interface: None,
            system_program,
        }
    }

    /// # Arguments
    /// * `mint` - Optional mint account (required for SPL<->ctoken transfers)
    /// * `spl_token_program` - Optional SPL token program (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda` - Optional token pool PDA (required for SPL<->ctoken transfers)
    /// * `compressed_token_pool_pda_bump` - Optional bump seed for token pool PDA
    pub fn with_spl_interface(
        mut self,
        mint: Option<AccountInfo<'info>>,
        spl_token_program: Option<AccountInfo<'info>>,
        token_pool_pda: Option<AccountInfo<'info>>,
        token_pool_pda_bump: Option<u8>,
    ) -> Result<Self, ProgramError> {
        let mint =
            mint.ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingMintAccount.into()))?;

        let spl_token_program = spl_token_program
            .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingSplTokenProgram.into()))?;

        let token_pool_pda = token_pool_pda
            .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingTokenPoolPda.into()))?;

        let token_pool_pda_bump = token_pool_pda_bump
            .ok_or_else(|| ProgramError::Custom(TokenSdkError::MissingTokenPoolPdaBump.into()))?;

        self.spl_interface = Some(SplInterface {
            mint,
            spl_token_program,
            token_pool_pda,
            token_pool_pda_bump,
        });
        Ok(self)
    }

    /// # Errors
    /// * `SplInterfaceRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    /// * `CannotDetermineAccountType` - If account type cannot be determined
    pub fn invoke(self) -> Result<(), ProgramError> {
        let source_is_ctoken = is_ctoken_account(&self.source_account)
            .map_err(|_| ProgramError::Custom(TokenSdkError::CannotDetermineAccountType.into()))?;
        let dest_is_ctoken = is_ctoken_account(&self.destination_account)
            .map_err(|_| ProgramError::Custom(TokenSdkError::CannotDetermineAccountType.into()))?;

        match (source_is_ctoken, dest_is_ctoken) {
            (true, true) => TransferCtokenAccountInfos {
                source: self.source_account.clone(),
                destination: self.destination_account.clone(),
                amount: self.amount,
                authority: self.authority.clone(),
                max_top_up: None, // No limit by default
            }
            .invoke(),

            (true, false) => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::SplInterfaceRequired.into())
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
                    decimals: self.decimals,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke()
            }

            (false, true) => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::SplInterfaceRequired.into())
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
                    decimals: self.decimals,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                    system_program: self.system_program.clone(),
                }
                .invoke()
            }

            (false, false) => Err(ProgramError::Custom(
                TokenSdkError::UseRegularSplTransfer.into(),
            )),
        }
    }

    /// # Errors
    /// * `SplInterfaceRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    /// * `CannotDetermineAccountType` - If account type cannot be determined
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let source_is_ctoken = is_ctoken_account(&self.source_account)
            .map_err(|_| ProgramError::Custom(TokenSdkError::CannotDetermineAccountType.into()))?;
        let dest_is_ctoken = is_ctoken_account(&self.destination_account)
            .map_err(|_| ProgramError::Custom(TokenSdkError::CannotDetermineAccountType.into()))?;

        match (source_is_ctoken, dest_is_ctoken) {
            (true, true) => TransferCtokenAccountInfos {
                source: self.source_account.clone(),
                destination: self.destination_account.clone(),
                amount: self.amount,
                authority: self.authority.clone(),
                max_top_up: None, // No limit by default
            }
            .invoke_signed(signer_seeds),

            (true, false) => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::SplInterfaceRequired.into())
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
                    decimals: self.decimals,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                }
                .invoke_signed(signer_seeds)
            }

            (false, true) => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(TokenSdkError::SplInterfaceRequired.into())
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
                    decimals: self.decimals,
                    spl_token_program: config.spl_token_program.clone(),
                    compressed_token_program_authority: self
                        .compressed_token_program_authority
                        .clone(),
                    system_program: self.system_program.clone(),
                }
                .invoke_signed(signer_seeds)
            }

            (false, false) => Err(ProgramError::Custom(
                TokenSdkError::UseRegularSplTransfer.into(),
            )),
        }
    }
}
