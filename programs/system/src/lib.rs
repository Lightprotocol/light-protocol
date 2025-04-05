use init_context_account::init_cpi_context_account;
use invoke::instruction::InvokeInstruction;
use invoke_cpi::{
    instruction::InvokeCpiInstruction, processor::process_invoke_cpi,
};
use light_account_checks::context::LightContext;
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

pub const ID: Pubkey = pubkey!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_system_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}
use pinocchio::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units,
    program_error::ProgramError, ProgramResult,
};

use crate::{
    invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
};
use light_compressed_account::instruction_data::{
    zero_copy::{ZInstructionDataInvoke, ZInstructionDataInvokeCpi},
};

use light_zero_copy::borsh::Deserialize;

pub type Result<T> = std::result::Result<T, ProgramError>;

pub enum InstructionDiscriminator {
    InitializeCpiContextAccount,
    Invoke,
    InvokeCpi,
    InvokeCpiWithReadOnly,
}
pub const INIT_CPI_CONTEXT_ACCOUNT_INSTRUCTION: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];
pub const INVOKE_INSTRUCTION: [u8; 8] = [26, 16, 169, 7, 21, 202, 242, 25];
pub const INVOKE_CPI_INSTRUCTION: [u8; 8] = [49, 212, 191, 129, 39, 194, 43, 196];
pub const INVOKE_CPI_WITH_READ_ONLY_INSTRUCTION: [u8; 8] = [86, 47, 163, 166, 21, 223, 92, 8];
pub const CPI_CONTEXT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];

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
        // InstructionDiscriminator::InvokeCpiWithReadOnly => {
        //     invoke_cpi_with_read_only(accounts, instruction_data)
        // }
        _ => panic!(""),
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
    process(
        wrapped_inputs,
        None,
        &ctx,
        0,
        None,
        None,
        remaining_accounts,
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
    process_invoke_cpi(ctx, inputs, None, None, remaining_accounts)?;
    sol_log_compute_units();
    // 22,903 bytes heap with 33 outputs
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("total_usage");
    Ok(())
}

// #[allow(unused_variables)]
// pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
//     accounts: &[AccountInfo],
//     instruction_data: &[u8],
// ) -> Result<()> {
//     #[cfg(not(feature = "readonly"))]
//     {
//         msg!("Readonly feature is not enabled.");

//         return Err(SystemProgramError::InstructionNotCallable.into());
//     }
//     let instruction_data = &instruction_data[4..];

//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_start!("cpda_deserialize");
//     #[allow(unreachable_code)]
//     {
//         let (inputs, _) =
//             InstructionDataInvokeCpiWithReadOnly::zero_copy_at(instruction_data).unwrap();
//         let account_options = OptionsConfig {
//             sol_pool_pda: inputs.is_compress,
//             decompression_recipient: inputs.decompression_amount > 0 && !inputs.is_compress,
//             cpi_context_account: inputs.with_cpi_context(),
//         };
//         let (ctx, remaining_accounts): (impl InvokeAccounts + SignerAccounts, &[AccountInfo]) =
//             if inputs.mode == 0 {
//                 <InvokeCpiInstruction<'_> as LightContext<'_>>::from_account_infos(accounts, None)?
//             } else {
//                 <InvokeCpiWithReadOnlyInstructionSmall<'_> as LightContext<'_>>::from_account_infos(
//                     accounts,
//                     Some(account_options),
//                 )?
//             };

//         #[cfg(feature = "bench-sbf")]
//         bench_sbf_end!("cpda_deserialize");
//         // disable set cpi context because cpi context account uses InvokeCpiInstruction
//         if let Some(cpi_context) = inputs.invoke_cpi.cpi_context {
//             if cpi_context.set_context() {
//                 msg!("Cannot set cpi context in invoke_cpi_with_read_only.");
//                 msg!("Please use invoke_cpi instead.");
//                 return Err(SystemProgramError::InstructionNotCallable.into());
//             }
//         }
//         // msg!(format!(
//         //     "remaining_accounts {:?}",
//         //     remaining_accounts
//         //         .iter()
//         //         .map(|x| x.key())
//         //         .collect::<Vec<_>>()
//         // )
//         // .as_str());
//         process_invoke_cpi(
//             ctx,
//             inputs.invoke_cpi,
//             Some(inputs.read_only_addresses),
//             Some(inputs.read_only_accounts),
//             remaining_accounts,
//         )
//     }
// }
