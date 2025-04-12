use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use light_sdk::{
    account::LightAccount, instruction_data::LightInstructionData, light_system_accounts,
    system_accounts::CompressionCpiAccounts, verify::verify_light_accounts, LightDiscriminator,
    LightHasher, LightTraits,
};

use solana_program::program_error::ProgramError;

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod memo {
    use light_hasher::Discriminator;
    use light_sdk::{
        address::derive_address, error::LightSdkError,
        program_merkle_context::unpack_address_merkle_context,
    };

    use super::*;

    pub fn create_memo<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMemo<'info>>,
        inputs: Vec<u8>,
        message: String,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let address_merkle_context = accounts[0]
            .address_merkle_context
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)
            .map_err(ProgramError::from)?;
        let address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let (address, address_seed) = derive_address(
            &[b"memo", ctx.accounts.signer.key().as_ref()],
            &address_merkle_context.address_merkle_tree_pubkey,
            &crate::ID,
        );

        let mut memo: LightAccount<'_, MemoAccount> = LightAccount::from_meta_init(
            &accounts[0],
            MemoAccount::discriminator(),
            address,
            address_seed,
            &crate::ID,
        )
        .map_err(ProgramError::from)?;

        memo.authority = ctx.accounts.signer.key();
        memo.message = message;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );

        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[memo],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn update_memo<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateMemo<'info>>,
        inputs: Vec<u8>,
        new_message: String,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)
            .map_err(ProgramError::from)?;

        let mut memo: LightAccount<'_, MemoAccount> =
            LightAccount::from_meta_mut(&accounts[0], MemoAccount::discriminator(), &crate::ID)
                .map_err(ProgramError::from)?;

        if memo.authority != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        memo.message = new_message;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );

        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[memo],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn delete_memo<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteMemo<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)
            .map_err(ProgramError::from)?;

        let memo: LightAccount<'_, MemoAccount> =
            LightAccount::from_meta_close(&accounts[0], MemoAccount::discriminator(), &crate::ID)
                .map_err(ProgramError::from)?;

        if memo.authority != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );

        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[memo],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }
}

// Memo account structure
#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct MemoAccount {
    #[hash]
    pub authority: Pubkey,
    pub message: String,
}

// Custom errors
#[error_code]
pub enum CustomError {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
}

// Context for creating a memo
#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct CreateMemo<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Memo>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

// Context for updating a memo
#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct UpdateMemo<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Memo>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

// Context for deleting a memo
#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct DeleteMemo<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Memo>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}
