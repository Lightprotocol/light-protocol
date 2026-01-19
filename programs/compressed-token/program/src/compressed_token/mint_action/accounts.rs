use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressible::config::CompressibleConfig;
use light_program_profiler::profile;
use light_token_interface::{
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    MINT_ADDRESS_TREE,
};
use light_zero_copy::U16;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::shared::{
    accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
    next_config_account, AccountIterator,
};

pub struct MintActionAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    /// Seed for mint PDA derivation.
    /// Required only for compressed mint creation.
    /// Note: mint_signer is not in executing accounts since create mint
    /// is allowed in combination with write to cpi context.
    pub mint_signer: Option<&'info AccountInfo>,
    pub authority: &'info AccountInfo,
    /// Required accounts to execute an instruction
    /// with or without cpi context.
    /// - write_to_cpi_context_system is None
    pub executing: Option<ExecutingAccounts<'info>>,
    /// Required accounts to write into a cpi context account.
    /// - executing is None
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
    /// Packed accounts contain
    /// [
    ///     ..tree_accounts,
    ///     ..recipient_token_accounts (mint_to_ctoken)
    /// ]
    pub packed_accounts: ProgramPackedAccounts<'info, AccountInfo>,
}

/// Required accounts to execute an instruction
/// with or without cpi context.
pub struct ExecutingAccounts<'info> {
    /// CompressibleConfig - parsed and validated (active state) when creating CMint.
    pub compressible_config: Option<&'info CompressibleConfig>,
    /// CMint Solana account (decompressed compressed mint).
    /// Required for DecompressMint, CompressAndCloseCMint, and operations on decompressed mints.
    pub cmint: Option<&'info AccountInfo>,
    /// Rent sponsor PDA - required when creating CMint (pays for account).
    pub rent_sponsor: Option<&'info AccountInfo>,
    pub system: LightSystemAccounts<'info>,
    /// Out output queue for the compressed mint account.
    pub out_output_queue: &'info AccountInfo,
    /// In state Merkle tree account for existing compressed mint.
    /// Required when compressed mint already exists.
    pub in_merkle_tree: Option<&'info AccountInfo>,
    /// Address Merkle tree account for creating compressed mint.
    /// Required when creating a new compressed mint.
    pub address_merkle_tree: Option<&'info AccountInfo>,
    /// Required, if compressed mint already exists.
    pub in_output_queue: Option<&'info AccountInfo>,
    /// Required, for action mint to compressed.
    pub tokens_out_queue: Option<&'info AccountInfo>,
}

impl<'info> MintActionAccounts<'info> {
    #[profile]
    #[track_caller]
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        config: &AccountsConfig,
        cmint_pubkey: Option<&solana_pubkey::Pubkey>,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;

        // mint_signer needs to sign for create_mint, but not for decompress_mint
        let mint_signer = if config.mint_signer_must_sign() {
            iter.next_option_signer("mint_signer", config.with_mint_signer)?
        } else {
            None
        };
        // Static non-CPI accounts first
        // Authority is always required to sign
        let authority = iter.next_signer("authority")?;
        if config.write_to_cpi_context {
            let write_to_cpi_context_system = CpiContextLightSystemAccounts::new(&mut iter)?;

            if !iter.iterator_is_empty() {
                msg!("Too many accounts for write to cpi context.");
                return Err(ProgramError::InvalidAccountData);
            }
            Ok(MintActionAccounts {
                light_system_program,
                mint_signer,
                authority,
                executing: None,
                write_to_cpi_context_system: Some(write_to_cpi_context_system),
                packed_accounts: ProgramPackedAccounts { accounts: &[] },
            })
        } else {
            // Parse and validate compressible config when creating or closing CMint
            let compressible_config = if config.needs_compressible_accounts() {
                Some(next_config_account(&mut iter)?)
            } else {
                None
            };

            // CMint account required if already decompressed OR being decompressed/closed
            let cmint = iter.next_option_mut("cmint", config.needs_cmint_account())?;

            // Parse rent_sponsor when creating or closing CMint
            let rent_sponsor =
                iter.next_option_mut("rent_sponsor", config.needs_compressible_accounts())?;

            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                false,
                false,
                config.with_cpi_context,
            )?;
            let out_output_queue = iter.next_account("out_output_queue")?;

