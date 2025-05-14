#![allow(clippy::too_many_arguments)]

use account_compression::utils::constants::{CPI_AUTHORITY_PDA_SEED, NOOP_PUBKEY};
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, pubkey::Pubkey},
    InstructionData,
};
use light_system_program::utils::get_registered_program_pda;
pub mod create_pda;
pub use create_pda::*;
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, data::NewAddressParamsPacked,
};
use light_sdk::cpi::accounts::CompressionCpiAccountsConfig;
declare_id!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[program]
pub mod system_cpi_test {

    use light_sdk::cpi::verify::invoke_light_system_program;

    use super::*;

    pub fn create_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        data: [u8; 31],
        proof: Option<CompressedProof>,
        new_address_parameters: NewAddressParamsPacked,
        bump: u8,
    ) -> Result<()> {
        process_create_pda(ctx, data, proof, new_address_parameters, bump)
    }

    /// Wraps system program invoke cpi
    /// This instruction is for tests only. It is insecure, do not use as
    /// inspiration to build a program with compressed accounts.
    pub fn invoke_cpi<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        inputs: Vec<u8>,
        bump: u8,
    ) -> Result<()> {
        process_invoke_cpi(&ctx, inputs, bump)
    }

    /// Test wrapper, for with read-only and with account info instructions.
    pub fn invoke_with_read_only<'info>(
        ctx: Context<'_, '_, '_, 'info, InvokeCpiReadOnly<'info>>,
        config: CompressionCpiAccountsConfig,
        small_ix: bool,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let fee_payer = ctx.accounts.signer.to_account_info();

        let (account_infos, account_metas) = if small_ix {
            use light_sdk::cpi::accounts_small_ix::CompressionCpiAccounts;
            let cpi_accounts =
                CompressionCpiAccounts::new_with_config(&fee_payer, ctx.remaining_accounts, config)
                    .map_err(ProgramError::from)?;
            let account_infos = cpi_accounts.to_account_infos();

            let account_metas = cpi_accounts.to_account_metas();
            (account_infos, account_metas)
        } else {
            use light_sdk::cpi::accounts::CompressionCpiAccounts;
            let cpi_accounts =
                CompressionCpiAccounts::new_with_config(&fee_payer, ctx.remaining_accounts, config)
                    .map_err(ProgramError::from)?;
            let account_infos = cpi_accounts.to_account_infos();

            let account_metas = cpi_accounts.to_account_metas();
            (account_infos, account_metas)
        };

        invoke_light_system_program(&crate::ID, &account_infos, account_metas, inputs)
            .map_err(ProgramError::from)?;
        Ok(())
    }

    pub fn invoke_cpi_multiple<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        inputs: Vec<u8>,
        bump: u8,
        num_invocations: u8,
    ) -> Result<()> {
        for i in 0..num_invocations {
            msg!("invoke_cpi_multiple cpi {}", i);
            process_invoke_cpi(&ctx, inputs.clone(), bump)?;
        }
        Ok(())
    }
}

pub fn process_invoke_cpi<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    inputs: Vec<u8>,
    bump: u8,
) -> Result<()> {
    anchor_lang::solana_program::log::sol_log_compute_units();
    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    anchor_lang::solana_program::log::sol_log_compute_units();
    light_system_program::cpi::invoke_cpi_with_read_only(cpi_ctx, inputs)?;
    Ok(())
}

pub fn create_invoke_cpi_instruction(
    signer: Pubkey,
    inputs: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
    num_invocations: Option<u8>,
) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::ID);

    let ix_data = if let Some(num_invocations) = num_invocations {
        crate::instruction::InvokeCpiMultiple {
            bump,
            inputs,
            num_invocations,
        }
        .data()
    } else {
        crate::instruction::InvokeCpi { bump, inputs }.data()
    };
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let registered_program_pda = get_registered_program_pda(&light_system_program::id());
    let accounts = crate::accounts::CreateCompressedPda {
        signer,
        light_system_program: light_system_program::id(),
        account_compression_authority,
        cpi_signer,
        registered_program_pda,
        noop_program: Pubkey::from(NOOP_PUBKEY),
        account_compression_program: account_compression::id(),
        self_program: crate::id(),
        system_program: Pubkey::default(),
    };
    Instruction {
        program_id: crate::id(),
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: ix_data,
    }
}

#[derive(Accounts)]
pub struct InvokeCpiReadOnly<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

pub fn create_invoke_read_only_account_info_instruction(
    signer: Pubkey,
    inputs: Vec<u8>,
    config: CompressionCpiAccountsConfig,
    small: bool,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let ix_data = crate::instruction::InvokeWithReadOnly {
        small_ix: small,
        inputs,
        config,
    }
    .data();
    let accounts = crate::accounts::InvokeCpiReadOnly { signer };
    Instruction {
        program_id: crate::id(),
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: ix_data,
    }
}
