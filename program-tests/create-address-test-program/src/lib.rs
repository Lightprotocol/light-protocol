#![allow(clippy::too_many_arguments)]
#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use account_compression::utils::constants::{CPI_AUTHORITY_PDA_SEED, NOOP_PUBKEY};
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, pubkey::Pubkey},
    InstructionData,
};
use light_sdk::{
    cpi::{v2::CpiAccounts, CpiAccountsConfig, CpiSigner},
    derive_light_cpi_signer,
    error::LightSdkError,
};
use light_sdk_types::cpi_accounts::v2::{CompressionCpiAccountIndex, PROGRAM_ACCOUNTS_LEN};
use light_system_program::utils::get_registered_program_pda;
pub mod create_pda;
pub use create_pda::*;
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, data::NewAddressParamsPacked,
};
use light_sdk::{
    constants::LIGHT_SYSTEM_PROGRAM_ID,
    cpi::{
        invoke::invoke_light_system_program,
        v1::lowlevel::{get_account_metas_from_config, CpiInstructionConfig},
        v2::lowlevel::to_account_metas,
    },
};
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct CpiAccountsConfigLocal {
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
}
impl From<CpiAccountsConfigLocal> for CpiAccountsConfig {
    fn from(config: CpiAccountsConfigLocal) -> Self {
        CpiAccountsConfig {
            cpi_context: config.cpi_context,
            sol_compression_recipient: config.sol_compression_recipient,
            sol_pool_pda: config.sol_pool_pda,
            cpi_signer: LIGHT_CPI_SIGNER,
        }
    }
}
impl From<CpiAccountsConfig> for CpiAccountsConfigLocal {
    fn from(config: CpiAccountsConfig) -> Self {
        CpiAccountsConfigLocal {
            cpi_context: config.cpi_context,
            sol_compression_recipient: config.sol_compression_recipient,
            sol_pool_pda: config.sol_pool_pda,
        }
    }
}

declare_id!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[program]
pub mod system_cpi_test {

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
        config: CpiAccountsConfigLocal,
        v2_ix: bool,
        inputs: Vec<u8>,
        write_cpi_context: bool,
    ) -> Result<()> {
        let fee_payer = ctx.accounts.signer.to_account_info();

        let (account_infos, account_metas) = if v2_ix {
            let cpi_accounts =
                CpiAccounts::new_with_config(&fee_payer, ctx.remaining_accounts, config.into());
            let account_infos = cpi_accounts.to_account_infos();

            let account_metas = if !write_cpi_context {
                to_account_metas(&cpi_accounts).map_err(|_| ErrorCode::AccountNotEnoughKeys)?
            } else {
                require!(
                    ctx.remaining_accounts.len() >= 3,
                    ErrorCode::AccountNotEnoughKeys
                );
                let mut account_metas = vec![];
                account_metas.push(AccountMeta {
                    pubkey: *cpi_accounts.fee_payer().key,
                    is_signer: true,
                    is_writable: true,
                });
                account_metas.push(AccountMeta {
                    pubkey: *ctx.remaining_accounts[1].key,
                    is_signer: true,
                    is_writable: false,
                });
                let account = &ctx.remaining_accounts[2];
                account_metas.push(AccountMeta {
                    pubkey: *account.key,
                    is_signer: false,
                    is_writable: true,
                });
                account_metas
            };
            (account_infos, account_metas)
        } else {
            use light_sdk::cpi::v1::CpiAccounts;
            let cpi_accounts =
                CpiAccounts::new_with_config(&fee_payer, ctx.remaining_accounts, config.into());

            let account_infos = cpi_accounts.to_account_infos();

            let config = CpiInstructionConfig::try_from(&cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?;
            let account_metas = get_account_metas_from_config(config);
            (account_infos, account_metas)
        };
        let instruction = Instruction {
            program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
            accounts: account_metas,
            data: inputs,
        };
        let cpi_config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
        invoke_light_system_program(&account_infos, instruction, cpi_config.bump())?;
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
    let mut account_infos = cpi_accounts.to_account_infos();

    // Add remaining accounts
    account_infos.extend_from_slice(ctx.remaining_accounts);

    // Create instruction
    let mut account_metas = cpi_accounts.to_account_metas(None);
    ctx.remaining_accounts.iter().for_each(|account| {
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: account.is_signer,
            is_writable: account.is_writable,
        });
    });
    let instruction = Instruction {
        program_id: ctx.accounts.light_system_program.key(),
        accounts: account_metas,
        data: inputs,
    };

    anchor_lang::solana_program::log::sol_log_compute_units();

    // Invoke the instruction with signer seeds
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        &account_infos,
        &signer_seeds,
    )?;
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
    config: CpiAccountsConfig,
    v2_ix: bool,
    remaining_accounts: Vec<AccountMeta>,
    write_cpi_context: bool,
) -> Instruction {
    let ix_data = crate::instruction::InvokeWithReadOnly {
        v2_ix,
        inputs,
        config: config.into(),
        write_cpi_context,
    }
    .data();
    let accounts = crate::accounts::InvokeCpiReadOnly { signer };
    Instruction {
        program_id: crate::id(),
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: ix_data,
    }
}
// Manual impl for failing tests
pub fn to_account_metas_small(
    cpi_accounts: CpiAccounts<'_, '_>,
) -> light_sdk::error::Result<Vec<AccountMeta>> {
    // TODO: do a version with a const array instead of vector.
    let extra = cpi_accounts
        .account_infos()
        .len()
        .saturating_sub(PROGRAM_ACCOUNTS_LEN);
    let mut account_metas = Vec::with_capacity(1 + extra);

    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.fee_payer().key,
        is_signer: true,
        is_writable: true,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.authority()?.key,
        is_signer: true,
        is_writable: false,
    });

    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.registered_program_pda()?.key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.account_compression_authority()?.key,
        is_signer: false,
        is_writable: false,
    });

    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.account_compression_program()?.key,
        is_signer: false,
        is_writable: false,
    });

    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.system_program()?.key,
        is_signer: false,
        is_writable: false,
    });

    let accounts = cpi_accounts.account_infos();
    let mut index = CompressionCpiAccountIndex::SolPoolPda as usize;

    if cpi_accounts.config().sol_pool_pda {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }
    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }
    assert_eq!(cpi_accounts.system_accounts_end_offset(), index);

    let tree_accounts = accounts
        .get(index..)
        .ok_or(LightSdkError::CpiAccountsIndexOutOfBounds(index))?;
    tree_accounts.iter().for_each(|acc| {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: true,
        });
    });

    Ok(account_metas)
}
