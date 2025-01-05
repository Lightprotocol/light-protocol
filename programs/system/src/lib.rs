use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_hasher::Discriminator as LightDiscriminator;

pub mod invoke;
pub use invoke::instruction::*;
pub mod invoke_cpi;
pub use invoke_cpi::{initialize::*, instruction::*};
pub mod constants;
pub mod errors;
pub mod sdk;
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
    use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeMetadata;
    use light_heap::{bench_sbf_end, bench_sbf_start};

    use self::{
        invoke::{processor::process, verify_signer::input_compressed_accounts_signer_check},
        invoke_cpi::processor::process_invoke_cpi,
    };
    use super::*;

    pub fn init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        // Check that Merkle tree is initialized.
        let data = ctx.accounts.associated_merkle_tree.data.borrow();

        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&data[0..8]);
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => Ok(()),
            BatchedMerkleTreeMetadata::DISCRIMINATOR => Ok(()),
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
        let inputs: InstructionDataInvoke =
            InstructionDataInvoke::deserialize(&mut inputs.as_slice())?;

        input_compressed_accounts_signer_check(
            &inputs.input_compressed_accounts_with_merkle_context,
            &ctx.accounts.authority.key(),
        )?;
        process(inputs, None, ctx, 0, None, None)
    }

    pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        bench_sbf_start!("cpda_deserialize");
        let inputs: InstructionDataInvokeCpi =
            InstructionDataInvokeCpi::deserialize(&mut inputs.as_slice())?;
        bench_sbf_end!("cpda_deserialize");

        process_invoke_cpi(ctx, inputs, None, None)
    }

    pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        bench_sbf_start!("cpda_deserialize");
        let inputs = InstructionDataInvokeCpiWithReadOnly::deserialize(&mut inputs.as_slice())?;
        bench_sbf_end!("cpda_deserialize");
        // disable set cpi context because cpi context account uses InvokeCpiInstruction
        if let Some(cpi_context) = inputs.invoke_cpi.cpi_context {
            if cpi_context.set_context {
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
