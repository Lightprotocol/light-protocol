use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::{
    transfer::Transfer, transfer_from_spl::TransferFromSpl, transfer_to_spl::TransferToSpl,
};
use crate::error::LightTokenError;

/// Internal enum to classify transfer types based on account owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferType {
    /// light -> light
    LightToLight,
    /// light -> SPL (decompress)
    LightToSpl,
    /// SPL -> light (compress)
    SplToLight,
    /// SPL -> SPL (pass-through to SPL token program)
    SplToSpl,
}

/// Determine transfer type from account owners.
///
/// Returns `Ok(TransferType)` if at least one account is a Light token account.
/// Returns `Err(UseRegularSplTransfer)` if both accounts are non-Light (SPL) accounts.
/// Returns `Err(CannotDetermineAccountType)` if an account owner is unrecognized.
fn determine_transfer_type(
    source_owner: &Pubkey,
    destination_owner: &Pubkey,
) -> Result<TransferType, ProgramError> {
    use crate::utils::is_light_token_owner;

    let source_is_light = is_light_token_owner(source_owner)
        .map_err(|_| ProgramError::Custom(LightTokenError::CannotDetermineAccountType.into()))?;
    let dest_is_light = is_light_token_owner(destination_owner)
        .map_err(|_| ProgramError::Custom(LightTokenError::CannotDetermineAccountType.into()))?;

    match (source_is_light, dest_is_light) {
        (true, true) => Ok(TransferType::LightToLight),
        (true, false) => Ok(TransferType::LightToSpl),
        (false, true) => Ok(TransferType::SplToLight),
        (false, false) => {
            // Both are SPL - verify same token program
            if source_owner == destination_owner {
                Ok(TransferType::SplToSpl)
            } else {
                Err(ProgramError::Custom(
                    LightTokenError::SplTokenProgramMismatch.into(),
                ))
            }
        }
    }
}

/// Required accounts to interface between light and SPL token accounts (Pubkey-based).
///
/// Use this struct when building instructions outside of CPI context.
#[derive(Debug, Clone, Copy)]
pub struct SplInterface {
    pub mint: Pubkey,
    pub spl_token_program: Pubkey,
    pub spl_interface_pda: Pubkey,
    pub spl_interface_pda_bump: u8,
}

impl<'info> From<&SplInterfaceCpi<'info>> for SplInterface {
    fn from(spl: &SplInterfaceCpi<'info>) -> Self {
        Self {
            mint: *spl.mint.key,
            spl_token_program: *spl.spl_token_program.key,
            spl_interface_pda: *spl.spl_interface_pda.key,
            spl_interface_pda_bump: spl.spl_interface_pda_bump,
        }
    }
}

/// Required accounts to interface between light and SPL token accounts (AccountInfo-based).
///
/// Use this struct when building CPIs.
pub struct SplInterfaceCpi<'info> {
    pub mint: AccountInfo<'info>,
    pub spl_token_program: AccountInfo<'info>,
    pub spl_interface_pda: AccountInfo<'info>,
    pub spl_interface_pda_bump: u8,
}

/// # Create a transfer interface instruction that auto-routes based on account types:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::{TransferInterface, SplInterface, LIGHT_TOKEN_PROGRAM_ID};
/// # let source = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// // For light -> light transfer (source_owner and destination_owner are LIGHT_TOKEN_PROGRAM_ID)
/// let instruction = TransferInterface {
///     source,
///     destination,
///     amount: 100,
///     decimals: 9,
///     authority,
///     payer,
///     spl_interface: None,
///     max_top_up: None,
///     source_owner: LIGHT_TOKEN_PROGRAM_ID,
///     destination_owner: LIGHT_TOKEN_PROGRAM_ID,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferInterface {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub spl_interface: Option<SplInterface>,
    /// Maximum lamports for rent and top-up combined (for light->light transfers)
    pub max_top_up: Option<u16>,
    /// Owner of the source account (used to determine transfer type)
    pub source_owner: Pubkey,
    /// Owner of the destination account (used to determine transfer type)
    pub destination_owner: Pubkey,
}

