use light_account_checks::{
    packed_accounts::ProgramPackedAccounts, AccountError, AccountInfoTrait, AccountIterator,
};
use light_program_profiler::profile;

use super::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, COMPRESSED_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID,
};

/// Parsed Transfer2 CPI accounts for structured access
pub struct Transfer2CpiAccounts<'a, A: AccountInfoTrait + Clone> {
    // Programs and authorities (in order)
    pub compressed_token_program: &'a A,
    pub light_system_program: &'a A,

    // Core system accounts
    pub fee_payer: A,
    pub compressed_token_cpi_authority: &'a A,
    pub registered_program_pda: &'a A,
    pub account_compression_authority: &'a A,
    pub account_compression_program: &'a A,
    pub system_program: &'a A,
    /// Packed accounts (trees, queues, mints, owners, delegates, etc)
    /// Trees and queues must be first.
    pub packed_accounts: ProgramPackedAccounts<'a, A>,
}

impl<'a, A: AccountInfoTrait + Clone> Transfer2CpiAccounts<'a, A> {
    /// Checks in this function are for convenience and not security critical.
    #[profile]
    #[inline(always)]
    pub fn try_from_account_infos(fee_payer: A, accounts: &'a [A]) -> Result<Self, AccountError> {
        let mut iter = AccountIterator::new(accounts);
        let compressed_token_program = iter.next_checked_pubkey(
            "compressed_token_program",
            COMPRESSED_TOKEN_PROGRAM_ID.to_bytes(),
        )?;

        let compressed_token_cpi_authority = iter.next_account("compressed_token_cpi_authority")?;

        let light_system_program =
            iter.next_checked_pubkey("light_system_program", LIGHT_SYSTEM_PROGRAM_ID.to_bytes())?;

        let registered_program_pda = iter.next_account("registered_program_pda")?;

        let account_compression_authority = iter.next_checked_pubkey(
            "account_compression_authority",
            ACCOUNT_COMPRESSION_AUTHORITY_PDA.to_bytes(),
        )?;

        let account_compression_program = iter.next_checked_pubkey(
            "account_compression_program",
            ACCOUNT_COMPRESSION_PROGRAM_ID.to_bytes(),
        )?;

        let system_program = iter.next_checked_pubkey("system_program", [0u8; 32])?;

        let packed_accounts = iter.remaining()?;
        if !packed_accounts[0].is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID.to_bytes()) {
            use anchor_lang::prelude::msg;
            msg!("First packed accounts must be tree or queue accounts.");
            msg!("Found {:?} instead", packed_accounts[0].pubkey());
            return Err(AccountError::InvalidAccount);
        }

        Ok(Self {
            compressed_token_program,
            light_system_program,
            fee_payer,
            compressed_token_cpi_authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            packed_accounts: ProgramPackedAccounts {
                accounts: packed_accounts,
            },
        })
    }

    /// Get accounts for CPI to light system program (excludes the programs themselves)
    #[profile]
    #[inline(always)]
    pub fn to_account_infos(&self) -> Vec<A> {
        let mut accounts = Vec::with_capacity(7 + self.packed_accounts.accounts.len());

        accounts.extend_from_slice(
            &[
                self.light_system_program.clone(),
                self.fee_payer.clone(),
                self.compressed_token_cpi_authority.clone(),
                self.registered_program_pda.clone(),
                self.account_compression_authority.clone(),
                self.account_compression_program.clone(),
                self.system_program.clone(),
            ][..],
        );

        self.packed_accounts.accounts.iter().for_each(|e| {
            accounts.push(e.clone());
        });

        accounts
    }
}
