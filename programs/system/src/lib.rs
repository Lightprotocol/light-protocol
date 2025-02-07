use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_account_checks::discriminator::Discriminator as LightDiscriminator;

mod check_accounts;
pub mod invoke_cpi;
pub mod processor;
pub use invoke::instruction::*;
pub use invoke_cpi::{initialize::*, instruction::*};
pub mod account_traits;
pub mod constants;
pub mod context;
pub mod errors;
pub mod invoke;
pub mod utils;

use errors::SystemProgramError;

declare_id!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_system_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}
use anchor_lang::Discriminator;

#[program]
pub mod light_system_program {

    use account_compression::{errors::AccountCompressionErrorCode, StateMerkleTreeAccount};
    use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
    use light_compressed_account::instruction_data::zero_copy::{
        ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZInstructionDataInvokeCpiWithReadOnly,
    };
    #[cfg(feature = "bench-sbf")]
    use light_heap::{bench_sbf_end, bench_sbf_start};
    use light_zero_copy::borsh::Deserialize;

    use self::invoke_cpi::processor::process_invoke_cpi;
    use super::*;
    use crate::{
        invoke::verify_signer::input_compressed_accounts_signer_check, processor::process::process,
    };

    pub fn init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        // Check that Merkle tree is initialized.
        let data = ctx.accounts.associated_merkle_tree.data.borrow();

        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&data[0..8]);
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => Ok(()),
            BatchedMerkleTreeAccount::DISCRIMINATOR => Ok(()),
            _ => {
                err!(AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch)
            }
        }?;
        ctx.accounts
            .cpi_context_account
            .init(ctx.accounts.associated_merkle_tree.key());
        Ok(())
    }

    pub fn invoke<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        #[cfg(feature = "bench-sbf")]
        bench_sbf_start!("invoke_deserialize");

        let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(inputs.as_slice()).unwrap();

        #[cfg(feature = "bench-sbf")]
        bench_sbf_end!("invoke_deserialize");
        input_compressed_accounts_signer_check(
            &inputs.input_compressed_accounts_with_merkle_context,
            &ctx.accounts.authority.key(),
        )?;
        process(inputs, None, ctx, 0, None, None)?;

        Ok(())
    }

    pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        #[cfg(feature = "bench-sbf")]
        bench_sbf_start!("cpda_deserialize");
        let (inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(inputs.as_slice()).unwrap();
        #[cfg(feature = "bench-sbf")]
        bench_sbf_end!("cpda_deserialize");

        process_invoke_cpi(ctx, inputs, None, None)?;

        // 22,903 bytes heap with 33 outputs
        #[cfg(feature = "bench-sbf")]
        light_heap::bench_sbf_end!("total_usage");
        Ok(())
    }

    #[allow(unused_variables)]
    pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        #[cfg(not(feature = "readonly"))]
        {
            msg!("Readonly feature is not enabled.");

            return Err(SystemProgramError::InstructionNotCallable.into());
        }
        #[cfg(feature = "bench-sbf")]
        bench_sbf_start!("cpda_deserialize");
        #[allow(unreachable_code)]
        {
            let (inputs, _) =
                ZInstructionDataInvokeCpiWithReadOnly::zero_copy_at(inputs.as_slice()).unwrap();
            #[cfg(feature = "bench-sbf")]
            bench_sbf_end!("cpda_deserialize");
            // disable set cpi context because cpi context account uses InvokeCpiInstruction
            if let Some(cpi_context) = inputs.invoke_cpi.cpi_context {
                if cpi_context.set_context() {
                    msg!("Cannot set cpi context in invoke_cpi_with_read_only.");
                    msg!("Please use invoke_cpi instead.");
                    return Err(SystemProgramError::InstructionNotCallable.into());
                }
            }
            process_invoke_cpi(
                ctx,
                inputs.invoke_cpi,
                inputs.read_only_addresses,
                inputs.read_only_accounts,
            )
        }
    }

    // /// This function is a stub to allow Anchor to include the input types in
    // /// the IDL. It should not be included in production builds nor be called in
    // /// practice.
    // #[cfg(feature = "idl-build")]
    // pub fn stub_idl_build<'info>(
    //     _ctx: Context<'_, '_, '_, 'info, InvokeInstruction<'info>>,
    //     _inputs1: InstructionDataInvoke,
    //     _inputs2: InstructionDataInvokeCpi,
    //     _inputs3: PublicTransactionEvent,
    // ) -> Result<()> {
    //     Err(SystemProgramError::InstructionNotCallable.into())
    // }
}
