#![allow(unexpected_cfgs)]

use anchor_lang::{prelude::*, Discriminator};
use light_sdk::{
    account::LightAccount,
    address::v1::derive_address,
    cpi::{CpiAccounts, CpiInputs},
    instruction::{account_meta::CompressedAccountMeta, tree_info::PackedAddressTreeInfo},
    LightDiscriminator, LightHasher, NewAddressParamsPacked, ValidityProof,
};

declare_id!("2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt");

#[program]
pub mod sdk_anchor_test {

    use super::*;

    pub fn with_nested_data<'info>(
        ctx: Context<'_, '_, '_, 'info, WithNestedData<'info>>,
        proof: ValidityProof,
        address_tree_info: PackedAddressTreeInfo,
        output_tree_index: u8,
        name: String,
    ) -> Result<()> {
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let (address, address_seed) = derive_address(
            &[b"compressed", name.as_bytes()],
            &light_cpi_accounts.tree_accounts()
                [address_tree_info.address_merkle_tree_pubkey_index as usize]
                .key(),
            &crate::ID,
        );
        let new_address_params = NewAddressParamsPacked {
            seed: address_seed,
            address_queue_account_index: address_tree_info.address_queue_pubkey_index,
            address_merkle_tree_root_index: address_tree_info.root_index,
            address_merkle_tree_account_index: address_tree_info.address_merkle_tree_pubkey_index,
        };

        let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_init(
            &crate::ID,
            Some(address),
            output_tree_index,
        );

        my_compressed_account.name = name;
        my_compressed_account.nested = NestedData::default();

        let cpi_inputs = CpiInputs::new_with_address(
            proof,
            vec![my_compressed_account
                .to_account_info()
                .map_err(ProgramError::from)?],
            vec![new_address_params],
        );

        cpi_inputs
            .invoke_light_system_program(light_cpi_accounts)
            .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn update_nested_data<'info>(
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
        )
        .map_err(ProgramError::from)?;

        my_compressed_account.nested = nested_data;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let cpi_inputs = CpiInputs::new(
            proof,
            vec![my_compressed_account
                .to_account_info()
                .map_err(ProgramError::from)?],
        );

        cpi_inputs
            .invoke_light_system_program(light_cpi_accounts)
            .map_err(ProgramError::from)?;

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
