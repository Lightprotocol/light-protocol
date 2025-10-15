use light_account_checks::{AccountError, AccountInfoTrait, AccountIterator};
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_program_profiler::profile;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA, SOL_POOL_PDA,
};
use solana_instruction::AccountMeta;
use solana_msg::msg;

use crate::error::TokenSdkError;

/// Parsed MintAction CPI accounts for structured access
#[derive(Debug)]
pub struct MintActionCpiAccounts<'a, A: AccountInfoTrait + Clone> {
    // Programs (in order)
    pub compressed_token_program: &'a A,
    pub light_system_program: &'a A,

    // Mint-specific accounts
    pub mint_signer: Option<&'a A>, // Required when creating mint or SPL mint
    pub authority: &'a A,           // Always required to sign

    // Decompressed mint accounts (conditional group - all or none)
    pub mint: Option<&'a A>,           // SPL mint account (when decompressed)
    pub token_pool_pda: Option<&'a A>, // Token pool PDA (when decompressed)
    pub token_program: Option<&'a A>,  // SPL Token 2022 (when decompressed)

    // Core Light system accounts
    pub fee_payer: &'a A,
    pub compressed_token_cpi_authority: &'a A,
    pub registered_program_pda: &'a A,
    pub account_compression_authority: &'a A,
    pub account_compression_program: &'a A,
    pub system_program: &'a A,

    // Optional system accounts
    pub sol_pool_pda: Option<&'a A>, // For lamports operations
    pub cpi_context: Option<&'a A>,  // For CPI context

    // Tree/Queue accounts (always present in execute mode)
    pub out_output_queue: &'a A,
    pub in_merkle_tree: &'a A, // Address tree when creating, state tree otherwise
    pub in_output_queue: Option<&'a A>, // When mint exists (not creating)
    pub tokens_out_queue: Option<&'a A>, // For MintTo actions

    // Remaining accounts for MintToCToken actions
    pub ctoken_accounts: &'a [A],
}