            // Parse merkle tree based on whether we're creating or updating mint
            let (in_merkle_tree, address_merkle_tree) = if config.create_mint {
                // Creating mint: next account is address merkle tree
                let address_tree = iter.next_account("address_merkle_tree")?;
                (None, Some(address_tree))
            } else {
                // Existing mint: next account is in merkle tree
                let in_tree = iter.next_account("in_merkle_tree")?;
                (Some(in_tree), None)
            };

            let in_output_queue = iter.next_option("in_output_queue", !config.create_mint)?;
            // Only needed for minting to compressed token accounts
            let tokens_out_queue =
                iter.next_option("tokens_out_queue", config.require_token_output_queue)?;

            let mint_accounts = MintActionAccounts {
                mint_signer,
                light_system_program,
                authority,
                executing: Some(ExecutingAccounts {
                    compressible_config,
                    cmint,
                    rent_sponsor,
                    system,
                    in_merkle_tree,
                    address_merkle_tree,
                    in_output_queue,
                    out_output_queue,
                    tokens_out_queue,
                }),
                write_to_cpi_context_system: None,
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining_unchecked()?,
                },
            };
            mint_accounts.validate_accounts(cmint_pubkey)?;

            Ok(mint_accounts)
        }
    }

    pub fn cpi_authority(&self) -> Result<&AccountInfo, ProgramError> {
        if let Some(executing) = &self.executing {
            Ok(executing.system.cpi_authority_pda)
        } else {
            let cpi_system = self
                .write_to_cpi_context_system
                .as_ref()
                .ok_or(ErrorCode::ExpectedCpiAuthority)?;
            Ok(cpi_system.cpi_authority_pda)
        }
    }

    #[inline(always)]
    pub fn tree_pubkeys(&self, deduplicated: bool) -> Vec<&'info Pubkey> {
        let mut pubkeys = Vec::with_capacity(4);

        if let Some(executing) = &self.executing {
            pubkeys.push(executing.out_output_queue.key());

            // Include either in_merkle_tree or address_merkle_tree based on which is present
            if let Some(in_tree) = executing.in_merkle_tree {
                pubkeys.push(in_tree.key());
            } else if let Some(address_tree) = executing.address_merkle_tree {
                pubkeys.push(address_tree.key());
            }

            if let Some(in_queue) = executing.in_output_queue {
                pubkeys.push(in_queue.key());
            }
            if let Some(tokens_out_queue) = executing.tokens_out_queue {
                if !deduplicated {
                    pubkeys.push(tokens_out_queue.key());
                }
            }
        }
        pubkeys
    }

    /// Calculate the dynamic CPI accounts offset based on which accounts are present
    pub fn cpi_accounts_start_offset(&self) -> usize {
        // light_system_program & authority (always present)
        let mut offset = 2;

        // mint_signer (optional)
        if self.mint_signer.is_some() {
            offset += 1;
        }

        if let Some(executing) = &self.executing {
            // compressible_config (optional) - when creating CMint
            if executing.compressible_config.is_some() {
                offset += 1;
            }
            // cmint (optional) - comes before rent_sponsor
            if executing.cmint.is_some() {
                offset += 1;
            }
            // rent_sponsor (optional) - when creating CMint
            if executing.rent_sponsor.is_some() {
                offset += 1;
            }
            // LightSystemAccounts - CPI accounts start here
        }
        // write_to_cpi_context_system - these are the CPI accounts that start here
        // We don't add them to offset since this is where CPI accounts begin

        offset
    }

    pub fn cpi_accounts_end_offset(&self, deduplicated: bool) -> usize {
        if self.write_to_cpi_context_system.is_some() {
            self.cpi_accounts_start_offset() + CpiContextLightSystemAccounts::cpi_len()
        } else {
            let mut offset = self.cpi_accounts_start_offset();
            if let Some(executing) = self.executing.as_ref() {
                offset += LightSystemAccounts::cpi_len();
                if executing.system.sol_pool_pda.is_some() {
                    offset += 1;
                }
                if executing.system.cpi_context.is_some() {
                    offset += 1;
                }

                // out_output_queue (always present)
                // Either in_merkle_tree or address_merkle_tree (always present)
                offset += 2;
                if executing.in_output_queue.is_some() {
                    offset += 1;
                }
                // When deduplicated=false, we need to include the extra queue account
                // When deduplicated=true, the duplicate queue is in the outer instruction but not in CPI slice
                if executing.tokens_out_queue.is_some() && !deduplicated {
                    offset += 1;
                }
            }
            offset
        }
    }

    pub fn get_cpi_accounts<'a>(
        &self,
        deduplicated: bool,
        account_infos: &'a [AccountInfo],
    ) -> Result<&'a [AccountInfo], ProgramError> {
        let start_offset = self.cpi_accounts_start_offset();
        let end_offset = self.cpi_accounts_end_offset(deduplicated);

        if end_offset > account_infos.len() {
            return Err(ErrorCode::CpiAccountsSliceOutOfBounds.into());
        }

        Ok(&account_infos[start_offset..end_offset])
    }

    /// Check if tokens_out_queue exists in executing accounts.
    /// Used for queue deduplication logic.
    pub fn has_tokens_out_queue(&self) -> bool {
        self.executing
            .as_ref()
            .map(|executing| executing.tokens_out_queue.is_some())
            .unwrap_or_else(|| false)
    }

    /// Check if out_output_queue and tokens_out_queue have the same key.
    /// Used for queue index logic when no CPI context is provided.
    pub fn queue_keys_match(&self) -> bool {
        if let Some(executing) = &self.executing {
            if let Some(tokens_out_queue) = executing.tokens_out_queue {
                return executing.out_output_queue.key() == tokens_out_queue.key();
            }
        }
        false
    }

    /// Get CMint account if present in executing accounts.
    pub fn get_cmint(&self) -> Option<&'info AccountInfo> {
        self.executing.as_ref().and_then(|exec| exec.cmint)
    }

    pub fn validate_accounts(
        &self,
        cmint_pubkey: Option<&solana_pubkey::Pubkey>,
    ) -> Result<(), ProgramError> {
        let accounts = self
            .executing
            .as_ref()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        // TODO: check whether we can simplify or move to decompress action processor.
        // When cmint_pubkey is provided, verify CMint account matches
        // When None (mint data from CMint), skip - CMint is validated when reading its data
        if let (Some(cmint_account), Some(expected_pubkey)) = (accounts.cmint, cmint_pubkey) {
            if expected_pubkey.to_bytes() != *cmint_account.key() {
                return Err(ErrorCode::MintAccountMismatch.into());
            }
        }

        // Validate address merkle tree when creating mint
        if let Some(address_tree) = accounts.address_merkle_tree {
            if *address_tree.key() != MINT_ADDRESS_TREE {
                msg!(
                    "Create mint action expects address Merkle tree {:?} received: {:?}",
                    solana_pubkey::Pubkey::from(MINT_ADDRESS_TREE),
                    solana_pubkey::Pubkey::from(*address_tree.key())
                );
                return Err(ErrorCode::InvalidAddressTree.into());
            }
        }

        Ok(())
    }
}

