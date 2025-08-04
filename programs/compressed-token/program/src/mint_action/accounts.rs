use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{
    account_info::AccountInfo,
    pubkey::{self, Pubkey},
};
use solana_pubkey::PUBKEY_BYTES;

use crate::shared::{
    accounts::{
        CpiContextLightSystemAccounts, LightSystemAccounts, UpdateOneCompressedAccountTreeAccounts,
    },
    AccountIterator,
};

pub struct MintActionAccounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub mint_signer: &'info AccountInfo,
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
        with_cpi_context: bool,
        write_to_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let light_system_program = iter.next_account("light_system_program")?;
        let mint_signer = iter.next_account("mint_signer")?;
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

        pubkeys
    }
}
