#![allow(unexpected_cfgs)]

use anchor_lang::{prelude::*, solana_program::program::invoke, Discriminator};
use light_compressed_token_sdk::cpi::{
    create_compressed_token_instruction, CpiAccounts, CpiInputs,
};

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[program]
pub mod sdk_token_test {

    use super::*;

    pub fn compress<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient: Pubkey,
        mint: Pubkey, // TODO: deserialize from token account.
        amount: u64,
    ) -> Result<()> {
        let light_cpi_accounts =
            CpiAccounts::new(ctx.accounts.signer.as_ref(), ctx.remaining_accounts);

        let mut token_account = light_compressed_token_sdk::account::CTokenAccount::new_empty(
            mint,
            recipient,
            output_tree_index,
        );
        token_account.compress(amount).unwrap();

        let cpi_inputs = CpiInputs::new_compress(vec![token_account]);

        // TODO: add to program error conversion
        let instruction =
            create_compressed_token_instruction(cpi_inputs, &light_cpi_accounts).unwrap();
        let account_infos = light_cpi_accounts.to_account_infos();
        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    // pub fn transfer<'info>(
    //     ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
    //     proof: ValidityProof,
    //     my_compressed_account: MyCompressedAccount,
    //     account_meta: CompressedAccountMeta,
    //     nested_data: NestedData,
    // ) -> Result<()> {
    //     let mut my_compressed_account = LightAccount::<'_, MyCompressedAccount>::new_mut(
    //         &crate::ID,
    //         &account_meta,
    //         my_compressed_account,
    //     )
    //     .map_err(ProgramError::from)?;

    //     my_compressed_account.nested = nested_data;

    //     let light_cpi_accounts = CpiAccounts::new(
    //         ctx.accounts.signer.as_ref(),
    //         ctx.remaining_accounts,
    //         crate::LIGHT_CPI_SIGNER,
    //     );

    //     let cpi_inputs = CpiInputs::new(
    //         proof,
    //         vec![my_compressed_account
    //             .to_account_info()
    //             .map_err(ProgramError::from)?],
    //     );

    //     cpi_inputs
    //         .invoke_light_system_program(light_cpi_accounts)
    //         .map_err(ProgramError::from)?;

    //     Ok(())
    // }

    // pub fn decompress<'info>(
    //     ctx: Context<'_, '_, '_, 'info, WithoutCompressedAccount<'info>>,
    //     name: String,
    // ) -> Result<()> {
    //     ctx.accounts.my_regular_account.name = name;
    //     Ok(())
    // }
}

#[derive(Accounts)]
pub struct Generic<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}
