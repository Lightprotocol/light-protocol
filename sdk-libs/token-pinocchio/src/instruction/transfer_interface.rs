//! Unified transfer interface that auto-routes based on account types.

use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use pinocchio::{
    AccountView as AccountInfo,
    cpi::{invoke, invoke_signed_with_slice, Signer},
    instruction::{InstructionAccount, InstructionView},
    error::ProgramError,
};

use super::{
    transfer_checked::TransferCheckedCpi, transfer_from_spl::TransferFromSplCpi,
    transfer_to_spl::TransferToSplCpi,
};
use crate::error::LightTokenError;

/// SPL Token transfer_checked instruction discriminator
const SPL_TRANSFER_CHECKED_DISCRIMINATOR: u8 = 12;

/// SPL Token Program ID
const SPL_TOKEN_PROGRAM_ID: [u8; 32] =
    light_macros::pubkey_array!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Token 2022 Program ID
const SPL_TOKEN_2022_PROGRAM_ID: [u8; 32] =
    light_macros::pubkey_array!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Check if an account is owned by the Light Token program.
///
/// Returns `Ok(true)` for Light Token program, `Ok(false)` for SPL Token / Token-2022,
/// and `Err(CannotDetermineAccountType)` for unrecognized owners.
fn is_light_token_owner(owner: &[u8; 32]) -> Result<bool, LightTokenError> {
    if owner == &LIGHT_TOKEN_PROGRAM_ID {
        return Ok(true);
    }

    if owner == &SPL_TOKEN_PROGRAM_ID || owner == &SPL_TOKEN_2022_PROGRAM_ID {
        return Ok(false);
    }

    Err(LightTokenError::CannotDetermineAccountType)
}

/// Internal enum to classify transfer types based on account owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferType {
    /// light -> light
    LightToLight,
    /// light -> SPL (decompress)
    LightToSpl,
    /// SPL -> light (compress)
    SplToLight,
    /// SPL -> SPL (pass-through)
    SplToSpl,
}

/// Determine transfer type from account owners.
///
/// Returns `Ok(TransferType)` for valid account combinations.
/// Returns `Err(CannotDetermineAccountType)` if an account owner is unrecognized.
/// Returns `Err(SplTokenProgramMismatch)` if both are SPL but with different token programs.
fn determine_transfer_type(
    source_owner: &[u8; 32],
    destination_owner: &[u8; 32],
) -> Result<TransferType, ProgramError> {
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

/// Required accounts to interface between light and SPL token accounts.
pub struct SplInterfaceCpi<'info> {
    pub mint: &'info AccountInfo,
    pub spl_token_program: &'info AccountInfo,
    pub spl_interface_pda: &'info AccountInfo,
    pub spl_interface_pda_bump: u8,
}

/// Unified transfer interface that auto-routes based on account types.
///
/// Detects whether source and destination are Light token accounts or SPL token
/// accounts and routes to the appropriate transfer implementation.
///
/// # Example
/// ```rust,ignore
/// TransferInterfaceCpi::new(
///     100,    // amount
///     9,      // decimals
///     &source_account,
///     &destination_account,
///     &authority,
///     &payer,
///     &compressed_token_program_authority,
///     &mint,
///     &system_program,
/// )
/// .invoke()?;
/// ```
pub struct TransferInterfaceCpi<'info> {
    pub amount: u64,
    pub decimals: u8,
    pub source_account: &'info AccountInfo,
    pub destination_account: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub payer: &'info AccountInfo,
    pub compressed_token_program_authority: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub spl_interface: Option<SplInterfaceCpi<'info>>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: &'info AccountInfo,
}

impl<'info> TransferInterfaceCpi<'info> {
    /// Create a new TransferInterfaceCpi.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        amount: u64,
        decimals: u8,
        source_account: &'info AccountInfo,
        destination_account: &'info AccountInfo,
        authority: &'info AccountInfo,
        payer: &'info AccountInfo,
        compressed_token_program_authority: &'info AccountInfo,
        mint: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> Self {
        Self {
            amount,
            decimals,
            source_account,
            destination_account,
            authority,
            payer,
            compressed_token_program_authority,
            mint,
            spl_interface: None,
            system_program,
        }
    }

    /// Add SPL interface accounts (required for SPL<->light transfers).
    pub fn with_spl_interface(mut self, spl_interface: SplInterfaceCpi<'info>) -> Self {
        self.spl_interface = Some(spl_interface);
        self
    }

