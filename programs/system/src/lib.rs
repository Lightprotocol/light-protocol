use context::WrappedInstructionData;
use init_context_account::init_cpi_context_account;
use invoke::instruction::InvokeInstruction;
use invoke_cpi::{
    instruction::InvokeCpiInstruction, processor::process_invoke_cpi,
    verify_signer::cpi_signer_checks,
};
use invoke_with_read_only_cpi::instruction::{
    InvokeCpiWithReadOnlyInstructionSmall, OptionsConfig,
};
use pinocchio::pubkey::Pubkey;

pub mod account_compression_state;
mod account_traits;
mod check_accounts;
pub mod constants;
pub mod context;
pub mod errors;
pub mod init_context_account;
pub mod invoke;
pub mod invoke_cpi;
pub mod invoke_with_read_only_cpi;
pub mod processor;
pub mod utils;

use errors::SystemProgramError;
use light_macros::pubkey;

use crate::account_traits::SignerAccounts;

pub const ID: Pubkey = pubkey!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_system_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}
use light_compressed_account::instruction_data::{
    traits::InstructionDataTrait,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
    zero_copy::{ZInstructionDataInvoke, ZInstructionDataInvokeCpi},
};
use light_zero_copy::borsh::Deserialize;
use pinocchio::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, msg,
    program_error::ProgramError, ProgramResult,
};

use crate::{
    invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
};

pub type Result<T> = std::result::Result<T, ProgramError>;

pub enum InstructionDiscriminator {
    InitializeCpiContextAccount,
    Invoke,
    InvokeCpi,
    InvokeCpiWithReadOnly,
    InvokeCpiWithAccountInfo,
}
pub const INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];
pub const INVOKE_INSTRUCTION: [u8; 8] = [26, 16, 169, 7, 21, 202, 242, 25];
pub const INVOKE_CPI_INSTRUCTION: [u8; 8] = [49, 212, 191, 129, 39, 194, 43, 196];
pub const INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION: [u8; 8] = [86, 47, 163, 166, 21, 223, 92, 8];
pub const CPI_CONTEXT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];
pub const INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 167];

impl TryFrom<&[u8]> for InstructionDiscriminator {
    type Error = crate::errors::SystemProgramError;

    // TODO: throw better errors
    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        let array: [u8; 8] = value
            .try_into()
            .map_err(|_| crate::errors::SystemProgramError::InvalidArgument)?;
        match array {
            INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION => {
                Ok(InstructionDiscriminator::InitializeCpiContextAccount)
            }
            INVOKE_INSTRUCTION => Ok(InstructionDiscriminator::Invoke),
            INVOKE_CPI_INSTRUCTION => Ok(InstructionDiscriminator::InvokeCpi),
            INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION => {
                Ok(InstructionDiscriminator::InvokeCpiWithReadOnly)
            }
            INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION => {
                Ok(InstructionDiscriminator::InvokeCpiWithAccountInfo)
            }
            _ => Err(SystemProgramError::InvalidArgument),
        }
    }
}

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
    let discriminator = InstructionDiscriminator::try_from(discriminator).unwrap();
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
        &ctx.authority.key(),
    )?;
    let wrapped_inputs = context::WrappedInstructionData::new(inputs, None);
    process(wrapped_inputs, None, &ctx, 0, remaining_accounts)?;
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
    // msg!(format!(
    //     "accounts {:?}",
    //     accounts.iter().map(|x| x.key()).collect::<Vec<_>>()
    // )
    // .as_str());
    let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;
    // msg!(format!(
    //     "remaining_accounts {:?}",
    //     remaining_accounts
    //         .iter()
    //         .map(|x| x.key())
    //         .collect::<Vec<_>>()
    // )
    // .as_str());
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_checks(
        &ctx.invoking_program.key(),
        &ctx.get_authority().key(),
        &inputs.output_compressed_accounts,
    )?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_signer_checks");
    let wrapped_inputs = WrappedInstructionData::new(inputs, None);
    process_invoke_cpi(
        *ctx.invoking_program.key(),
        ctx,
        wrapped_inputs,
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
    msg!(format!("inputs {:?}", inputs).as_str());

    shared_invoke_cpi(
        accounts,
        inputs.invoking_program_id.into(),
        inputs.mode,
        inputs,
    )
}

pub fn invoke_cpi_with_account_info<'a, 'b, 'c: 'info, 'info>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()> {
    let instruction_data = &instruction_data[4..];

    let (inputs, _) = InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    shared_invoke_cpi(
        accounts,
        inputs.invoking_program_id.into(),
        inputs.mode,
        inputs,
    )
}

fn shared_invoke_cpi<'a, 'info, T: InstructionDataTrait<'a>>(
    accounts: &[AccountInfo],
    invoking_program: Pubkey,
    mode: u8,
    inputs: T,
) -> Result<()> {
    let account_options = OptionsConfig {
        sol_pool_pda: inputs.is_compress(),
        decompression_recipient: inputs.compress_or_decompress_lamports().is_some()
            && !inputs.is_compress(),
        cpi_context_account: inputs.cpi_context().is_some(),
    };
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_deserialize");
    // disable set cpi context because cpi context account uses InvokeCpiInstruction
    if let Some(cpi_context) = inputs.cpi_context() {
        if cpi_context.set_context {
            msg!("Cannot set cpi context in invoke_cpi_with_read_only.");
            msg!("Please use invoke_cpi instead.");
            return Err(SystemProgramError::InstructionNotCallable.into());
        }
    }

    if mode == 0 {
        let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;
        #[cfg(feature = "bench-sbf")]
        bench_sbf_start!("cpda_cpi_signer_checks");
        cpi_signer_checks(
            &invoking_program,
            &ctx.get_authority().key(),
            &inputs.output_accounts(),
        )?;
        #[cfg(feature = "bench-sbf")]
        bench_sbf_end!("cpda_cpi_signer_checks");
        process_invoke_cpi(
            invoking_program,
            ctx,
            WrappedInstructionData::new(inputs, None),
            remaining_accounts,
        )
    } else {
        let (ctx, remaining_accounts) =
            InvokeCpiWithReadOnlyInstructionSmall::from_account_infos(accounts, account_options)?;
        #[cfg(feature = "bench-sbf")]
        bench_sbf_start!("cpda_cpi_signer_checks");
        cpi_signer_checks(
            &invoking_program,
            &ctx.get_authority().key(),
            &inputs.output_accounts(),
        )?;
        #[cfg(feature = "bench-sbf")]
        bench_sbf_end!("cpda_cpi_signer_checks");

        process_invoke_cpi(
            invoking_program,
            ctx,
            WrappedInstructionData::new(inputs, None),
            remaining_accounts,
        )
    }
}
