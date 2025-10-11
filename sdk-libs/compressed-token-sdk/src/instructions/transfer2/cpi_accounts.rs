use light_account_checks::{AccountError, AccountInfoTrait, AccountIterator};
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_program_profiler::profile;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA,
};
use solana_instruction::AccountMeta;
use solana_msg::msg;

use crate::error::TokenSdkError;

/// Parsed Transfer2 CPI accounts for structured access
#[derive(Debug)]
pub struct Transfer2CpiAccounts<'a, A: AccountInfoTrait + Clone> {
    // Programs and authorities (in order)
    pub compressed_token_program: &'a A,
    /// Needed with cpi context to do the other cpi to the system program.
    pub invoking_program_cpi_authority: Option<&'a A>,
    pub light_system_program: &'a A,

    // Core system accounts
    pub fee_payer: &'a A,
    pub compressed_token_cpi_authority: &'a A,
    pub registered_program_pda: &'a A,
    pub account_compression_authority: &'a A,
    pub account_compression_program: &'a A,
    pub system_program: &'a A,

    // Optional accounts
    pub sol_pool_pda: Option<&'a A>,
    pub sol_decompression_recipient: Option<&'a A>,
    pub cpi_context: Option<&'a A>,

    /// Packed accounts (trees, queues, mints, owners, delegates, etc)
    /// Trees and queues must be first.
    pub packed_accounts: &'a [A],
}

impl<'a, A: AccountInfoTrait + Clone> Transfer2CpiAccounts<'a, A> {
    /// Following the order: compressed_token_program, invoking_program_cpi_authority, light_system_program, ...
    /// Checks in this function are for convenience and not security critical.
    #[profile]
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_full(
        fee_payer: &'a A,
        accounts: &'a [A],
        with_sol_pool: bool,
        with_sol_decompression: bool,
        with_cpi_context: bool,
        light_system_cpi_authority: bool,
    ) -> Result<Self, TokenSdkError> {
        let mut iter = AccountIterator::new(accounts);

        let compressed_token_program =
            iter.next_checked_pubkey("compressed_token_program", COMPRESSED_TOKEN_PROGRAM_ID)?;

        let invoking_program_cpi_authority =
            iter.next_option("CPI_SIGNER.cpi_authority", light_system_cpi_authority)?;
        let compressed_token_cpi_authority =
            iter.next_checked_pubkey("compressed_token_cpi_authority", CPI_AUTHORITY_PDA)?;

        let light_system_program =
            iter.next_checked_pubkey("light_system_program", LIGHT_SYSTEM_PROGRAM_ID)?;

        let registered_program_pda =
            iter.next_checked_pubkey("registered_program_pda", REGISTERED_PROGRAM_PDA)?;

        let account_compression_authority = iter.next_checked_pubkey(
            "account_compression_authority",
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
        )?;

        let account_compression_program = iter.next_checked_pubkey(
            "account_compression_program",
            ACCOUNT_COMPRESSION_PROGRAM_ID,
        )?;

        let system_program = iter.next_checked_pubkey("system_program", [0u8; 32])?;

        let sol_pool_pda = iter.next_option_mut("sol_pool_pda", with_sol_pool)?;

        let sol_decompression_recipient =
            iter.next_option_mut("sol_decompression_recipient", with_sol_decompression)?;

        let cpi_context = iter.next_option_mut("cpi_context", with_cpi_context)?;

        let packed_accounts = iter.remaining()?;
        if !packed_accounts[0].is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
            msg!("First packed accounts must be tree or queue accounts.");
            msg!("Found {:?} instead", packed_accounts[0].pubkey());
            return Err(AccountError::InvalidAccount.into());
        }

        Ok(Self {
            compressed_token_program,
            invoking_program_cpi_authority,
            light_system_program,
            fee_payer,
            compressed_token_cpi_authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            sol_pool_pda,
            sol_decompression_recipient,
            cpi_context,
            packed_accounts,
        })
    }

    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos(
        fee_payer: &'a A,
        accounts: &'a [A],
    ) -> Result<Self, TokenSdkError> {
        Self::try_from_account_infos_full(fee_payer, accounts, false, false, false, false)
    }

    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_cpi_context(
        fee_payer: &'a A,
        accounts: &'a [A],
    ) -> Result<Self, TokenSdkError> {
        Self::try_from_account_infos_full(fee_payer, accounts, false, false, true, false)
    }

    /// Get tree accounts (accounts owned by account compression program)
    pub fn packed_accounts(&self) -> &'a [A] {
        self.packed_accounts
    }

    /// Get tree accounts (accounts owned by account compression program)
    #[profile]
    #[inline(always)]
    pub fn packed_account_metas(&self) -> Vec<AccountMeta> {
        let mut vec = Vec::with_capacity(self.packed_accounts.len());
        for account in self.packed_accounts {
            vec.push(AccountMeta {
                pubkey: account.key().into(),
                is_writable: account.is_writable(),
                is_signer: account.is_signer(),
            });
        }
        vec
    }

    /// Get a packed account by index
    pub fn packed_account_by_index(&self, index: u8) -> Option<&'a A> {
        self.packed_accounts.get(index as usize)
    }

    /// Get accounts for CPI to light system program (excludes the programs themselves)
    #[profile]
    #[inline(always)]
    pub fn to_account_infos(&self) -> Vec<A> {
        let mut accounts = Vec::with_capacity(10 + self.packed_accounts.len());

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

        if let Some(sol_pool) = self.sol_pool_pda {
            accounts.push(sol_pool.clone());
        }
        if let Some(recipient) = self.sol_decompression_recipient {
            accounts.push(recipient.clone());
        }
        if let Some(context) = self.cpi_context {
            accounts.push(context.clone());
        }
        self.packed_accounts.iter().for_each(|e| {
            accounts.push(e.clone());
        });

        accounts
    }
}