/// Config to parse AccountInfos based on instruction data.
/// We use instruction data to convey which accounts are expected.
#[derive(Debug, PartialEq)]
pub struct AccountsConfig {
    /// 1. cpi context is some
    pub with_cpi_context: bool,
    /// 2. cpi context.first_set() || cpi context.set()
    pub write_to_cpi_context: bool,
    /// 4. Whether the compressed mint has been decompressed to a CMint Solana account.
    ///    When true, the CMint account is the decompressed (compressed account is empty).
    pub cmint_decompressed: bool,
    /// 5. Mint
    pub require_token_output_queue: bool,
    /// 6. Compressed mint is created.
    pub with_mint_signer: bool,
    /// 7. Compressed mint is created.
    pub create_mint: bool,
    /// 8. Has DecompressMint action
    pub has_decompress_mint_action: bool,
    /// 9. Has CompressAndCloseCMint action
    pub has_compress_and_close_cmint_action: bool,
}

impl AccountsConfig {
    /// Returns true when CMint Solana account is the decompressed for mint data.
    /// This is the case when the mint is decompressed (or being decompressed) and not being closed.
    /// When true, compressed account uses zero sentinel values (discriminator=[0;8], data_hash=[0;32]).
    #[inline(always)]
    pub fn cmint_output_decompressed(&self) -> bool {
        (self.has_decompress_mint_action || self.cmint_decompressed)
            && !self.has_compress_and_close_cmint_action
    }

    /// Returns true if compressible extension accounts are needed.
    /// Required for DecompressMint and CompressAndCloseCMint actions.
    #[inline(always)]
    pub fn needs_compressible_accounts(&self) -> bool {
        self.has_decompress_mint_action || self.has_compress_and_close_cmint_action
    }

