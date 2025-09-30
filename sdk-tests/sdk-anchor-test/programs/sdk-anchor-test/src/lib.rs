#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::{prelude::*, Discriminator};
use light_sdk::{
    // anchor test test poseidon LightAccount, native tests sha256 LightAccount
    account::LightAccount,
    address::v1::derive_address,
    cpi::{
        v1::CpiAccounts, v2::InstructionDataInvokeCpiWithReadOnly, CpiSigner,
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    derive_light_cpi_signer,
    instruction::{
        account_meta::{CompressedAccountMeta, CompressedAccountMetaBurn},
        PackedAddressTreeInfo, ValidityProof,
    },
    LightDiscriminator,
    LightHasher,
};

declare_id!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");

#[program]
pub mod sdk_anchor_test {

    use super::*;

    pub fn create_compressed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, WithNestedData<'info>>,
        proof: ValidityProof,
        address_tree_info: PackedAddressTreeInfo,
        output_tree_index: u8,
        name: String,
    ) -> Result<()> {
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        let (address, address_seed) = derive_address(
            &[b"compressed", name.as_bytes()],
            &address_tree_info
                .get_tree_pubkey(&light_cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
            &crate::ID,
        );
        let new_address_params =
            address_tree_info.into_new_address_params_assigned_packed(address_seed, Some(0));

        let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_init(
            &crate::ID,
            Some(address),
            output_tree_index,
        );

        my_compressed_account.name = name;
        my_compressed_account.nested = NestedData::default();

        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .with_new_addresses(&[new_address_params])
            .invoke(light_cpi_accounts)?;

        Ok(())
    }

    pub fn update_compressed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        proof: ValidityProof,
        my_compressed_account: MyCompressedAccount,
        account_meta: CompressedAccountMeta,
        nested_data: NestedData,
    ) -> Result<()> {
        let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_mut(
            &crate::ID,
            &account_meta,
            my_compressed_account,
        )?;

        my_compressed_account.nested = nested_data;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );
        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke(light_cpi_accounts)?;

        Ok(())
    }

    pub fn close_compressed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        proof: ValidityProof,
        my_compressed_account: MyCompressedAccount,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_close(
            &crate::ID,
            &account_meta,
            my_compressed_account,
        )?;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke(light_cpi_accounts)?;

        Ok(())
    }

    pub fn reinit_closed_account<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        proof: ValidityProof,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let my_compressed_account =
            LightAccount::<'_, MyCompressedAccount>::new_empty(&crate::ID, &account_meta)?;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke(light_cpi_accounts)?;

        Ok(())
    }

    pub fn close_compressed_account_permanent<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
        proof: ValidityProof,
        account_meta: CompressedAccountMetaBurn,
    ) -> Result<()> {
        let my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_burn(
            &crate::ID,
            &account_meta,
            MyCompressedAccount::default(),
        )?;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );
        InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke(light_cpi_accounts)?;

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

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct MyCompressedAccount {
    #[hash]
    pub name: String,
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

#[derive(Accounts)]
#[instruction(name: String)]
pub struct WithCompressedAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithNestedData<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateNestedData<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
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
