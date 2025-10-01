use anchor_lang::prelude::*;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    },
    instruction_data::{
        with_account_info::{
            CompressedAccountInfo, InAccountInfo, InstructionDataInvokeCpiWithAccountInfo,
        },
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
};
use light_sdk::{
    cpi::{
        invoke::invoke_light_system_program,
        v1::{CpiAccounts, LightSystemProgramCpi},
        CpiAccountsConfig, CpiAccountsTrait, InvokeLightSystemProgram, LightCpiInstruction,
        LightInstructionData,
    },
    error::LightSdkError,
};
use light_sdk_types::{cpi_context_write::CpiContextWriteAccounts, LIGHT_SYSTEM_PROGRAM_ID};

use crate::{GenericAnchorAccounts, LIGHT_CPI_SIGNER};

pub fn process_cpi_context_indexing_inputs<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, GenericAnchorAccounts<'info>>,
    mode: u8,
    leaf_indices: [u8; 3],
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

    // In mode 0, we can only consume program-owned accounts (indices 1 and 3)
    // In modes 1-2, all 4 accounts are program-owned
    match mode {
        0 => {
            // Mode 0: Only consume the 2 program-owned accounts
            // Account at index 1: Has default data (from CompressedAccountInfo)
            // Account at index 3: Has [9u8; 8] discriminator
            consume_mode_0_accounts(light_cpi_accounts)?;
        }
        1 | 2 => {
            // Modes 1-2: All 4 accounts are program-owned, consume all
            let (data, _owner) = match mode {
                1 => (Some(CompressedAccountData::default()), crate::ID),
                2 => (
                    Some(CompressedAccountData {
                        discriminator: [1u8; 8],
                        data: vec![2u8; 32],
                        data_hash: [3u8; 32],
                    }),
                    crate::ID,
                ),
                _ => unreachable!(),
            };
            consume_all_accounts(light_cpi_accounts, data, leaf_indices)?;
        }
        _ => panic!("Invalid mode"),
    }
    Ok(())
}

fn consume_mode_0_accounts(light_cpi_accounts: CpiAccounts) -> Result<()> {
    // In mode 0, only accounts at indices 1 and 3 are program-owned

    // Second account (index 1) - from CompressedAccountInfo, always has default data
    {
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: light_cpi_accounts.fee_payer(),
            authority: light_cpi_accounts.authority().unwrap(),
            cpi_context: light_cpi_accounts.cpi_context().unwrap(),
            cpi_signer: LIGHT_CPI_SIGNER,
        };

        let in_account = CompressedAccountInfo {
            input: Some(InAccountInfo {
                discriminator: [0u8; 8], // default discriminator
                data_hash: [0u8; 32],    // default data_hash
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 1, // Second account
                    prove_by_index: true,
                },
                root_index: 0,
                lamports: 0,
            }),
            output: None,
            address: None,
        };

        InstructionDataInvokeCpiWithAccountInfo::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_account_infos(&[in_account])
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    }

    // Fourth account (index 3) - always has [9u8; 8] discriminator
    let input_account = InAccount {
        discriminator: [9u8; 8],
        data_hash: [9u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: 3,
            prove_by_index: true,
        },
        root_index: 0,
        lamports: 0,
        address: None,
    };

    InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, None.into())
        .mode_v1()
        .with_input_compressed_accounts(&[input_account])
        .invoke_execute_cpi_context(light_cpi_accounts)?;

    Ok(())
}

fn consume_all_accounts(
    light_cpi_accounts: CpiAccounts,
    data: Option<CompressedAccountData>,
    leaf_indices: [u8; 3],
) -> Result<()> {
    // First CPI write_to_cpi_context_first - uses leaf_indices[0]
    {
        let discriminator = data.clone().map(|x| x.discriminator).unwrap_or_default();
        let data_hash = data.clone().map(|x| x.data_hash).unwrap_or_default();
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: light_cpi_accounts.fee_payer(),
            authority: light_cpi_accounts.authority().unwrap(),
            cpi_context: light_cpi_accounts.cpi_context().unwrap(),
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        let input_account = InAccount {
            discriminator,
            data_hash,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: leaf_indices[0] as u32,
                prove_by_index: true,
            },
            root_index: 0,
            lamports: 0,
            address: None,
        };

        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_input_compressed_accounts(&[input_account])
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;
    }

    // Second CPI write_to_cpi_context_set with CompressedAccountInfo - uses leaf_indices[1]
    {
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: light_cpi_accounts.fee_payer(),
            authority: light_cpi_accounts.authority().unwrap(),
            cpi_context: light_cpi_accounts.cpi_context().unwrap(),
            cpi_signer: LIGHT_CPI_SIGNER,
        };

        let discriminator = data.clone().map(|x| x.discriminator).unwrap_or_default();
        let data_hash = data.clone().map(|x| x.data_hash).unwrap_or_default();

        let in_account = CompressedAccountInfo {
            input: Some(InAccountInfo {
                discriminator,
                data_hash,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: leaf_indices[1] as u32,
                    prove_by_index: true,
                },
                root_index: 0,
                lamports: 0,
            }),
            output: None,
            address: None,
        };

        InstructionDataInvokeCpiWithAccountInfo::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_account_infos(&[in_account])
            .invoke_write_to_cpi_context_set(cpi_context_accounts)?;
    }

    // Third CPI write_to_cpi_context_set with PackedCompressedAccountWithMerkleContext - uses leaf_indices[2]
    {
        let compressed_account = CompressedAccount {
            address: None,
            data: data.clone(),
            lamports: 0,
            owner: crate::ID.into(),
        };

        let input_account = PackedCompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                leaf_index: leaf_indices[2] as u32,
                prove_by_index: true,
            },
            root_index: 0,
            read_only: false,
        };

        let instruction_data = LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, None.into())
            .with_input_compressed_accounts_with_merkle_context(&[input_account])
            .write_to_cpi_context_set();

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
    // Fourth account - leaf index 3 (always has [9u8; 8] discriminator)
    let input_account = InAccount {
        discriminator: [9u8; 8],
        data_hash: [9u8; 32],
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 0,
            queue_pubkey_index: 1,
            leaf_index: 3,
            prove_by_index: true,
        },
        root_index: 0,
        lamports: 0,
        address: None,
    };

    InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, None.into())
        .with_input_compressed_accounts(&[input_account])
        .mode_v1()
        .invoke_execute_cpi_context(light_cpi_accounts)?;

    Ok(())
}
