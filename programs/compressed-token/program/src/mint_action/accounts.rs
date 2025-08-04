use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::shared::{
    accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
    AccountIterator,
};

pub struct MintActionAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub mint_signer: Option<&'info AccountInfo>,
    pub authority: &'info AccountInfo,
    pub executing: Option<ExecutingAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
}

pub struct ExecutingAccounts<'info> {
    pub mint: Option<&'info AccountInfo>,
    pub token_pool_pda: Option<&'info AccountInfo>,
    pub token_program: Option<&'info AccountInfo>,
    pub system: LightSystemAccounts<'info>,
    pub out_output_queue: &'info AccountInfo,
    pub in_merkle_tree: Option<&'info AccountInfo>,
    pub in_output_queue: Option<&'info AccountInfo>,
    pub tokens_out_queue: Option<&'info AccountInfo>,
}

impl<'info> MintActionAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_lamports: bool,
        is_decompressed: bool,
        with_mint_signer: bool,
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;
        // TODO: make it option signer
        let mint_signer = iter.next_option("mint_signer", with_mint_signer)?;
        // Static non-CPI accounts first
        let authority = iter.next_signer("authority")?;
        if write_to_cpi_context {
            Ok(MintActionAccounts {
                light_system_program,
                mint_signer,
                authority,
                executing: None,
                write_to_cpi_context_system: Some(
                    CpiContextLightSystemAccounts::validate_and_parse(&mut iter)?,
                ),
            })
        } else {
            let mint = iter.next_option_mut("mint", is_decompressed)?;
            let token_pool_pda = iter.next_option_mut("token_pool_pda", is_decompressed)?;
            let token_program = iter.next_option("token_program", is_decompressed)?;

            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                with_lamports,
                false,
                with_cpi_context,
            )?;

            let out_output_queue = iter.next_account("out_output_queue")?;
            let in_merkle_tree = iter.next_option("in_merkle_tree", is_decompressed)?;
            let in_output_queue = iter.next_option("in_output_queue", is_decompressed)?;
            let tokens_out_queue = iter.next_option("tokens_out_queue", is_decompressed)?;

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
            if let Some(in_tree) = executing.in_merkle_tree {
                pubkeys.push(in_tree.key());
            }
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
        } else if let Some(_) = &self.write_to_cpi_context_system {
            // CpiContextLightSystemAccounts - these are the CPI accounts that start here
            // We don't add them to offset since this is where CPI accounts begin
        }

        offset
    }
}
