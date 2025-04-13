use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use light_sdk::{
    account::LightAccount, instruction_data::LightInstructionData, light_system_accounts,
    verify::verify_light_accounts, LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod counter {
    use light_sdk::{
        address::derive_address, error::LightSdkError,
        program_merkle_context::unpack_address_merkle_context,
        system_accounts::CompressionCpiAccounts, Discriminator,
    };

    use super::*;

    pub fn create_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCounter<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let address_merkle_context = accounts[0]
            .address_merkle_context
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)?;
        let address_merkle_context =
            unpack_address_merkle_context(address_merkle_context, ctx.remaining_accounts);
        let (address, address_seed) = derive_address(
            &[b"counter", ctx.accounts.signer.key().as_ref()],
            &address_merkle_context,
            &crate::ID,
        );

        let mut counter: LightAccount<'_, CounterAccount> = LightAccount::from_meta_init(
            &accounts[0],
            CounterAccount::discriminator(),
            address,
            address_seed,
            &crate::ID,
        )?;

        counter.owner = ctx.accounts.signer.key();
        counter.value = 0;
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[counter],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn increment_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateCounter<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;
        let mut counter: LightAccount<'_, CounterAccount> =
            LightAccount::from_meta_mut(&accounts[0], CounterAccount::discriminator(), &crate::ID)?;

        if counter.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }
        counter.value = counter.value.checked_add(1).ok_or(CustomError::Overflow)?;
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[counter],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;
        Ok(())
    }
    pub fn decrement_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateCounter<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let mut counter: LightAccount<'_, CounterAccount> =
            LightAccount::from_meta_mut(&accounts[0], CounterAccount::discriminator(), &crate::ID)?;

        if counter.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        counter.value = counter.value.checked_sub(1).ok_or(CustomError::Underflow)?;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[counter],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;
        Ok(())
    }

    pub fn reset_counter<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateCounter<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let mut counter: LightAccount<'_, CounterAccount> =
            LightAccount::from_meta_mut(&accounts[0], CounterAccount::discriminator(), &crate::ID)?;

        if counter.owner != ctx.accounts.signer.key() {
            return err!(CustomError::Unauthorized);
        }

        counter.value = 0;
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.cpi_signer.as_ref(),
            ctx.remaining_accounts,
        );
        verify_light_accounts(
            &light_cpi_accounts,
            inputs.proof,
            &[counter],
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;
        Ok(())
    }
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct CounterAccount {
    #[hash]
    pub owner: Pubkey,
    pub value: u64,
}

#[error_code]
pub enum CustomError {
    #[msg("No authority to perform this action")]
    Unauthorized,
    #[msg("Counter overflow")]
    Overflow,
    #[msg("Counter underflow")]
    Underflow,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct CreateCounter<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Counter>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct UpdateCounter<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Counter>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}
