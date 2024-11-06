use anchor_lang::prelude::*;
use light_sdk::{
    compressed_account::LightAccount, light_account, light_accounts, light_program,
    merkle_context::PackedAddressMerkleContext, LightHasher,
};

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[light_program]
#[program]
pub mod sdk_test {
    use super::*;

    pub fn with_compressed_account<'info>(
        ctx: LightContext<'_, '_, '_, 'info, WithCompressedAccount<'info>>,
        name: String,
    ) -> Result<()> {
        ctx.light_accounts.my_compressed_account.name = name;
        Ok(())
    }

    pub fn with_nested_data<'info>(
        ctx: LightContext<'_, '_, '_, 'info, WithNestedData<'info>>,
        one: u16,
    ) -> Result<()> {
        ctx.light_accounts.my_compressed_account.nested = NestedData::default();
        ctx.light_accounts.my_compressed_account.nested.one = one;
        Ok(())
    }

    pub fn update_nested_data<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        nested_data: NestedData,
    ) -> Result<()> {
        ctx.light_accounts.my_compressed_account.nested = nested_data;
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

#[light_accounts]
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

    #[light_account(
        init,
        seeds = [b"compressed".as_slice(), name.as_bytes()],
    )]
    pub my_compressed_account: LightAccount<MyCompressedAccount>,
}

#[light_accounts]
#[instruction(one: u16)]
pub struct WithNestedData<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::SdkTest>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        init,
        seeds = [b"compressed".as_slice()],
    )]
    pub my_compressed_account: LightAccount<MyCompressedAccount>,
}

#[light_accounts]
pub struct UpdateNestedData<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::SdkTest>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        mut,
        seeds = [b"compressed".as_slice()],
    )]
    pub my_compressed_account: LightAccount<MyCompressedAccount>,
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
