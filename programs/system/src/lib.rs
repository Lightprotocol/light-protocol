pub mod account_compression_state;
pub mod accounts;
pub mod constants;
pub mod context;
pub mod errors;
pub mod invoke;
pub mod invoke_cpi;
pub mod processor;
pub mod utils;

use accounts::{init_context_account::init_cpi_context_account, mode::AccountMode};
pub use constants::*;
use invoke::instruction::InvokeInstruction;
use invoke_cpi::{
    instruction::InvokeCpiInstruction, processor::process_invoke_cpi,
    small_accounts::InvokeCpiInstructionSmall,
};
use light_compressed_account::instruction_data::{
    traits::InstructionData,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
    zero_copy::{ZInstructionDataInvoke, ZInstructionDataInvokeCpi},
};
use light_macros::pubkey;
use light_zero_copy::borsh::Deserialize;
use pinocchio::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, msg,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::{
    invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
};

pub const ID: Pubkey = pubkey!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_system_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

pub type Result<T> = std::result::Result<T, ProgramError>;

pub enum InstructionDiscriminator {
    InitializeCpiContextAccount,
    Invoke,
    InvokeCpi,
    InvokeCpiWithReadOnly,
    InvokeCpiWithAccountInfo,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    sol_log_compute_units();
    if *program_id != ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (discriminator, instruction_data) = instruction_data.split_at(8);
    let discriminator =
        InstructionDiscriminator::try_from(discriminator).map_err(ProgramError::from)?;
    match discriminator {
        InstructionDiscriminator::InitializeCpiContextAccount => {
            init_cpi_context_account(accounts, instruction_data)
        }
        InstructionDiscriminator::Invoke => invoke(accounts, instruction_data),
        InstructionDiscriminator::InvokeCpi => invoke_cpi(accounts, instruction_data),
        InstructionDiscriminator::InvokeCpiWithReadOnly => {
            invoke_cpi_with_read_only(accounts, instruction_data)
        }
        InstructionDiscriminator::InvokeCpiWithAccountInfo => {
            invoke_cpi_with_account_info(accounts, instruction_data)
        }
    }?;
    Ok(())
}

pub fn invoke<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    // remove vec prefix
    let instruction_data = &instruction_data[4..];
    sol_log_compute_units();

    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("invoke_deserialize");
    let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(instruction_data).unwrap();
    let (ctx, remaining_accounts) = InvokeInstruction::from_account_infos(accounts)?;
    sol_log_compute_units();
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("invoke_deserialize");
    input_compressed_accounts_signer_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        ctx.authority.key(),
    )?;
    let wrapped_inputs = context::WrappedInstructionData::new(inputs);
    process::<false, InvokeInstruction, ZInstructionDataInvoke>(
        wrapped_inputs,
        None,
        &ctx,
        0,
        remaining_accounts,
        None,
    )?;
    sol_log_compute_units();
    Ok(())
}

pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    let instruction_data = &instruction_data[4..];

    sol_log_compute_units();
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_deserialize");
    let (inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(instruction_data).unwrap();
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_deserialize");
    let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;

    process_invoke_cpi::<false, InvokeCpiInstruction, ZInstructionDataInvokeCpi>(
        *ctx.invoking_program.key(),
        ctx,
        inputs,
        remaining_accounts,
    )?;
    sol_log_compute_units();
    // 22,903 bytes heap with 33 outputs
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("total_usage");
    Ok(())
}

pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    let instruction_data = &instruction_data[4..];
    msg!("invoke_cpi_with_read_only");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_deserialize");
    #[allow(unreachable_code)]
    let (inputs, _) = InstructionDataInvokeCpiWithReadOnly::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    shared_invoke_cpi(
        accounts,
        inputs.invoking_program_id.into(),
        AccountMode::try_from(inputs.mode)?,
        inputs,
    )
}

pub fn invoke_cpi_with_account_info<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    msg!("invoke_cpi_with_account_info");
    let instruction_data = &instruction_data[4..];

    let (inputs, _) = InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    shared_invoke_cpi(
        accounts,
        inputs.invoking_program_id.into(),
        AccountMode::try_from(inputs.mode)?,
        inputs,
    )
}

fn shared_invoke_cpi<'a, 'info, T: InstructionData<'a>>(
    accounts: &[AccountInfo],
    invoking_program: Pubkey,
    mode: AccountMode,
    inputs: T,
) -> Result<()> {
    msg!(format!("mode {:?}", mode).as_str());
    match mode {
        AccountMode::Anchor => {
            let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;
            process_invoke_cpi::<true, InvokeCpiInstruction, T>(
                invoking_program,
                ctx,
                inputs,
                remaining_accounts,
            )
        }
        AccountMode::Small => {
            let (ctx, remaining_accounts) = InvokeCpiInstructionSmall::from_account_infos(
                accounts,
                inputs.account_option_config(),
            )?;
            process_invoke_cpi::<true, InvokeCpiInstructionSmall, T>(
                invoking_program,
                ctx,
                inputs,
                remaining_accounts,
            )
        }
    }
}
