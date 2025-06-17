#![allow(unexpected_cfgs)]

use anchor_lang::{prelude::*, solana_program::program::invoke, Discriminator};
use light_compressed_token_sdk::{
    cpi::{create_compressed_token_instruction, CpiAccounts, CpiInputs},
    InputTokenDataWithContext, ValidityProof,
};

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[program]
pub mod sdk_token_test {

    use super::*;

    pub fn compress<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient: Pubkey, // TODO: make recpient pda
        mint: Pubkey,      // TODO: deserialize from token account.
        amount: u64,
    ) -> Result<()> {
        let mut token_account = light_compressed_token_sdk::account::CTokenAccount::new_empty(
            mint,
            recipient,
            output_tree_index,
        );
        token_account.compress(amount).unwrap();

        let cpi_inputs = CpiInputs::new_compress(vec![token_account]);

        let light_cpi_accounts = CpiAccounts::new_compress(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );
        msg!(
            "light_cpi_accounts config {:?}",
            light_cpi_accounts.config()
        );
        msg!(
            "light_cpi_accounts config is_compress_or_decompress {:?}",
            light_cpi_accounts.config().is_compress_or_decompress()
        );
        msg!(
            "ctx.remaining_accounts len {:?}",
            ctx.remaining_accounts.len()
        );
        // TODO: add to program error conversion
        let instruction =
            create_compressed_token_instruction(cpi_inputs, &light_cpi_accounts).unwrap();
        let account_infos = light_cpi_accounts.to_account_infos();
        msg!("account_infos {:?}", account_infos);
        msg!("instruction {:?}", instruction);
        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_data: Vec<InputTokenDataWithContext>,
        output_tree_index: u8,
        mint: Pubkey,
        recipient: Pubkey,
    ) -> Result<()> {
        let mut token_account = light_compressed_token_sdk::account::CTokenAccount::new(
            mint,
            ctx.accounts.signer.key(), // TODO: reconsider whether this makes sense
            token_data,
            output_tree_index,
        );
        // None is the same output_tree_index as token account
        let recipient_token_account = token_account.transfer(&recipient, 10, None).unwrap();

        let cpi_inputs =
            CpiInputs::new(vec![token_account, recipient_token_account], validity_proof);
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );

        // TODO: add to program error conversion
        let instruction =
            create_compressed_token_instruction(cpi_inputs, &light_cpi_accounts).unwrap();
        let account_infos = light_cpi_accounts.to_account_infos();

        // TODO: make invoke_signed
        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    pub fn decompress<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_data: Vec<InputTokenDataWithContext>,
        output_tree_index: u8,
        mint: Pubkey,
    ) -> Result<()> {
        let mut token_account = light_compressed_token_sdk::account::CTokenAccount::new(
            mint,
            ctx.accounts.signer.key(), // TODO: reconsider whether this makes sense
            token_data,
            output_tree_index,
        );
        token_account.decompress(10).unwrap();

        let cpi_inputs = CpiInputs::new(vec![token_account], validity_proof);
        let light_cpi_accounts = CpiAccounts::new_decompress(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );

        // TODO: add to program error conversion
        let instruction =
            create_compressed_token_instruction(cpi_inputs, &light_cpi_accounts).unwrap();
        let account_infos = light_cpi_accounts.to_account_infos();
        msg!("account_infos {:?}", account_infos);
        msg!("instruction {:?}", instruction);
        // TODO: make invoke_signed
        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Generic<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}