impl TransferInterface {
    /// Build instruction based on detected transfer type
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        match determine_transfer_type(&self.source_owner, &self.destination_owner)? {
            TransferType::LightToLight => Transfer {
                source: self.source,
                destination: self.destination,
                amount: self.amount,
                authority: self.authority,
                max_top_up: self.max_top_up,
                fee_payer: None,
            }
            .instruction(),

            TransferType::LightToSpl => {
                let spl = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                TransferToSpl {
                    source: self.source,
                    destination_spl_token_account: self.destination,
                    amount: self.amount,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    spl_token_program: spl.spl_token_program,
                }
                .instruction()
            }

            TransferType::SplToLight => {
                let spl = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                TransferFromSpl {
                    source_spl_token_account: self.source,
                    destination: self.destination,
                    amount: self.amount,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    spl_token_program: spl.spl_token_program,
                }
                .instruction()
            }

            TransferType::SplToSpl => {
                let spl = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;

                // Build SPL transfer_checked instruction manually
                // Discriminator 12 = TransferChecked
                let mut data = vec![12u8];
                data.extend_from_slice(&self.amount.to_le_bytes());
                data.push(self.decimals);

                Ok(Instruction {
                    program_id: self.source_owner, // SPL token program
                    accounts: vec![
                        AccountMeta::new(self.source, false),
                        AccountMeta::new_readonly(spl.mint, false),
                        AccountMeta::new(self.destination, false),
                        AccountMeta::new_readonly(self.authority, true),
                    ],
                    data,
                })
            }
        }
    }
}

impl<'info> From<&TransferInterfaceCpi<'info>> for TransferInterface {
    fn from(cpi: &TransferInterfaceCpi<'info>) -> Self {
        Self {
            source: *cpi.source_account.key,
            destination: *cpi.destination_account.key,
            amount: cpi.amount,
            decimals: cpi.decimals,
            authority: *cpi.authority.key,
            payer: *cpi.payer.key,
            spl_interface: cpi.spl_interface.as_ref().map(SplInterface::from),
            max_top_up: None,
            source_owner: *cpi.source_account.owner,
            destination_owner: *cpi.destination_account.owner,
        }
    }
}

/// # Transfer interface via CPI (auto-detects account types):
/// ```rust,no_run
/// # use light_token::instruction::{TransferInterfaceCpi, SplInterfaceCpi};
/// # use solana_account_info::AccountInfo;
/// # let source_account: AccountInfo = todo!();
/// # let destination_account: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let compressed_token_program_authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// TransferInterfaceCpi::new(
///     100,    // amount
///     9,      // decimals
///     source_account,
///     destination_account,
///     authority,
///     payer,
///     compressed_token_program_authority,
///     system_program,
/// )
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferInterfaceCpi<'info> {
    pub amount: u64,
    pub decimals: u8,
    pub source_account: AccountInfo<'info>,
    pub destination_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub compressed_token_program_authority: AccountInfo<'info>,
    pub spl_interface: Option<SplInterfaceCpi<'info>>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: AccountInfo<'info>,
}

