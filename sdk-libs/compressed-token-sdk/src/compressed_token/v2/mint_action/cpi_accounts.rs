use light_account_checks::{AccountError, AccountInfoTrait, AccountIterator};
use light_program_profiler::profile;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA,
};
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use light_token_types::CPI_AUTHORITY_PDA;
use solana_instruction::AccountMeta;
use solana_msg::msg;

use crate::error::TokenSdkError;

#[derive(Debug, Clone, Default, Copy)]
pub struct MintActionCpiAccountsConfig {
    pub with_cpi_context: bool,
    pub create_mint: bool,        // true = address tree, false = state tree
    pub mint_to_compressed: bool, // true = tokens_out_queue required
}

impl MintActionCpiAccountsConfig {
    pub fn create_mint() -> Self {
        Self {
            with_cpi_context: false,
            create_mint: true,
            mint_to_compressed: false,
        }
    }

    pub fn mint_to_compressed(self) -> Self {
        Self {
            with_cpi_context: self.with_cpi_context,
            create_mint: self.create_mint,
            mint_to_compressed: true,
        }
    }
}

/// Parsed MintAction CPI accounts for structured access
#[derive(Debug)]
pub struct MintActionCpiAccounts<'a, A: AccountInfoTrait + Clone> {
    pub compressed_token_program: &'a A,
    pub light_system_program: &'a A,

    pub mint_signer: Option<&'a A>,
    pub authority: &'a A,

    /// Rent sponsor PDA — required when creating a new compressed mint (receives the creation fee).
    pub rent_sponsor: Option<&'a A>,

    pub fee_payer: &'a A,
    pub compressed_token_cpi_authority: &'a A,
    pub registered_program_pda: &'a A,
    pub account_compression_authority: &'a A,
    pub account_compression_program: &'a A,
    pub system_program: &'a A,

    pub cpi_context: Option<&'a A>,

    pub out_output_queue: &'a A,
    pub in_merkle_tree: &'a A,
    pub in_output_queue: Option<&'a A>,
    pub tokens_out_queue: Option<&'a A>,

    pub ctoken_accounts: &'a [A],
}

impl<'a, A: AccountInfoTrait + Clone> MintActionCpiAccounts<'a, A> {
    #[profile]
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_full(
        accounts: &'a [A],
        config: MintActionCpiAccountsConfig,
    ) -> Result<Self, TokenSdkError> {
        let mut iter = AccountIterator::new(accounts);

        let compressed_token_program =
            iter.next_checked_pubkey("compressed_token_program", LIGHT_TOKEN_PROGRAM_ID)?;

        let light_system_program =
            iter.next_checked_pubkey("light_system_program", LIGHT_SYSTEM_PROGRAM_ID)?;

        let mint_signer = iter.next_option("mint_signer", config.create_mint)?;

        let authority = iter.next_account("authority")?;
        if !authority.is_signer() {
            msg!("Authority must be a signer");
            return Err(AccountError::InvalidSigner.into());
        }

        let rent_sponsor = iter.next_option_mut("rent_sponsor", config.create_mint)?;

        let fee_payer = iter.next_account("fee_payer")?;
        if !fee_payer.is_signer() || !fee_payer.is_writable() {
            msg!("Fee payer must be a signer and mutable");
            return Err(AccountError::InvalidSigner.into());
        }

        let compressed_token_cpi_authority =
            iter.next_checked_pubkey("compressed_token_cpi_authority", CPI_AUTHORITY_PDA)?;

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

        let cpi_context = iter.next_option_mut("cpi_context", config.with_cpi_context)?;

        let out_output_queue = iter.next_account("out_output_queue")?;
        if !out_output_queue.is_writable() {
            msg!("Out output queue must be mutable");
            return Err(AccountError::AccountMutable.into());
        }

        let in_merkle_tree = iter.next_account("in_merkle_tree")?;
        if !in_merkle_tree.is_writable() {
            msg!("In merkle tree must be mutable");
            return Err(AccountError::AccountMutable.into());
        }

        if !in_merkle_tree.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
            msg!("In merkle tree must be owned by account compression program");
            return Err(AccountError::AccountOwnedByWrongProgram.into());
        }

