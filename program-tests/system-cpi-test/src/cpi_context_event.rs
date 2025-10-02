use anchor_lang::prelude::*;
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_account_info::InstructionDataInvokeCpiWithAccountInfo,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
};
use light_sdk::{
    cpi::{
        invoke::invoke_light_system_program,
        v1::{CpiAccounts, LightSystemProgramCpi},
        v2::lowlevel::{CompressedAccountInfo, OutAccountInfo},
        CpiAccountsConfig, CpiAccountsTrait, InvokeLightSystemProgram, LightCpiInstruction,
    },
    error::LightSdkError,
};
use light_sdk_types::{cpi_context_write::CpiContextWriteAccounts, LIGHT_SYSTEM_PROGRAM_ID};

use crate::LIGHT_CPI_SIGNER;

pub fn process_cpi_context_indexing<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, GenericAnchorAccounts<'info>>,
    mode: u8,
) -> Result<()> {
    let fee_payer = ctx.accounts.signer.to_account_info();
    let light_cpi_accounts = CpiAccounts::new_with_config(
        &fee_payer,
        ctx.remaining_accounts,
        CpiAccountsConfig {
            cpi_context: true,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            sol_compression_recipient: false,
            sol_pool_pda: false,
        },
    );
    let (data, owner) = match mode {
        0 => (None, Pubkey::default()),
        1 => (Some(CompressedAccountData::default()), crate::ID),
        2 => (
            Some(CompressedAccountData {
                discriminator: [1u8; 8],
                data: vec![2u8; 32],
                data_hash: [3u8; 32],
            }),
            crate::ID,
        ),
        _ => panic!("Invalid mode"),
    };

    let merkle_tree_index = 0;
    {
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: &ctx.accounts.signer.to_account_info(),
            authority: light_cpi_accounts.authority().unwrap(),
            cpi_context: light_cpi_accounts.cpi_context().unwrap(),
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        let out_account = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                address: None,
                data: data.clone(),
                lamports: 0,
                owner: owner.into(),
            },
            merkle_tree_index,
        };
        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_output_compressed_accounts(&[out_account])
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    }
    {
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: &ctx.accounts.signer.to_account_info(),
            authority: light_cpi_accounts.authority().unwrap(),
            cpi_context: light_cpi_accounts.cpi_context().unwrap(),
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        let discriminator = data.clone().map(|x| x.discriminator).unwrap_or_default();
        let data_hash = data.clone().map(|x| x.data_hash).unwrap_or_default();
        let data = data.clone().map(|x| x.data).unwrap_or_default();
        let out_account = CompressedAccountInfo {
            input: None,
            output: Some(OutAccountInfo {
                discriminator,
                data,
                data_hash,
                ..Default::default()
            }),
            address: None,
        };
        InstructionDataInvokeCpiWithAccountInfo::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_account_infos(&[out_account])
            .invoke_write_to_cpi_context_set(cpi_context_accounts)?;
    }
    {
        let out_account = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                address: None,
                data: data.clone(),
                lamports: 0,
                owner: owner.into(),
            },
            merkle_tree_index,
        };
        let instruction_data = LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_output_compressed_accounts(&[out_account])
            .write_to_cpi_context_set();
        use light_sdk::cpi::LightInstructionData;
        let data = instruction_data
            .data()
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?;

        let account_infos = light_cpi_accounts.to_account_infos();

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
            accounts: light_cpi_accounts.to_account_metas().unwrap(),
            data,
        };

        invoke_light_system_program(&account_infos, instruction, instruction_data.get_bump())?;
    }
    let out_account = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            address: None,
            data: Some(CompressedAccountData {
                discriminator: [9u8; 8],
                data: vec![],
                data_hash: [9u8; 32],
            }),
            lamports: 0,
            owner: crate::ID.into(),
        },
        merkle_tree_index,
    };
    InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, None.into())
        .mode_v1()
        .with_output_compressed_accounts(&[out_account])
        .invoke_execute_cpi_context(light_cpi_accounts)?;
    Ok(())
}

#[derive(Accounts)]
pub struct GenericAnchorAccounts<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}