    /// Returns true if CMint account is needed in the transaction.
    /// Required when: already decompressed, decompressing, or compressing and closing CMint.
    #[inline(always)]
    pub fn needs_cmint_account(&self) -> bool {
        self.cmint_decompressed
            || self.has_decompress_mint_action
            || self.has_compress_and_close_cmint_action
    }

    /// Returns true if mint_signer must be a signer.
    /// Required for create_mint, but NOT for decompress_mint.
    /// decompress_mint only needs mint_signer.key() for PDA derivation.
    #[inline(always)]
    pub fn mint_signer_must_sign(&self) -> bool {
        self.create_mint
    }

    /// Initialize AccountsConfig based in instruction data.  -
    #[profile]
    pub fn new(
        parsed_instruction_data: &ZMintActionCompressedInstructionData,
    ) -> Result<AccountsConfig, ProgramError> {
        if let Some(create_mint) = parsed_instruction_data.create_mint.as_ref() {
            if [0u8; 4] != create_mint.read_only_address_trees {
                msg!("read_only_address_trees must be 0");
                return Err(ProgramError::InvalidInstructionData);
            }
            if [U16::from(0); 4] != create_mint.read_only_address_tree_root_indices {
                msg!("read_only_address_tree_root_indices must be 0");
                return Err(ProgramError::InvalidInstructionData);
            }
        }

        // 1.cpi context
        let with_cpi_context = parsed_instruction_data.cpi_context.is_some();

        // 2. write to cpi context
        let write_to_cpi_context = parsed_instruction_data
            .cpi_context
            .as_ref()
            .map(|x| x.first_set_context() || x.set_context())
            .unwrap_or_default();

        // Check if DecompressMint action is present
        let has_decompress_mint_action = parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::DecompressMint(_)));

        // Check if CompressAndCloseCMint action is present
        let has_compress_and_close_cmint_action = parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::CompressAndCloseMint(_)));

        // Validation: Cannot combine DecompressMint and CompressAndCloseCMint in the same instruction
        if has_decompress_mint_action && has_compress_and_close_cmint_action {
            msg!("Cannot combine DecompressMint and CompressAndCloseCMint in the same instruction");
            return Err(ErrorCode::CannotDecompressAndCloseInSameInstruction.into());
        }

        // Validation: CompressAndCloseCMint must be the only action
        if has_compress_and_close_cmint_action && parsed_instruction_data.actions.len() != 1 {
            msg!("CompressAndCloseCMint must be the only action in the instruction");
            return Err(ErrorCode::CompressAndCloseCMintMustBeOnlyAction.into());
        }

        // We need mint signer only if creating a new mint.
        // CompressAndCloseCMint does NOT need mint_signer - it verifies CMint by compressed_mint.metadata.mint
        let with_mint_signer = parsed_instruction_data.create_mint.is_some();
        // CMint account needed when mint is already decompressed (metadata flag)
        // When mint is None, CMint is decompressed (data lives in CMint account, compressed account is empty)
        let cmint_decompressed = parsed_instruction_data.mint.is_none();

        if write_to_cpi_context {
            // Must not have any MintToCToken actions
            let has_mint_to_ctoken_actions = parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::MintTo(_)));
            if has_mint_to_ctoken_actions {
                msg!("Mint to ctokens not allowed when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }
            if has_decompress_mint_action {
                msg!("Decompress mint not allowed when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }

            if cmint_decompressed {
                msg!("CMint decompressed not allowed when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }
            let require_token_output_queue = parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::MintToCompressed(_)));
            Ok(AccountsConfig {
                with_cpi_context,
                write_to_cpi_context,
                cmint_decompressed,
                require_token_output_queue,
                with_mint_signer,
                create_mint: parsed_instruction_data.create_mint.is_some(),
                has_decompress_mint_action,
                has_compress_and_close_cmint_action,
            })
        } else {
            // For MintToCompressed actions
            // - needed for tokens_out_queue (only MintToCompressed creates new compressed outputs)
            // - MintToCToken mints to existing decompressed accounts, doesn't need tokens_out_queue
            let require_token_output_queue = parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::MintToCompressed(_)));

            Ok(AccountsConfig {
                with_cpi_context,
                write_to_cpi_context,
                cmint_decompressed,
                require_token_output_queue,
                with_mint_signer,
                create_mint: parsed_instruction_data.create_mint.is_some(),
                has_decompress_mint_action,
                has_compress_and_close_cmint_action,
            })
        }
    }
}