impl<'a, A: AccountInfoTrait + Clone> MintActionCpiAccounts<'a, A> {
    // TODO: add a config and derive config from instruction data
    /// Parse accounts for mint_action CPI with full configuration
    /// Following the exact order expected by the on-chain program
    #[profile]
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_full(
        accounts: &'a [A],
        with_mint_signer: bool,
        spl_mint_initialized: bool,
        with_lamports: bool,
        with_cpi_context: bool,
        create_mint: bool,         // true = address tree, false = state tree
        has_mint_to_actions: bool, // true = tokens_out_queue required
    ) -> Result<Self, TokenSdkError> {
        let mut iter = AccountIterator::new(accounts);

        // 1. Compressed token program (always required)
        let compressed_token_program =
            iter.next_checked_pubkey("compressed_token_program", COMPRESSED_TOKEN_PROGRAM_ID)?;

        // 2. Light system program (always required)
        let light_system_program =
            iter.next_checked_pubkey("light_system_program", LIGHT_SYSTEM_PROGRAM_ID)?;

        // 3. Mint signer (conditional - when creating mint or SPL mint)
        let mint_signer = iter.next_option("mint_signer", with_mint_signer)?;

        // 4. Authority (always required, must be signer)
        let authority = iter.next_account("authority")?;
        if !authority.is_signer() {
            msg!("Authority must be a signer");
            return Err(AccountError::InvalidSigner.into());
        }

        // 5-7. Decompressed mint accounts (conditional group)
        let (mint, token_pool_pda, token_program) = if spl_mint_initialized {
            let mint = Some(iter.next_account("mint")?);
            let pool = Some(iter.next_account("token_pool_pda")?);
            let program = Some(iter.next_account("token_program")?);

            // Validate SPL Token 2022 program
            if let Some(prog) = program {
                if prog.key() != spl_token_2022::ID.to_bytes() {
                    msg!(
                        "Invalid token program. Expected SPL Token 2022 ({:?}), got {:?}",
                        spl_token_2022::ID,
                        prog.pubkey()
                    );
                    return Err(AccountError::InvalidProgramId.into());
                }
            }

            (mint, pool, program)
        } else {
            (None, None, None)
        };

        // 8. Fee payer (always required, must be signer and mutable)
        let fee_payer = iter.next_account("fee_payer")?;
        if !fee_payer.is_signer() || !fee_payer.is_writable() {
            msg!("Fee payer must be a signer and mutable");
            return Err(AccountError::InvalidSigner.into());
        }

        // 9. CPI authority PDA
        let compressed_token_cpi_authority =
            iter.next_checked_pubkey("compressed_token_cpi_authority", CPI_AUTHORITY_PDA)?;

        // 10. Registered program PDA
        let registered_program_pda =
            iter.next_checked_pubkey("registered_program_pda", REGISTERED_PROGRAM_PDA)?;

        // 11. Account compression authority
        let account_compression_authority = iter.next_checked_pubkey(
            "account_compression_authority",
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
        )?;

        // 12. Account compression program
        let account_compression_program = iter.next_checked_pubkey(
            "account_compression_program",
            ACCOUNT_COMPRESSION_PROGRAM_ID,
        )?;

        // 13. System program
        let system_program = iter.next_checked_pubkey("system_program", [0u8; 32])?;

        // 14. SOL pool PDA (optional - for lamports operations)
        let sol_pool_pda = if with_lamports {
            Some(iter.next_checked_pubkey("sol_pool_pda", SOL_POOL_PDA)?)
        } else {
            None
        };

        // 15. CPI context (optional)
        let cpi_context = iter.next_option_mut("cpi_context", with_cpi_context)?;

        // 16. Out output queue (always required)
        let out_output_queue = iter.next_account("out_output_queue")?;
        if !out_output_queue.is_writable() {
            msg!("Out output queue must be mutable");
            return Err(AccountError::AccountMutable.into());
        }

        // 17. In merkle tree (always required)
        // When create_mint=true: this is the address tree for creating new mint addresses
        // When create_mint=false: this is the state tree containing the existing compressed mint
        let in_merkle_tree = iter.next_account("in_merkle_tree")?;
        if !in_merkle_tree.is_writable() {
            msg!("In merkle tree must be mutable");
            return Err(AccountError::AccountMutable.into());
        }

        // Validate tree ownership
        if !in_merkle_tree.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
            msg!("In merkle tree must be owned by account compression program");
            return Err(AccountError::AccountOwnedByWrongProgram.into());
        }

