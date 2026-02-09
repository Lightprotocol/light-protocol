pub mod constants;
pub mod invoke;
pub mod invoke_cpi;

// Re-export everything from light-vm so downstream consumers
// can still import from this crate.
use accounts::{init_context_account::init_cpi_context_account, mode::AccountMode};
pub use constants::*;
use invoke::instruction::InvokeInstruction;
use invoke_cpi::{instruction::InvokeCpiInstruction, instruction_v2::InvokeCpiInstructionV2};
use light_compressed_account::instruction_data::{
    traits::InstructionData,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
    zero_copy::{ZInstructionDataInvoke, ZInstructionDataInvokeCpi},
};
use light_macros::pubkey_array;
use light_vm::Processor;
pub use light_vm::{
    account_compression_state, accounts, context, cpi_context, errors, processor, utils,
};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::invoke::verify_signer::input_compressed_accounts_signer_check;

pub const ID: Pubkey = pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

pub struct LightSystemProgram;

impl Processor for LightSystemProgram {
    const ID: Pubkey = ID;
}

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
    ReInitCpiContextAccount,
}
#[cfg(not(feature = "no-entrypoint"))]
use pinocchio::entrypoint;
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
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
        InstructionDiscriminator::InitializeCpiContextAccount => init_cpi_context_account(accounts),
        InstructionDiscriminator::Invoke => invoke(accounts, instruction_data),
        InstructionDiscriminator::InvokeCpi => invoke_cpi(accounts, instruction_data),
        InstructionDiscriminator::InvokeCpiWithReadOnly => {
            invoke_cpi_with_read_only(accounts, instruction_data)
        }
        InstructionDiscriminator::InvokeCpiWithAccountInfo => {
            invoke_cpi_with_account_info(accounts, instruction_data)
        }
        #[cfg(feature = "reinit")]
        InstructionDiscriminator::ReInitCpiContextAccount => {
            LightSystemProgram::reinit_cpi_context_account(accounts)
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

    let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(instruction_data)?;
    let (ctx, remaining_accounts) = InvokeInstruction::from_account_infos(accounts)?;

    input_compressed_accounts_signer_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        ctx.authority.key(),
    )?;
    let wrapped_inputs = context::WrappedInstructionData::new(inputs)?;
    LightSystemProgram::process::<false, InvokeInstruction, ZInstructionDataInvoke>(
        wrapped_inputs,
        None,
        &ctx,
        0,
        remaining_accounts,
    )?;
    Ok(())
}

pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    // remove vec prefix
    let instruction_data = &instruction_data[4..];

    let (inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(instruction_data)?;

    let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;

    LightSystemProgram::process_invoke_cpi::<false, InvokeCpiInstruction, ZInstructionDataInvokeCpi>(
        *ctx.invoking_program.key(),
        ctx,
        inputs,
        remaining_accounts,
    )?;
    Ok(())
}

pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    msg!("invoke_cpi_with_read_only");
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

    let (inputs, _) = InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    shared_invoke_cpi(
        accounts,
        inputs.invoking_program_id.into(),
        AccountMode::try_from(inputs.mode)?,
        inputs,
    )
}

#[inline(never)]
fn shared_invoke_cpi<'a, 'info, T: InstructionData<'a> + 'a>(
    accounts: &[AccountInfo],
    invoking_program: Pubkey,
    mode: AccountMode,
    inputs: T,
) -> Result<()> {
    msg!(format!("mode {:?}", mode).as_str());
    match mode {
        AccountMode::Anchor => {
            let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;
            LightSystemProgram::process_invoke_cpi::<true, InvokeCpiInstruction, T>(
                invoking_program,
                ctx,
                inputs,
                remaining_accounts,
            )
        }
        AccountMode::V2 => {
            let (ctx, remaining_accounts) = InvokeCpiInstructionV2::from_account_infos(
                accounts,
                inputs.account_option_config()?,
            )?;
            LightSystemProgram::process_invoke_cpi::<true, InvokeCpiInstructionV2, T>(
                invoking_program,
                ctx,
                inputs,
                remaining_accounts,
            )
        }
    }
}
