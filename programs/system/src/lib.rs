use invoke_cpi::account::CpiContextAccount;
// use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_account_checks::{
    checks::check_signer, discriminator::Discriminator as LightDiscriminator,
};

// mod check_accounts;
pub mod invoke_cpi;
// pub mod processor;
// pub use invoke::instruction::*;
// pub use invoke_cpi::{initialize::*, instruction::*};
// pub mod account_traits;
// pub mod constants;
// pub mod context;
pub mod errors;
// pub mod invoke;
// pub mod utils;

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
use anchor_lang::Discriminator;
use pinocchio::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, msg,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

pub type Result<T> = std::result::Result<T, ProgramError>;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if *program_id != ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    let (discriminator, instruction_data) = instruction_data.split_at(8);
    let discriminator = InstructionDiscriminator::try_from(discriminator).unwrap();
    match discriminator {
        InstructionDiscriminator::InitializeCpiContextAccount => {
            init_cpi_context_account(accounts, instruction_data)
        }
        _ => panic!(""),
    }?;
    Ok(())
}

// use account_compression::{errors::AccountCompressionErrorCode, StateMerkleTreeAccount};

// use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::instruction_data::zero_copy::{
    ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZInstructionDataInvokeCpiWithReadOnly,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::borsh::Deserialize;
// use self::invoke_cpi::processor::process_invoke_cpi;
// use crate::{
//     invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
// };

pub fn init_cpi_context_account(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<()> {
    // Check that Merkle tree is initialized.
    let (ctx, _accounts, _instruction_data) = <InitializeCpiContextAccount<'_> as LightContext<
        '_,
    >>::from_account_infos(
        accounts, instruction_data
    )?;
    let data = ctx.associated_merkle_tree.try_borrow_data()?;
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&data[0..8]);
    match discriminator_bytes {
        // StateMerkleTreeAccount::DISCRIMINATOR => Ok(()),
        BatchedMerkleTreeAccount::DISCRIMINATOR => Ok(()),
        _ => Err(SystemProgramError::AppendStateFailed),
    }
    .map_err(ProgramError::from)?;
    let mut cpi_context_account = CpiContextAccount::default();
    cpi_context_account.init(*ctx.associated_merkle_tree.key());
    use anchor_lang::prelude::borsh::BorshSerialize;
    cpi_context_account
        .serialize(&mut &mut ctx.cpi_context_account.try_borrow_mut_data()?[..])
        .unwrap();
    // ctx.accounts
    //     .cpi_context_account
    //     .init(ctx.accounts.associated_merkle_tree.key());
    Ok(())
}

pub trait LightContext<'info>: Sized {
    fn from_account_infos(
        accounts: &'info [AccountInfo],
        instruction_data: &'info [u8],
    ) -> Result<(Self, &'info [AccountInfo], &'info [u8])>;
}

pub struct InitializeCpiContextAccount<'info> {
    // #[signer]
    pub fee_payer: &'info AccountInfo,
    // TODO: figure out check
    // #[account(zero)]
    pub cpi_context_account: &'info AccountInfo,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: &'info AccountInfo,
}

impl<'info> LightContext<'info> for InitializeCpiContextAccount<'info> {
    fn from_account_infos(
        accounts: &'info [AccountInfo],
        instruction_data: &'info [u8],
    ) -> Result<(Self, &'info [AccountInfo], &'info [u8])> {
        check_signer(&accounts[0]).map_err(ProgramError::from)?;

        if accounts.len() < 3 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        Ok((
            Self {
                fee_payer: &accounts[0],
                cpi_context_account: &accounts[1],
                associated_merkle_tree: &accounts[2],
            },
            &accounts[3..],
            instruction_data,
        ))
    }
}

// pub fn invoke<'a, 'b, 'c: 'info, 'info>(
//     accounts: &[AccountInfo],
//     instruction_data: &[u8],
// ) -> Result<()> {
//     sol_log_compute_units();

//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_start!("invoke_deserialize");
//     msg!("Invoke instruction");
//     let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(inputs.as_slice()).unwrap();
//     sol_log_compute_units();
//     #[cfg(feature = "bench-sbf")]
//     bench_sbf_end!("invoke_deserialize");
//     input_compressed_accounts_signer_check(
//         &inputs.input_compressed_accounts_with_merkle_context,
//         &ctx.accounts.authority.key(),
//     )?;
//     process(inputs, None, ctx, 0, None, None)?;
//     sol_log_compute_units();
//     Ok(())
// }

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