        // 18. In output queue (conditional - when mint exists, not creating)
        let in_output_queue = iter.next_option_mut("in_output_queue", !create_mint)?;
        if let Some(queue) = in_output_queue {
            if !queue.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
                msg!("In output queue must be owned by account compression program");
                return Err(AccountError::AccountOwnedByWrongProgram.into());
            }
        }

        // 19. Tokens out queue (conditional - for MintTo actions)
        let tokens_out_queue = iter.next_option_mut("tokens_out_queue", has_mint_to_actions)?;
        if let Some(queue) = tokens_out_queue {
            if !queue.is_owned_by(&ACCOUNT_COMPRESSION_PROGRAM_ID) {
                msg!("Tokens out queue must be owned by account compression program");
                return Err(AccountError::AccountOwnedByWrongProgram.into());
            }
        }

        // 20+. Decompressed token accounts (remaining accounts for MintToCToken)
        let ctoken_accounts = iter.remaining_unchecked()?;

        Ok(Self {
            compressed_token_program,
            light_system_program,
            mint_signer,
            authority,
            mint,
            token_pool_pda,
            token_program,
            fee_payer,
            compressed_token_cpi_authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            sol_pool_pda,
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
        Self::try_from_account_infos_full(
            accounts, false, // with_mint_signer
            false, // spl_mint_initialized
            false, // with_lamports
            false, // with_cpi_context
            false, // create_mint
            false, // has_mint_to_actions
        )
    }

    /// Parse for creating a new mint
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_create_mint(
        accounts: &'a [A],
        with_mint_signer: bool,
        spl_mint_initialized: bool,
        with_lamports: bool,
        has_mint_to_actions: bool,
    ) -> Result<Self, TokenSdkError> {
        Self::try_from_account_infos_full(
            accounts,
            with_mint_signer,
            spl_mint_initialized,
            with_lamports,
            false, // with_cpi_context
            true,  // create_mint
            has_mint_to_actions,
        )
    }

    /// Parse for updating an existing mint
    #[inline(always)]
    #[track_caller]
    pub fn try_from_account_infos_update_mint(
        accounts: &'a [A],
        spl_mint_initialized: bool,
        with_lamports: bool,
        has_mint_to_actions: bool,
    ) -> Result<Self, TokenSdkError> {
        Self::try_from_account_infos_full(
            accounts,
            false, // with_mint_signer
            spl_mint_initialized,
            with_lamports,
            false, // with_cpi_context
            false, // create_mint
            has_mint_to_actions,
        )
    }

    /// Get tree/queue pubkeys
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

    /// Convert to account infos for CPI (excludes compressed_token_program)
    #[profile]
    #[inline(always)]
    pub fn to_account_infos(&self) -> Vec<A> {
        let mut accounts = Vec::with_capacity(20 + self.ctoken_accounts.len());

        // Start with light_system_program
        accounts.push(self.light_system_program.clone());

        // Add mint_signer if present
        if let Some(signer) = self.mint_signer {
            accounts.push(signer.clone());
        }

        // Authority
        accounts.push(self.authority.clone());

        // Decompressed mint accounts
        if let Some(mint) = self.mint {
            accounts.push(mint.clone());
        }
        if let Some(pool) = self.token_pool_pda {
            accounts.push(pool.clone());
        }
        if let Some(program) = self.token_program {
            accounts.push(program.clone());
        }

        // Core Light system accounts
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

        // Optional system accounts
        if let Some(pool) = self.sol_pool_pda {
            accounts.push(pool.clone());
        }
        if let Some(context) = self.cpi_context {
            accounts.push(context.clone());
        }

        // Tree/Queue accounts
        accounts.push(self.out_output_queue.clone());
        accounts.push(self.in_merkle_tree.clone());

        if let Some(queue) = self.in_output_queue {
            accounts.push(queue.clone());
        }
        if let Some(queue) = self.tokens_out_queue {
            accounts.push(queue.clone());
        }

        // Decompressed token accounts
        for account in self.ctoken_accounts {
            accounts.push(account.clone());
        }

        accounts
    }

    /// Convert to AccountMeta vector for instruction building
    #[profile]
    #[inline(always)]
    pub fn to_account_metas(&self, include_compressed_token_program: bool) -> Vec<AccountMeta> {
        let mut metas = Vec::with_capacity(21 + self.ctoken_accounts.len());

        // Optionally include compressed_token_program
        if include_compressed_token_program {
            metas.push(AccountMeta {
                pubkey: self.compressed_token_program.key().into(),
                is_writable: false,
                is_signer: false,
            });
        }

        // Light system program
        metas.push(AccountMeta {
            pubkey: self.light_system_program.key().into(),
            is_writable: false,
            is_signer: false,
        });

        // Mint signer if present
        if let Some(signer) = self.mint_signer {
            metas.push(AccountMeta {
                pubkey: signer.key().into(),
                is_writable: false,
                is_signer: signer.is_signer(),
            });
        }

        // Authority
        metas.push(AccountMeta {
            pubkey: self.authority.key().into(),
            is_writable: false,
            is_signer: true,
        });

        // Decompressed mint accounts
        if let Some(mint) = self.mint {
            metas.push(AccountMeta {
                pubkey: mint.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }
        if let Some(pool) = self.token_pool_pda {
            metas.push(AccountMeta {
                pubkey: pool.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }
        if let Some(program) = self.token_program {
            metas.push(AccountMeta {
                pubkey: program.key().into(),
                is_writable: false,
                is_signer: false,
            });
        }

        // Core Light system accounts
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

        // Optional system accounts
        if let Some(pool) = self.sol_pool_pda {
            metas.push(AccountMeta {
                pubkey: pool.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }
        if let Some(context) = self.cpi_context {
            metas.push(AccountMeta {
                pubkey: context.key().into(),
                is_writable: true,
                is_signer: false,
            });
        }

        // Tree/Queue accounts
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

        // Decompressed token accounts
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
