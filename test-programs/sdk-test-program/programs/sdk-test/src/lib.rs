use anchor_lang::prelude::*;
use light_hasher::Discriminator;
use light_sdk::{
    account::LightAccount, address::derive_address, error::LightSdkError,
    instruction_data::LightInstructionData, light_account, light_system_accounts,
    program_merkle_context::unpack_address_merkle_context, verify::verify_light_accounts,
    LightHasher, LightTraits,
};

declare_id!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");

#[program]
pub mod sdk_test {
    use super::*;

    pub fn with_compressed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, WithCompressedAccount<'info>>,
        inputs: Vec<u8>,
        name: String,
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
            &[b"compressed", name.as_bytes()],
            &address_merkle_context,
            &crate::ID,
        );

        let mut my_compressed_account: LightAccount<'_, MyCompressedAccount> =
            LightAccount::from_meta_init(
                &accounts[0],
                MyCompressedAccount::discriminator(),
                address,
                address_seed,
                &crate::ID,
            )?;

        my_compressed_account.name = name;

        verify_light_accounts(
            &ctx,
            inputs.proof,
            &[my_compressed_account],
            None,
            false,
            None,
        )?;

        Ok(())
    }

    pub fn with_nested_data<'info>(
        ctx: Context<'_, '_, '_, 'info, WithNestedData<'info>>,
        inputs: Vec<u8>,
        name: String,
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
            &[b"compressed", name.as_bytes()],
            &address_merkle_context,
            &crate::ID,
        );

        let mut my_compressed_account: LightAccount<'_, MyCompressedAccount> =
            LightAccount::from_meta_init(
                &accounts[0],
                MyCompressedAccount::discriminator(),
                address,
                address_seed,
                &crate::ID,
            )?;

        my_compressed_account.name = name;
        my_compressed_account.nested = NestedData::default();

        verify_light_accounts(
            &ctx,
            inputs.proof,
            &[my_compressed_account],
            None,
            false,
            None,
        )?;

        Ok(())
    }

    pub fn update_nested_data<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        inputs: Vec<u8>,
        nested_data: NestedData,
    ) -> Result<()> {
        let inputs = LightInstructionData::deserialize(&inputs)?;
        let accounts = inputs
            .accounts
            .as_ref()
            .ok_or(LightSdkError::ExpectedAccounts)?;

        let mut my_compressed_account: LightAccount<'_, MyCompressedAccount> =
            LightAccount::from_meta_mut(
                &accounts[0],
                MyCompressedAccount::discriminator(),
                &crate::ID,
            )?;

        my_compressed_account.nested = nested_data;

        verify_light_accounts(
            &ctx,
            inputs.proof,
            &[my_compressed_account],
            None,
            false,
            None,
        )?;

        Ok(())
    }

    pub fn without_compressed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, WithoutCompressedAccount<'info>>,
        name: String,
    ) -> Result<()> {
        ctx.accounts.my_regular_account.name = name;
        Ok(())
    }
}

#[light_account]
#[derive(Clone, Debug, Default)]
pub struct MyCompressedAccount {
    name: String,
    #[nested]
    pub nested: NestedData,
}

// Illustrates nested hashing feature.
#[derive(LightHasher, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct NestedData {
    pub one: u16,
    pub two: u16,
    pub three: u16,
    pub four: u16,
    pub five: u16,
    pub six: u16,
    pub seven: u16,
    pub eight: u16,
    pub nine: u16,
    pub ten: u16,
    pub eleven: u16,
    pub twelve: u16,
}

impl Default for NestedData {
    fn default() -> Self {
        Self {
            one: 1,
            two: 2,
            three: 3,
            four: 4,
            five: 5,
            six: 6,
            seven: 7,
            eight: 8,
            nine: 9,
            ten: 10,
            eleven: 11,
            twelve: 12,
        }
    }
}

#[account]
pub struct MyRegularAccount {
    name: String,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
#[instruction(name: String)]
pub struct WithCompressedAccount<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::SdkTest>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    // #[light_account(
    //     init,
    //     seeds = [b"compressed".as_slice()],
    // )]
    // pub my_compressed_account: LightAccount<MyCompressedAccount>,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct WithNestedData<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::SdkTest>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    // #[light_account(
    //     init,
    //     seeds = [b"compressed".as_slice()],
    // )]
    // pub my_compressed_account: LightAccount<MyCompressedAccount>,
}

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct UpdateNestedData<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::SdkTest>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    // #[light_account(
    //     mut,
    //     seeds = [b"compressed".as_slice()],
    // )]
    // pub my_compressed_account: LightAccount<MyCompressedAccount>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct WithoutCompressedAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"compressed".as_slice(), name.as_bytes()],
        bump,
        payer = signer,
        space = 8 + 8,
    )]
    pub my_regular_account: Account<'info, MyRegularAccount>,
    pub system_program: Program<'info, System>,
}