    /// Invoke the appropriate transfer based on account types.
    pub fn invoke(self) -> Result<(), ProgramError> {
        let source_owner = unsafe { self.source_account.owner() };
        let destination_owner = unsafe { self.destination_account.owner() };
        let transfer_type = determine_transfer_type(
            source_owner.as_array(),
            destination_owner.as_array(),
        )?;

        match transfer_type {
            TransferType::LightToLight => TransferCheckedCpi {
                source: self.source_account,
                mint: self.mint,
                destination: self.destination_account,
                amount: self.amount,
                decimals: self.decimals,
                authority: self.authority,
                system_program: self.system_program,
                fee_payer: self.payer,
            }
            .invoke(),

            TransferType::LightToSpl => {
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;
                TransferToSplCpi {
                    source: self.source_account,
                    destination_spl_token_account: self.destination_account,
                    amount: self.amount,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    spl_token_program: spl.spl_token_program,
                    compressed_token_program_authority: self.compressed_token_program_authority,
                }
                .invoke()
            }

            TransferType::SplToLight => {
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;
                TransferFromSplCpi {
                    amount: self.amount,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    source_spl_token_account: self.source_account,
                    destination: self.destination_account,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_token_program: spl.spl_token_program,
                    compressed_token_program_authority: self.compressed_token_program_authority,
                    system_program: self.system_program,
                }
                .invoke()
            }

            TransferType::SplToSpl => {
                // For SPL-to-SPL, invoke SPL token program directly via transfer_checked
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;

                // Build SPL transfer_checked instruction data: [12, amount(8), decimals(1)]
                let mut ix_data = [0u8; 10];
                ix_data[0] = SPL_TRANSFER_CHECKED_DISCRIMINATOR;
                ix_data[1..9].copy_from_slice(&self.amount.to_le_bytes());
                ix_data[9] = self.decimals;

                // Account order for SPL transfer_checked:
                // [0] source (writable)
                // [1] mint (readonly)
                // [2] destination (writable)
                // [3] authority (signer)
                let account_metas = [
                    InstructionAccount::writable(self.source_account.address()),
                    InstructionAccount::readonly(spl.mint.address()),
                    InstructionAccount::writable(self.destination_account.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ];

                // SPL token program ID from source account owner (Pubkey = [u8; 32])
                let source_owner = unsafe { self.source_account.owner() };
                let instruction = InstructionView {
                    program_id: source_owner,
                    accounts: &account_metas,
                    data: &ix_data,
                };

                let account_infos = [
                    self.source_account,
                    spl.mint,
                    self.destination_account,
                    self.authority,
                ];

                invoke(&instruction, &account_infos)
            }
        }
    }

    /// Invoke with signer seeds.
    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let source_owner = unsafe { self.source_account.owner() };
        let destination_owner = unsafe { self.destination_account.owner() };
        let transfer_type = determine_transfer_type(
            source_owner.as_array(),
            destination_owner.as_array(),
        )?;

        match transfer_type {
            TransferType::LightToLight => TransferCheckedCpi {
                source: self.source_account,
                mint: self.mint,
                destination: self.destination_account,
                amount: self.amount,
                decimals: self.decimals,
                authority: self.authority,
                system_program: self.system_program,
                fee_payer: self.payer,
            }
            .invoke_signed(signers),

            TransferType::LightToSpl => {
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;
                TransferToSplCpi {
                    source: self.source_account,
                    destination_spl_token_account: self.destination_account,
                    amount: self.amount,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    spl_token_program: spl.spl_token_program,
                    compressed_token_program_authority: self.compressed_token_program_authority,
                }
                .invoke_signed(signers)
            }

            TransferType::SplToLight => {
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;
                TransferFromSplCpi {
                    amount: self.amount,
                    spl_interface_pda_bump: spl.spl_interface_pda_bump,
                    decimals: self.decimals,
                    source_spl_token_account: self.source_account,
                    destination: self.destination_account,
                    authority: self.authority,
                    mint: spl.mint,
                    payer: self.payer,
                    spl_interface_pda: spl.spl_interface_pda,
                    spl_token_program: spl.spl_token_program,
                    compressed_token_program_authority: self.compressed_token_program_authority,
                    system_program: self.system_program,
                }
                .invoke_signed(signers)
            }

            TransferType::SplToSpl => {
                // For SPL-to-SPL, invoke SPL token program directly via transfer_checked
                let spl = self.spl_interface.ok_or(ProgramError::Custom(
                    LightTokenError::SplInterfaceRequired.into(),
                ))?;

                // Build SPL transfer_checked instruction data: [12, amount(8), decimals(1)]
                let mut ix_data = [0u8; 10];
                ix_data[0] = SPL_TRANSFER_CHECKED_DISCRIMINATOR;
                ix_data[1..9].copy_from_slice(&self.amount.to_le_bytes());
                ix_data[9] = self.decimals;

                // Account order for SPL transfer_checked:
                // [0] source (writable)
                // [1] mint (readonly)
                // [2] destination (writable)
                // [3] authority (signer)
                let account_metas = [
                    InstructionAccount::writable(self.source_account.address()),
                    InstructionAccount::readonly(spl.mint.address()),
                    InstructionAccount::writable(self.destination_account.address()),
                    InstructionAccount::readonly_signer(self.authority.address()),
                ];

                // SPL token program ID from source account owner (Pubkey = [u8; 32])
                let source_owner = unsafe { self.source_account.owner() };
                let instruction = InstructionView {
                    program_id: source_owner,
                    accounts: &account_metas,
                    data: &ix_data,
                };

                let account_infos = [
                    self.source_account,
                    spl.mint,
                    self.destination_account,
                    self.authority,
                ];

                invoke_signed_with_slice(&instruction, &account_infos, signers)
            }
        }
    }
}
