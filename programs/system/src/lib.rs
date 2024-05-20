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

// TODO(vadorovsky): Come up with some less glass chewy way of reusing our
// light-heap allocator if it's already used in some dependency.
// #[cfg(all(feature = "custom-heap", target_os = "solana"))] pub use
// account_compression::GLOBAL_ALLOCATOR;

#[program]
pub mod light_system_program {

    use light_heap::{bench_sbf_end, bench_sbf_start};

    use self::{
        invoke::{processor::process, verify_signer::input_compressed_accounts_signer_check},
        invoke_cpi::processor::process_invoke_cpi,
    };

    use super::*;

    // TODO: test init from registry program method
    pub fn init_cpi_context_account(
        ctx: Context<InitializeCpiContextAccount>,
        network_fee: u64,
    ) -> Result<()> {
        // check that merkle tree is initialized and signer is eligible
        let merkle_tree_account = ctx.accounts.associated_merkle_tree.load()?;
        merkle_tree_account.load_merkle_tree()?;
        if merkle_tree_account.owner != ctx.accounts.fee_payer.key() {
            return Err(crate::errors::CompressedPdaError::InvalidMerkleTreeOwner.into());
        }
        ctx.accounts
            .cpi_context_account
            .init(ctx.accounts.associated_merkle_tree.key(), network_fee);
        Ok(())
    }

    pub fn claim_from_cpi_context_account(ctx: Context<ClaimCpiContextAccount>) -> Result<()> {
        // check that merkle tree is initialized and signer is eligible
        let merkle_tree_account = ctx.accounts.associated_merkle_tree.load()?;
        merkle_tree_account.load_merkle_tree()?;
        if merkle_tree_account.owner != ctx.accounts.fee_payer.key() {
            return Err(crate::errors::CompressedPdaError::InvalidMerkleTreeOwner.into());
        }
        let sender_account_info = &mut ctx.accounts.cpi_context_account.to_account_info();
        let rent = Rent::get()?;
        let rent_exempt_reserve = rent.minimum_balance(sender_account_info.data.borrow().len());
        let lamports = sender_account_info.lamports();
        let claim_amount = lamports.checked_sub(rent_exempt_reserve).unwrap();
        ctx.accounts
            .cpi_context_account
            .sub_lamports(claim_amount)
            .map_err(|_| ProgramError::InsufficientFunds)?;
        ctx.accounts
            .recipient
            .add_lamports(claim_amount)
            .map_err(|_| ProgramError::InsufficientFunds)?;
        Ok(())
    }

    pub fn invoke<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: InstructionDataInvoke =
            InstructionDataInvoke::deserialize(&mut inputs.as_slice())?;

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

    // TODO:
    // - add compress and decompress sol as a wrapper around
    // process_compressed_transaction
    // - add create_pda as a wrapper around process_compressed_transaction
}
