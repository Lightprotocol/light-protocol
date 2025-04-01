use invoke::instruction::InvokeInstruction;
use invoke_cpi::account::CpiContextAccount;
use light_account_checks::{
    checks::check_signer, discriminator::Discriminator as LightDiscriminator,
};
use pinocchio::pubkey::Pubkey;

pub mod account_compression_state;
mod check_accounts;
pub mod invoke_cpi;
pub mod processor;
// pub use invoke::instruction::*;
// pub use invoke_cpi::{initialize::*, instruction::*};
pub mod account_traits;
pub mod constants;
pub mod context;
pub mod errors;
pub mod invoke;
pub mod utils;

use errors::SystemProgramError;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
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
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, msg,
    program_error::ProgramError, syscalls::sol_log_compute_units_, ProgramResult,
};

use crate::{
    invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
};
use light_compressed_account::{
    constants::StateMerkleTreeAccount_DISCRIMINATOR,
    instruction_data::zero_copy::{
        ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZInstructionDataInvokeCpiWithReadOnly,
    },
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
        _ => panic!(""),
    }?;
    Ok(())
}

pub fn init_cpi_context_account(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<()> {
    // Check that Merkle tree is initialized.
    let (ctx, _accounts) =
        <InitializeCpiContextAccount<'_> as LightContext<'_>>::from_account_infos(accounts)?;
    let data = ctx.associated_merkle_tree.try_borrow_data()?;
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&data[0..8]);
    match discriminator_bytes {
        StateMerkleTreeAccount_DISCRIMINATOR => Ok(()),
        BatchedMerkleTreeAccount::DISCRIMINATOR => Ok(()),
        _ => Err(SystemProgramError::AppendStateFailed),
    }
    .map_err(ProgramError::from)?;
    let mut cpi_context_account = CpiContextAccount::default();
    cpi_context_account.init(*ctx.associated_merkle_tree.key());
    use borsh::BorshSerialize;
    cpi_context_account
        .serialize(&mut &mut ctx.cpi_context_account.try_borrow_mut_data()?[8..])
        .unwrap();
    ctx.cpi_context_account.try_borrow_mut_data()?[..8]
        .copy_from_slice(&CPI_CONTEXT_ACCOUNT_DISCRIMINATOR);
    Ok(())
}

pub trait LightContext<'info>: Sized {
    /// Attributes:
    /// - `#[signer]` - account must be a signer
    /// - `#[account(zero)]` - account must be empty
    /// - `#[account(Option<ProgramId>)]` - checks owner is this program
    /// - `#[unchecked_account]` - account is not checked
    /// - `#[pda_derivation(seeds, Option<ProgramId)- account is derived from seeds
    /// - `#[constraint = statement ]` - custom constraint
    /// - `#[compressed_account(Option<ProgramId>)]` - account is compressed owner is this program by default
    /// - '#[program]` - account is a program
    /// Macro rules for this function:
    /// 1. check that accounts len is sufficient
    ///     1.1. count number of fields marked with account attribute
    ///     1.2. throw if a field is not marked
    /// 2. create a variable for each account
    ///
    /// Notes:
    /// 1. replace instruction_data with optional T to keep instruction data deserialization separate
    fn from_account_infos(accounts: &'info [AccountInfo]) -> Result<(Self, &'info [AccountInfo])>;
}

pub struct InitializeCpiContextAccount<'info> {
    // #[signer]
    pub fee_payer: &'info AccountInfo,
    // #[account(zero)]
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

pub fn check_is_empty(account_info: &AccountInfo) -> Result<()> {
    if !account_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

impl<'info> LightContext<'info> for InitializeCpiContextAccount<'info> {
    fn from_account_infos(accounts: &'info [AccountInfo]) -> Result<(Self, &'info [AccountInfo])> {
        if accounts.len() < 3 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let fee_payer = &accounts[0];
        let cpi_context_account = &accounts[1];
        let associated_merkle_tree = &accounts[2];
        check_signer(&accounts[0]).map_err(ProgramError::from)?;

        // check_is_empty(cpi_context_account)?;

        Ok((
            Self {
                fee_payer,
                cpi_context_account,
                associated_merkle_tree,
            },
            &accounts[3..],
        ))
    }
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
    // msg!("Invoke instruction");
    let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(instruction_data).unwrap();
    let (ctx, remaining_accounts) =
        <InvokeInstruction<'_> as LightContext<'_>>::from_account_infos(accounts)?;
    sol_log_compute_units();
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("invoke_deserialize");
    input_compressed_accounts_signer_check(
        &inputs.input_compressed_accounts_with_merkle_context,
        &ctx.authority.key(),
    )?;
    // msg!(format!(
    //     "remaining_accounts {:?}",
    //     remaining_accounts
    //         .iter()
    //         .map(|x| x.key())
    //         .collect::<Vec<_>>()
    // )
    // .as_str());

    // msg!("Invoke instruction: post input_compressed_accounts_signer_check");
    process(inputs, None, ctx, 0, None, None, remaining_accounts)?;
    sol_log_compute_units();
    Ok(())
}

// pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
//     accounts: &[AccountInfo],
//     instruction_data: &[u8],
// ) -> Result<()> {
//     sol_log_compute_units();
//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_start!("cpda_deserialize");
//     let (inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(inputs.as_slice()).unwrap();
//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_end!("cpda_deserialize");

//     process_invoke_cpi(ctx, inputs, None, None)?;
//     sol_log_compute_units();
//     // 22,903 bytes heap with 33 outputs
//     #[cfg(feature = "bench-sbf")]
//     light_heap::bench_sbf_end!("total_usage");
//     Ok(())
// }

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
//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_start!("cpda_deserialize");
//     #[allow(unreachable_code)]
//     {
//         let (inputs, _) =
//             ZInstructionDataInvokeCpiWithReadOnly::zero_copy_at(inputs.as_slice()).unwrap();
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
//         process_invoke_cpi(
//             ctx,
//             inputs.invoke_cpi,
//             inputs.read_only_addresses,
//             inputs.read_only_accounts,
//         )
//     }
// }
