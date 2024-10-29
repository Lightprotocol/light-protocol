use anchor_lang::prelude::*;

use light_sdk::{
    account_info::{convert_metas_to_infos, LightAccountInfo},
    compressed_account::{LightAccount, LightAccounts},
    instruction_data::LightInstructionData,
    light_system_accounts,
    verify::verify_compressed_accounts,
    LightDiscriminator, LightHasher, LightTraits,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod subscription {
    use super::*;

    pub fn start_trial(
        ctx: Context<StartTrial>,
        inputs: Vec<u8>,
        trial_duration: i64,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let account_infos = convert_metas_to_infos(&inputs.accounts, &crate::ID)?;
        let mut light_accounts = LightStartTrial::try_light_accounts(&account_infos)?;

        let clock = Clock::get()?;

        light_accounts.trial.user = *ctx.accounts.signer.key;
        light_accounts.trial.start_date = clock.unix_timestamp;
        light_accounts.trial.expiration_date = clock.unix_timestamp + trial_duration;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.trial],
            None,
            false,
            None,
            &crate::ID,
        )?;

        Ok(())
    }

    pub fn convert_to_subscription(
        ctx: Context<ConvertToSubscription>,
        inputs: Vec<u8>,
        subscription_duration: i64,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let account_infos = convert_metas_to_infos(&inputs.accounts, &crate::ID)?;
        let mut light_accounts = LightConvertToSubscription::try_light_accounts(&account_infos)?;

        let clock = Clock::get()?;

        light_accounts.subscription.user = light_accounts.trial.user;
        light_accounts.subscription.start_date = clock.unix_timestamp;
        light_accounts.subscription.renewal_date = clock.unix_timestamp + subscription_duration;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.trial, light_accounts.subscription],
            None,
            false,
            None,
            &crate::ID,
        )?;

        Ok(())
    }

    pub fn cancel_subscription(ctx: Context<CancelSubscription>, inputs: Vec<u8>) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let account_infos = convert_metas_to_infos(&inputs.accounts, &crate::ID)?;
        let mut light_accounts = LightCancelSubscription::try_light_accounts(&account_infos)?;

        verify_compressed_accounts(
            &ctx,
            inputs.proof,
            &[light_accounts.subscription],
            None,
            false,
            None,
            &crate::ID,
        )?;

        Ok(())
    }
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct Trial {
    pub user: Pubkey,
    pub start_date: i64,
    pub expiration_date: i64,
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct Subscription {
    pub user: Pubkey,
    pub start_date: i64,
    pub renewal_date: i64,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct StartTrial<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Subscription>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightStartTrial<'info> {
    pub trial: LightAccount<'info, Trial>,
}

impl<'info> LightAccounts<'info> for LightStartTrial<'info> {
    fn try_light_accounts(accounts: &'info [LightAccountInfo]) -> Result<Self> {
        let trial: LightAccount<Trial> = LightAccount::new(&accounts[0])?;
        Ok(Self { trial })
    }
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct ConvertToSubscription<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Subscription>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightConvertToSubscription<'info> {
    pub trial: LightAccount<'info, Trial>,
    pub subscription: LightAccount<'info, Subscription>,
}

impl<'info> LightAccounts<'info> for LightConvertToSubscription<'info> {
    fn try_light_accounts(accounts: &'info [LightAccountInfo]) -> Result<Self> {
        let trial: LightAccount<Trial> = LightAccount::new(&accounts[0])?;
        let subscription: LightAccount<Subscription> = LightAccount::new(&accounts[1])?;
        Ok(Self {
            trial,
            subscription,
        })
    }
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct CancelSubscription<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::Subscription>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
}

pub struct LightCancelSubscription<'info> {
    pub subscription: LightAccount<'info, Subscription>,
}

impl<'info> LightAccounts<'info> for LightCancelSubscription<'info> {
    fn try_light_accounts(accounts: &'info [LightAccountInfo]) -> Result<Self> {
        let subscription: LightAccount<Subscription> = LightAccount::new(&accounts[0])?;
        Ok(Self { subscription })
    }
}
