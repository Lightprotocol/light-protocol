use crate::{
    shared::{
        accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
        AccountIterator,
    },
    transfer2::accounts::ProgramPackedAccounts,
};
use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::instructions::mint_actions::{
    ZAction, ZMintActionCompressedInstructionData,
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

pub struct MintActionAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub mint_signer: Option<&'info AccountInfo>,
    pub authority: &'info AccountInfo,
    pub executing: Option<ExecutingAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
    pub packed_accounts: ProgramPackedAccounts<'info>,
}

pub struct ExecutingAccounts<'info> {
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
    pub system: LightSystemAccounts<'info>,
    pub out_output_queue: &'info AccountInfo,
    pub in_merkle_tree: &'info AccountInfo,
    pub in_output_queue: Option<&'info AccountInfo>,
    pub tokens_out_queue: Option<&'info AccountInfo>,
}

impl<'info> MintActionAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        config: &AccountsConfig,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;
        // TODO: make it option signer
        let mint_signer = iter.next_option("mint_signer", config.with_mint_signer)?;
        // Static non-CPI accounts first
        let authority = iter.next_signer("authority")?;
        if config.write_to_cpi_context {
            Ok(MintActionAccounts {
                light_system_program,
                mint_signer,
                authority,
                executing: None,
                write_to_cpi_context_system: Some(
                    CpiContextLightSystemAccounts::validate_and_parse(&mut iter)?,
                ),
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining_unchecked()?,
                },
            })
        } else {
            let mint = iter.next_option_mut("mint", config.is_decompressed)?;
            let token_pool_pda = iter.next_option_mut("token_pool_pda", config.is_decompressed)?;
            let token_program = iter.next_option("token_program", config.is_decompressed)?;

            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                config.with_lamports,
                false,
                config.with_cpi_context,
            )?;

            let out_output_queue = iter.next_account("out_output_queue")?;
            // When create mint this is the address tree
            // When mint exists this is the in merkle tree.
            let in_merkle_tree = iter.next_account("in_merkle_tree")?;
            let in_output_queue = iter.next_option("in_output_queue", !config.create_mint)?;
            // Only needed for minting to compressed token accounts
            let tokens_out_queue =
                iter.next_option("tokens_out_queue", config.has_mint_to_actions)?;

            Ok(MintActionAccounts {
                mint_signer,
                light_system_program,
                authority,
                executing: Some(ExecutingAccounts {
                    mint,
                    token_pool_pda,
                    token_program,
                    system,
                    in_merkle_tree,
                    in_output_queue,
                    out_output_queue,
                    tokens_out_queue,
                }),
                write_to_cpi_context_system: None,
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining_unchecked()?,
                },
            })
        }
    }

    pub fn cpi_authority(&self) -> Result<&AccountInfo, ProgramError> {
        if let Some(executing) = &self.executing {
            Ok(executing.system.cpi_authority_pda)
        } else {
            let cpi_system = self
                .write_to_cpi_context_system
                .as_ref()
                .ok_or(ProgramError::InvalidInstructionData)?; // TODO: better error
            Ok(cpi_system.cpi_authority_pda)
        }
    }

    #[inline(always)]
    pub fn tree_pubkeys(&self) -> Vec<&'info Pubkey> {
        let mut pubkeys = Vec::with_capacity(4);

        if let Some(executing) = &self.executing {
            pubkeys.push(executing.out_output_queue.key());
            pubkeys.push(executing.in_merkle_tree.key());
            if let Some(in_queue) = executing.in_output_queue {
                pubkeys.push(in_queue.key());
            }
            if let Some(tokens_out_queue) = executing.tokens_out_queue {
                pubkeys.push(tokens_out_queue.key());
            }
        }
        msg!(
            "Tree pubkeys {:?}",
            pubkeys
                .iter()
                .map(|p| solana_pubkey::Pubkey::new_from_array(**p))
                .collect::<Vec<_>>()
        );
        pubkeys
    }

    /// Calculate the dynamic CPI accounts offset based on which accounts are present
    pub fn cpi_accounts_offset(&self) -> usize {
        let mut offset = 0;

        // light_system_program (always present)
        offset += 1;

        // mint_signer (optional)
        if self.mint_signer.is_some() {
            offset += 1;
        }

        // authority (always present)
        offset += 1;

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
        // CpiContextLightSystemAccounts - these are the CPI accounts that start here
        // We don't add them to offset since this is where CPI accounts begin

        offset
    }
}

#[derive(Debug)]
pub struct AccountsConfig {
    pub with_cpi_context: bool,
    pub write_to_cpi_context: bool,
    pub with_lamports: bool,
    pub is_decompressed: bool,
    pub has_mint_to_actions: bool,
    pub with_mint_signer: bool,
    pub create_mint: bool,
}

impl AccountsConfig {
    pub fn new(parsed_instruction_data: &ZMintActionCompressedInstructionData) -> AccountsConfig {
        let with_cpi_context = parsed_instruction_data.cpi_context.is_some();
        let write_to_cpi_context = parsed_instruction_data
            .cpi_context
            .as_ref()
            .map(|x| x.first_set_context() || x.set_context())
            .unwrap_or_default();
        let with_lamports = parsed_instruction_data
        .actions
        .iter()
        .any(|action| matches!(action, ZAction::MintTo(mint_to_action) if mint_to_action.lamports.is_some()));
        // Check if we have MintTo actions - needed for tokens_out_queue
        let has_mint_to_actions = parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::MintTo(_)));
        // TODO: differentiate between will be compressed or is compressed.
        let is_decompressed = parsed_instruction_data.mint.is_decompressed()
            | parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::CreateSplMint(_)));
        // We need mint signer if create mint, and create spl mint.
        let with_mint_signer = parsed_instruction_data.create_mint()
            | parsed_instruction_data
                .actions
                .iter()
                .any(|action| matches!(action, ZAction::CreateSplMint(_)));

        AccountsConfig {
            with_cpi_context,
            write_to_cpi_context,
            with_lamports,
            is_decompressed,
            has_mint_to_actions,
            with_mint_signer,
            create_mint: parsed_instruction_data.create_mint(),
        }
    }
}
