use anchor_lang::prelude::*;
use light_sdk::{
    account::LightAccount,
    cpi::verify::verify_compressed_account_infos,
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, instruction_data::LightInstructionData},
    Discriminator, LightDiscriminator, LightHasher,
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_program {
    use light_sdk::{
        address::v1::derive_address,
        cpi::accounts::CompressionCpiAccounts,
        NewAddressParamsPacked,
    };

    use super::*;

    pub fn create<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        light_ix_data: LightInstructionData,
        output_merkle_tree_index: u8,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        let address_merkle_context = light_ix_data
            .new_addresses
            .ok_or(LightSdkError::ExpectedAddressMerkleContext)
            .map_err(ProgramError::from)?[0];

        let (address, address_seed) = derive_address(
            &[b"counter", ctx.accounts.signer.key().as_ref()],
            &light_cpi_accounts.tree_accounts()[address_merkle_context.address_merkle_tree_pubkey_index as
usize].key(),
            &crate::ID,
        );

        let new_address_params = NewAddressParamsPacked {
            seed: address_seed,
            address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
            address_merkle_tree_root_index: address_merkle_context.root_index,
            address_merkle_tree_account_index: address_merkle_context.address_merkle_tree_pubkey_index,
        };

        let mut counter = LightAccount::<'_, CounterCompressedAccount>::new_init(
            &program_id,
            Some(address),
            output_merkle_tree_index,
        );

        counter.owner = ctx.accounts.signer.key();

        verify_compressed_account_infos(
            &light_cpi_accounts,
            light_ix_data.proof,
            &[counter.to_account_info().unwrap()],
            Some(vec![new_address_params]),
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn increment<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        light_ix_data: LightInstructionData,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();
        let mut counter = LightAccount::<'_, CounterCompressedAccount>::new_mut(
            &program_id,
            &account_meta,
            CounterCompressedAccount {
                owner: ctx.accounts.signer.key(),
                counter: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        counter.counter += 1;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        verify_compressed_account_infos(
            &light_cpi_accounts,
            light_ix_data.proof,
            &[counter.to_account_info().unwrap()],
            None,
            None,
            false,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }

    pub fn delete<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericAnchorAccounts<'info>>,
        light_ix_data: LightInstructionData,
        counter_value: u64,
        account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let program_id = crate::ID.into();

        let counter = LightAccount::<'_, CounterCompressedAccount>::new_close(
            &program_id,
            &account_meta,
            CounterCompressedAccount {
                owner: ctx.accounts.signer.key(),
                counter: counter_value,
            },
        )
        .map_err(ProgramError::from)?;

        let light_cpi_accounts = CompressionCpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
            crate::ID,
        )
        .map_err(ProgramError::from)?;

        // The true parameter indicates that accounts should be closed
        verify_compressed_account_infos(
            &light_cpi_accounts,
            light_ix_data.proof,
            &[counter.to_account_info().unwrap()],
            None,
            None,
            true,
            None,
        )
        .map_err(ProgramError::from)?;

        Ok(())
    }
}

#[derive(
    Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, LightDiscriminator, LightHasher,
)]
pub struct CounterCompressedAccount {
    #[hash]
    pub owner: Pubkey,
    pub counter: u64,
}

#[derive(Accounts)]
pub struct GenericAnchorAccounts<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}
