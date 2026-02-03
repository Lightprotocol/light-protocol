//! Unified transfer interface that auto-routes based on account types.

use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

use super::{
    transfer::TransferCpi, transfer_from_spl::TransferFromSplCpi, transfer_to_spl::TransferToSplCpi,
};

/// Check if an account is owned by the Light Token program.
fn is_light_token_owner(owner: &[u8; 32]) -> bool {
    owner == &LIGHT_TOKEN_PROGRAM_ID
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
fn determine_transfer_type(source_owner: &[u8; 32], destination_owner: &[u8; 32]) -> TransferType {
    let source_is_light = is_light_token_owner(source_owner);
    let dest_is_light = is_light_token_owner(destination_owner);

    match (source_is_light, dest_is_light) {
        (true, true) => TransferType::LightToLight,
        (true, false) => TransferType::LightToSpl,
        (false, true) => TransferType::SplToLight,
        (false, false) => TransferType::SplToSpl,
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
    pub spl_interface: Option<SplInterfaceCpi<'info>>,
    /// System program - required for compressible account lamport top-ups
    pub system_program: &'info AccountInfo,
    /// Maximum lamports for rent and top-up combined (for light->light transfers)
    pub max_top_up: Option<u16>,
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
            spl_interface: None,
            system_program,
            max_top_up: None,
        }
    }

    /// Add SPL interface accounts (required for SPL<->light transfers).
    pub fn with_spl_interface(mut self, spl_interface: SplInterfaceCpi<'info>) -> Self {
        self.spl_interface = Some(spl_interface);
        self
    }

    /// Set max top-up lamports (for light->light transfers).
    pub fn with_max_top_up(mut self, max_top_up: u16) -> Self {
        self.max_top_up = Some(max_top_up);
        self
    }

    /// Invoke the appropriate transfer based on account types.
    pub fn invoke(self) -> Result<(), ProgramError> {
        let transfer_type = determine_transfer_type(
            self.source_account.owner(),
            self.destination_account.owner(),
        );

        match transfer_type {
            TransferType::LightToLight => {
                TransferCpi {
                    source: self.source_account,
                    destination: self.destination_account,
                    amount: self.amount,
                    authority: self.authority,
                    system_program: self.system_program,
                    max_top_up: self.max_top_up,
                    fee_payer: None,
                }
                .invoke()
            }

            TransferType::LightToSpl => {
                let spl = self
                    .spl_interface
                    .ok_or(ProgramError::InvalidAccountData)?;
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
                let spl = self
                    .spl_interface
                    .ok_or(ProgramError::InvalidAccountData)?;
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
                // For SPL-to-SPL, delegate to the SPL token program directly
                // Caller should use SPL token program directly, not Light Token interface
                Err(ProgramError::Custom(
                    crate::error::LightTokenError::UseRegularSplTransfer as u32,
                ))
            }
        }
    }

    /// Invoke with signer seeds.
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let transfer_type = determine_transfer_type(
            self.source_account.owner(),
            self.destination_account.owner(),
        );

        match transfer_type {
            TransferType::LightToLight => {
                TransferCpi {
                    source: self.source_account,
                    destination: self.destination_account,
                    amount: self.amount,
                    authority: self.authority,
                    system_program: self.system_program,
                    max_top_up: self.max_top_up,
                    fee_payer: None,
                }
                .invoke_signed(signer_seeds)
            }

            TransferType::LightToSpl => {
                let spl = self
                    .spl_interface
                    .ok_or(ProgramError::InvalidAccountData)?;
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
                .invoke_signed(signer_seeds)
            }

            TransferType::SplToLight => {
                let spl = self
                    .spl_interface
                    .ok_or(ProgramError::InvalidAccountData)?;
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
                .invoke_signed(signer_seeds)
            }

            TransferType::SplToSpl => {
                // For SPL-to-SPL, delegate to the SPL token program directly
                // Caller should use SPL token program directly, not Light Token interface
                Err(ProgramError::Custom(
                    crate::error::LightTokenError::UseRegularSplTransfer as u32,
                ))
            }
        }
    }
}