        let in_output_queue = iter.next_option_mut("in_output_queue", !config.create_mint)?;
        if let Some(queue) = in_output_queue {
            if !queue.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
                msg!("In output queue must be owned by account compression program");
                return Err(AccountError::AccountOwnedByWrongProgram.into());
            }
        }

        let tokens_out_queue =
            iter.next_option_mut("tokens_out_queue", config.mint_to_compressed)?;
        if let Some(queue) = tokens_out_queue {
            if !queue.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
                msg!("Tokens out queue must be owned by account compression program");
                return Err(AccountError::AccountOwnedByWrongProgram.into());
            }
        }

        let ctoken_accounts = iter.remaining_unchecked()?;

        Ok(Self {
            compressed_token_program,
            light_system_program,
            mint_signer,
            authority,
            rent_sponsor,
            fee_payer,
            compressed_token_cpi_authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            cpi_context,
            out_output_queue,
            in_merkle_tree,
            in_output_queue,
            tokens_out_queue,
            ctoken_accounts,
        })
    }

    /// Simple version for common case (no optional features)
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos(accounts: &'a [A]) -> Result<Self, TokenSdkError> {
        Self::try_from_account_infos_full(accounts, MintActionCpiAccountsConfig::default())
    }

    #[profile]
    #[inline(always)]
    pub fn tree_queue_pubkeys(&self) -> Vec<[u8; 32]> {
        let mut pubkeys = vec![self.out_output_queue.key(), self.in_merkle_tree.key()];

        if let Some(queue) = self.in_output_queue {
            pubkeys.push(queue.key());
        }

        if let Some(queue) = self.tokens_out_queue {
            pubkeys.push(queue.key());
        }

        pubkeys
    }

    #[profile]
    #[inline(always)]
    pub fn to_account_infos(&self) -> Vec<A> {
        let mut accounts = Vec::with_capacity(20 + self.ctoken_accounts.len());

        accounts.push(self.light_system_program.clone());

        if let Some(signer) = self.mint_signer {
            accounts.push(signer.clone());
        }

        accounts.push(self.authority.clone());

        if let Some(sponsor) = self.rent_sponsor {
            accounts.push(sponsor.clone());
        }

        accounts.extend_from_slice(
            &[
                self.fee_payer.clone(),
                self.compressed_token_cpi_authority.clone(),
                self.registered_program_pda.clone(),
                self.account_compression_authority.clone(),
                self.account_compression_program.clone(),
                self.system_program.clone(),
            ][..],
        );

        if let Some(context) = self.cpi_context {
            accounts.push(context.clone());
        }

        accounts.push(self.out_output_queue.clone());
        accounts.push(self.in_merkle_tree.clone());

        if let Some(queue) = self.in_output_queue {
            accounts.push(queue.clone());
        }
        if let Some(queue) = self.tokens_out_queue {
            accounts.push(queue.clone());
        }

        for account in self.ctoken_accounts {
            accounts.push(account.clone());
        }

        accounts
    }

    #[profile]
    #[inline(always)]
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut metas = Vec::with_capacity(15 + self.ctoken_accounts.len());

        metas.push(AccountMeta {
            pubkey: self.light_system_program.key().into(),
            is_writable: false,
            is_signer: false,
        });

        if let Some(signer) = self.mint_signer {
            metas.push(AccountMeta {
                pubkey: signer.key().into(),
                is_writable: false,
                is_signer: signer.is_signer(),
            });
        }

        metas.push(AccountMeta {
            pubkey: self.authority.key().into(),
            is_writable: false,
            is_signer: true,
        });

        if let Some(sponsor) = self.rent_sponsor {
            metas.push(AccountMeta {
                pubkey: sponsor.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }

        metas.push(AccountMeta {
            pubkey: self.fee_payer.key().into(),
            is_writable: true,
            is_signer: true,
        });
        metas.push(AccountMeta {
            pubkey: self.compressed_token_cpi_authority.key().into(),
            is_writable: false,
            is_signer: false,
        });
        metas.push(AccountMeta {
            pubkey: self.registered_program_pda.key().into(),
            is_writable: false,
            is_signer: false,
        });
        metas.push(AccountMeta {
            pubkey: self.account_compression_authority.key().into(),
            is_writable: false,
            is_signer: false,
        });
        metas.push(AccountMeta {
            pubkey: self.account_compression_program.key().into(),
            is_writable: false,
            is_signer: false,
        });
        metas.push(AccountMeta {
            pubkey: self.system_program.key().into(),
            is_writable: false,
            is_signer: false,
        });

        if let Some(context) = self.cpi_context {
            metas.push(AccountMeta {
                pubkey: context.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }

        metas.push(AccountMeta {
            pubkey: self.out_output_queue.key().into(),
            is_writable: true,
            is_signer: false,
        });
        metas.push(AccountMeta {
            pubkey: self.in_merkle_tree.key().into(),
            is_writable: true,
            is_signer: false,
        });

        if let Some(queue) = self.in_output_queue {
            metas.push(AccountMeta {
                pubkey: queue.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }
        if let Some(queue) = self.tokens_out_queue {
            metas.push(AccountMeta {
                pubkey: queue.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }

        for account in self.ctoken_accounts {
            metas.push(AccountMeta {
                pubkey: account.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }
        metas
    }
}
