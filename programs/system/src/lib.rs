use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

pub mod invoke;
pub use invoke::instruction::*;
pub mod invoke_cpi;
pub use invoke_cpi::{initialize::*, instruction::*};
pub mod constants;
pub mod errors;
pub mod sdk;
pub mod utils;
declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

// // TODO(vadorovsky): Come up with some less glass chewy way of reusing
// // our light-heap allocator if it's already used in some dependency.
// #[cfg(all(feature = "custom-heap", target_os = "solana"))]
// pub use account_compression::GLOBAL_ALLOCATOR;

#[program]
pub mod light_system_program {

    use light_heap::{bench_sbf_end, bench_sbf_start};

    use self::{
        invoke::{processor::process, verify_signer::input_compressed_accounts_signer_check},
        invoke_cpi::processor::process_invoke_cpi,
    };

    use super::*;

    pub fn init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        // check that merkle tree is initialized
        let merkle_tree_account = ctx.accounts.associated_merkle_tree.load()?;
        merkle_tree_account.load_merkle_tree()?;
        ctx.accounts
            .cpi_context_account
            .init(ctx.accounts.associated_merkle_tree.key());
        msg!(
            "initialized cpi signature account pubkey {:?}",
            ctx.accounts.cpi_context_account.key()
        );
        Ok(())
    }

    pub fn invoke<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: InstructionDataInvoke =
            InstructionDataInvoke::deserialize(&mut inputs.as_slice())?;
        inputs.check_input_lengths()?;
        input_compressed_accounts_signer_check(&inputs, &ctx.accounts.authority.key())?;
        process(inputs, None, ctx)
    }

    pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        bench_sbf_start!("cpda_deserialize");
        let inputs: InstructionDataInvokeCpi =
            InstructionDataInvokeCpi::deserialize(&mut inputs.as_slice())?;
        bench_sbf_end!("cpda_deserialize");

        process_invoke_cpi(ctx, inputs)
    }

    // /// This function can be used to transfer sol and execute any other compressed transaction.
    // /// Instruction data is optimized for space.
    // pub fn execute_compressed_transaction2<'a, 'b, 'c: 'info, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, InvokeInstruction<'info>>,
    //     inputs: Vec<u8>,
    // ) -> Result<crate::event::PublicTransactionEvent> {
    //     let inputs: InstructionDataInvoke2 = InstructionDataInvoke2::try_deserialize_unchecked(
    //         &mut [vec![0u8; 8], inputs].concat().as_slice(),
    //     )?;
    //     let inputs = into_inputs(
    //         inputs,
    //         &ctx.accounts
    //             .to_account_infos()
    //             .iter()
    //             .map(|a| a.key())
    //             .collect::<Vec<Pubkey>>(),
    //         &ctx.remaining_accounts
    //             .iter()
    //             .map(|a| a.key())
    //             .collect::<Vec<Pubkey>>(),
    //     )?;
    //     process_compressed_transaction(&inputs, &ctx)
    // }

    // TODO: add compress and decompress sol as a wrapper around process_compressed_transaction

    // TODO: add create_pda as a wrapper around process_compressed_transaction
}