impl<'info> TransferInterfaceCpi<'info> {
    /// # Arguments
    /// * `amount` - Amount to transfer
    /// * `decimals` - Token decimals (required for SPL transfers)
    /// * `source_account` - Source token account (can be light or SPL)
    /// * `destination_account` - Destination token account (can be light or SPL)
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
    /// * `mint` - Optional mint account (required for SPL<->light transfers)
    /// * `spl_token_program` - Optional SPL token program (required for SPL<->light transfers)
    /// * `spl_interface_pda` - Optional SPL interface PDA (required for SPL<->light transfers)
    /// * `spl_interface_pda_bump` - Optional bump seed for SPL interface PDA
    pub fn with_spl_interface(
        mut self,
        mint: Option<AccountInfo<'info>>,
        spl_token_program: Option<AccountInfo<'info>>,
        spl_interface_pda: Option<AccountInfo<'info>>,
        spl_interface_pda_bump: Option<u8>,
    ) -> Result<Self, ProgramError> {
        let mint =
            mint.ok_or_else(|| ProgramError::Custom(LightTokenError::MissingMintAccount.into()))?;

        let spl_token_program = spl_token_program
            .ok_or_else(|| ProgramError::Custom(LightTokenError::MissingSplTokenProgram.into()))?;

        let spl_interface_pda = spl_interface_pda
            .ok_or_else(|| ProgramError::Custom(LightTokenError::MissingSplInterfacePda.into()))?;

        let spl_interface_pda_bump = spl_interface_pda_bump.ok_or_else(|| {
            ProgramError::Custom(LightTokenError::MissingSplInterfacePdaBump.into())
        })?;

        self.spl_interface = Some(SplInterfaceCpi {
            mint,
            spl_token_program,
            spl_interface_pda,
            spl_interface_pda_bump,
        });
        Ok(self)
    }

    /// Build instruction from CPI context
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferInterface::from(self).instruction()
    }

    /// # Errors
    /// * `SplInterfaceRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    pub fn invoke(self) -> Result<(), ProgramError> {
        use solana_cpi::invoke;

        let transfer_type =
            determine_transfer_type(self.source_account.owner, self.destination_account.owner)?;
        let instruction = self.instruction()?;

        match transfer_type {
            TransferType::LightToLight => {
                let account_infos = [
                    self.source_account,
                    self.destination_account,
                    self.authority,
                ];
                invoke(&instruction, &account_infos)
            }

            TransferType::LightToSpl => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.compressed_token_program_authority,
                    self.payer,
                    config.mint,
                    self.source_account,
                    self.destination_account,
                    self.authority,
                    config.spl_interface_pda,
                    config.spl_token_program,
                ];
                invoke(&instruction, &account_infos)
            }

            TransferType::SplToLight => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.compressed_token_program_authority,
                    self.payer,
                    config.mint,
                    self.destination_account,
                    self.authority,
                    self.source_account,
                    config.spl_interface_pda,
                    config.spl_token_program,
                    self.system_program,
                ];
                invoke(&instruction, &account_infos)
            }

            TransferType::SplToSpl => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.source_account,
                    config.mint,
                    self.destination_account,
                    self.authority,
                ];
                invoke(&instruction, &account_infos)
            }
        }
    }

    /// # Errors
    /// * `SplInterfaceRequired` - If transferring to/from SPL without required accounts
    /// * `UseRegularSplTransfer` - If both source and destination are SPL accounts
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        use solana_cpi::invoke_signed;

        let transfer_type =
            determine_transfer_type(self.source_account.owner, self.destination_account.owner)?;
        let instruction = self.instruction()?;

        match transfer_type {
            TransferType::LightToLight => {
                let account_infos = [
                    self.source_account,
                    self.destination_account,
                    self.authority,
                ];
                invoke_signed(&instruction, &account_infos, signer_seeds)
            }

            TransferType::LightToSpl => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.compressed_token_program_authority,
                    self.payer,
                    config.mint,
                    self.source_account,
                    self.destination_account,
                    self.authority,
                    config.spl_interface_pda,
                    config.spl_token_program,
                ];
                invoke_signed(&instruction, &account_infos, signer_seeds)
            }

            TransferType::SplToLight => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.compressed_token_program_authority,
                    self.payer,
                    config.mint,
                    self.destination_account,
                    self.authority,
                    self.source_account,
                    config.spl_interface_pda,
                    config.spl_token_program,
                    self.system_program,
                ];
                invoke_signed(&instruction, &account_infos, signer_seeds)
            }

            TransferType::SplToSpl => {
                let config = self.spl_interface.ok_or_else(|| {
                    ProgramError::Custom(LightTokenError::SplInterfaceRequired.into())
                })?;
                let account_infos = [
                    self.source_account,
                    config.mint,
                    self.destination_account,
                    self.authority,
                ];
                invoke_signed(&instruction, &account_infos, signer_seeds)
            }
        }
    }
}
