use anchor_compressed_token::{check_spl_token_pool_derivation_with_index, ErrorCode};
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::{
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    CMINT_ADDRESS_TREE,
};
use light_program_profiler::profile;
use light_zero_copy::U16;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::shared::{
    accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
    AccountIterator,
};

pub struct MintActionAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    /// Seed for spl mint pda.
    /// Required for mint and spl mint creation.
    /// Note: mint_signer is not in executing accounts since create mint
    /// is allowed in combination with write to cpi context.
    pub mint_signer: Option<&'info AccountInfo>,
    pub authority: &'info AccountInfo,
    /// Reqired accounts to execute an instruction
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

/// Reqired accounts to execute an instruction
/// with or without cpi context.
pub struct ExecutingAccounts<'info> {
    /// Spl mint acccount.
    pub mint: Option<&'info AccountInfo>,
    /// Ctoken pool pda, spl token account.
    pub token_pool_pda: Option<&'info AccountInfo>,
    /// Spl token 2022 program.
    pub token_program: Option<&'info AccountInfo>,
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
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        config: &AccountsConfig,
        cmint_pubkey: &solana_pubkey::Pubkey,
        token_pool_index: u8,
        token_pool_bump: u8,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;

        let mint_signer = iter.next_option_signer("mint_signer", config.with_mint_signer)?;
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
            let mint = iter.next_option_mut("mint", config.spl_mint_initialized)?;
            let token_pool_pda =
                iter.next_option_mut("token_pool_pda", config.spl_mint_initialized)?;
            let token_program = iter.next_option("token_program", config.spl_mint_initialized)?;
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
                iter.next_option("tokens_out_queue", config.has_mint_to_actions)?;
            let mint_accounts = MintActionAccounts {
                mint_signer,
                light_system_program,
                authority,
                executing: Some(ExecutingAccounts {
                    mint,
                    token_pool_pda,
                    token_program,
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
            mint_accounts.validate_accounts(cmint_pubkey, token_pool_index, token_pool_bump)?;

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
            // mint (optional)
            if executing.mint.is_some() {
                offset += 1;
            }

            // token_pool_pda (optional)
            if executing.token_pool_pda.is_some() {
                offset += 1;
            }

            // token_program (optional)
            if executing.token_program.is_some() {
                offset += 1;
            }

            // LightSystemAccounts - these are the CPI accounts that start here
            // We don't add them to offset since this is where CPI accounts begin
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

    pub fn validate_accounts(
        &self,
        cmint_pubkey: &solana_pubkey::Pubkey,
        token_pool_index: u8,
        token_pool_bump: u8,
    ) -> Result<(), ProgramError> {
        let accounts = self
            .executing
            .as_ref()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        // Validate token program is SPL Token 2022
        if let Some(token_program) = accounts.token_program.as_ref() {
            if *token_program.key() != spl_token_2022::ID.to_bytes() {
                msg!(
                    "invalid token program {:?} expected {:?}",
                    solana_pubkey::Pubkey::new_from_array(*token_program.key()),
                    spl_token_2022::ID
                );
                return Err(ProgramError::InvalidAccountData);
            }
        }

        // Validate token pool PDA is correct using provided bump and index
        if let Some(token_pool_pda) = accounts.token_pool_pda {
            let token_pool_pubkey_solana =
                solana_pubkey::Pubkey::new_from_array(*token_pool_pda.key());

            check_spl_token_pool_derivation_with_index(
                &token_pool_pubkey_solana,
                cmint_pubkey,
                token_pool_index,
                Some(token_pool_bump),
            )
            .map_err(|_| {
                msg!(
                    "invalid token pool PDA {:?} for mint {:?} with index {} and bump {}",
                    token_pool_pubkey_solana,
                    cmint_pubkey,
                    token_pool_index,
                    token_pool_bump
                );
                ProgramError::InvalidAccountData
            })?;
        }

        if let Some(mint_account) = accounts.mint {
            // Verify mint account matches expected mint
            if cmint_pubkey.to_bytes() != *mint_account.key() {
                return Err(ErrorCode::MintAccountMismatch.into());
            }
        }

        // Validate address merkle tree when creating mint
        if let Some(address_tree) = accounts.address_merkle_tree {
            if *address_tree.key() != CMINT_ADDRESS_TREE {
                msg!(
                    "Create mint action expects address Merkle tree {:?} received: {:?}",
                    solana_pubkey::Pubkey::from(CMINT_ADDRESS_TREE),
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
    /// 4. SPL mint is either:
    ///    4.1. already initialized
    ///    4.2. or is initialized in this instruction
    pub spl_mint_initialized: bool,
    /// 5. Mint
    pub has_mint_to_actions: bool,
    /// 6. Either compressed mint and/or spl mint is created.
    pub with_mint_signer: bool,
    /// 7. Compressed mint is created.
    pub create_mint: bool,
}

impl AccountsConfig {
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
        // An action in this instruction creates a the spl mint corresponding to a compressed mint.
        let create_spl_mint = parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::CreateSplMint(_)));

        // We need mint signer if create mint, and create spl mint.
        let with_mint_signer = parsed_instruction_data.create_mint.is_some() || create_spl_mint;
        // Scenarios:
        // 1. mint is already decompressed
        // 2. mint is decompressed in this instruction
        let spl_mint_initialized =
            parsed_instruction_data.mint.metadata.spl_mint_initialized() || create_spl_mint;

        if parsed_instruction_data.mint.metadata.spl_mint_initialized() && create_spl_mint {
            return Err(ProgramError::InvalidInstructionData);
        }

        if write_to_cpi_context {
            // Must not have any MintToCToken actions
            let has_mint_to_ctoken_actions = parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::MintToCToken(_)));
            if has_mint_to_ctoken_actions {
                msg!("Mint to ctokens not allowed when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }
            if create_spl_mint {
                msg!("Create spl mint not allowed when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }
            let has_mint_to_actions = parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::MintToCompressed(_)));
            if spl_mint_initialized && has_mint_to_actions {
                msg!("Mint to compressed not allowed if associated spl mint exists when writing to cpi context");
                return Err(ErrorCode::CpiContextSetNotUsable.into());
            }

            Ok(AccountsConfig {
                with_cpi_context,
                write_to_cpi_context,
                spl_mint_initialized,
                has_mint_to_actions,
                with_mint_signer,
                create_mint: parsed_instruction_data.create_mint.is_some(),
            })
        } else {
            // For MintTo or MintToCToken actions
            // - needed for tokens_out_queue and authority validation
            let has_mint_to_actions = parsed_instruction_data.actions.iter().any(|action| {
                matches!(
                    action,
                    ZAction::MintToCompressed(_) | ZAction::MintToCToken(_)
                )
            });

            Ok(AccountsConfig {
                with_cpi_context,
                write_to_cpi_context,
                spl_mint_initialized,
                has_mint_to_actions,
                with_mint_signer,
                create_mint: parsed_instruction_data.create_mint.is_some(),
            })
        }
    }
}
