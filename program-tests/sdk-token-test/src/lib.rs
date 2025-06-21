#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use light_compressed_token_sdk::instructions::Recipient;
use light_compressed_token_sdk::{TokenAccountMeta, ValidityProof};
use light_sdk::instruction::{PackedAddressTreeInfo, ValidityProof as LightValidityProof};

mod process_batch_compress_tokens;
mod process_compress_tokens;
mod process_create_compressed_account;
mod process_decompress_tokens;
mod process_transfer_tokens;
mod process_update_depost;

use light_sdk::{cpi::CpiAccounts, instruction::account_meta::CompressedAccountMeta};
use process_batch_compress_tokens::process_batch_compress_tokens;
use process_compress_tokens::process_compress_tokens;
use process_create_compressed_account::process_create_compressed_account;
use process_decompress_tokens::process_decompress_tokens;
use process_transfer_tokens::process_transfer_tokens;

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[program]
pub mod sdk_token_test {
    use anchor_lang::solana_program::pubkey;
    use light_sdk::address::v1::derive_address;
    use light_sdk_types::CpiAccountsConfig;

    use crate::{
        process_create_compressed_account::deposit_tokens,
        process_update_depost::{deposit_additional_tokens, process_update_escrow_pda},
    };

    use super::*;

    pub fn compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient: Pubkey,
        mint: Pubkey,
        amount: u64,
    ) -> Result<()> {
        process_compress_tokens(ctx, output_tree_index, recipient, mint, amount)
    }

    pub fn transfer_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_metas: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
        recipient: Pubkey,
    ) -> Result<()> {
        process_transfer_tokens(
            ctx,
            validity_proof,
            token_metas,
            output_tree_index,
            mint,
            recipient,
        )
    }

    pub fn decompress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_data: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
    ) -> Result<()> {
        process_decompress_tokens(ctx, validity_proof, token_data, output_tree_index, mint)
    }

    pub fn batch_compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        recipients: Vec<Recipient>,
        token_pool_index: u8,
        token_pool_bump: u8,
    ) -> Result<()> {
        process_batch_compress_tokens(ctx, recipients, token_pool_index, token_pool_bump)
    }

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        proof: LightValidityProof,
        address_tree_info: PackedAddressTreeInfo,
        output_tree_index: u8,
        deposit_amount: u64,
        token_metas: Vec<TokenAccountMeta>,
        mint: Pubkey,
        system_accounts_start_offset: u8,
        recipient_bump: u8,
    ) -> Result<()> {
        // It makes sense to parse accounts once.
        let config = CpiAccountsConfig {
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            // TODO: add sanity check that account is a cpi context account.
            cpi_context: true,
            // TODO: add sanity check that account is a sol_pool_pda account.
            sol_pool_pda: false,
            sol_compression_recipient: false,
        };
        let (_, system_account_infos) = ctx
            .remaining_accounts
            .split_at(system_accounts_start_offset as usize);
        // Could add with pre account infos Option<u8>
        let light_cpi_accounts = CpiAccounts::try_new_with_config(
            ctx.accounts.signer.as_ref(),
            system_account_infos,
            config,
        )
        .unwrap();
        let (address, address_seed) = derive_address(
            &[
                b"escrow",
                light_cpi_accounts.fee_payer().key.to_bytes().as_ref(),
            ],
            &address_tree_info
                .get_tree_pubkey(&light_cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
            &crate::ID,
        );
        msg!("seeds: {:?}", b"escrow");
        msg!("seeds: {:?}", address);
        msg!("recipient_bump: {:?}", recipient_bump);
        let recipient = Pubkey::create_program_address(
            &[b"escrow", &address, &[recipient_bump]],
            ctx.program_id,
        )
        .unwrap();
        deposit_tokens(
            &light_cpi_accounts,
            token_metas,
            output_tree_index,
            mint,
            recipient,
            deposit_amount,
            ctx.remaining_accounts,
        )?;
        let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

        process_create_compressed_account(
            light_cpi_accounts,
            proof,
            output_tree_index,
            deposit_amount,
            address,
            new_address_params,
        )
    }

    pub fn update_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, GenericWithAuthority<'info>>,
        proof: LightValidityProof,
        output_tree_index: u8,
        output_tree_queue_index: u8,
        deposit_amount: u64,
        depositing_token_metas: Vec<TokenAccountMeta>,
        mint: Pubkey,
        escrowed_token_meta: TokenAccountMeta,
        system_accounts_start_offset: u8,
        account_meta: CompressedAccountMeta,
        existing_amount: u64,
        recipient_bump: u8,
    ) -> Result<()> {
        // It makes sense to parse accounts once.
        let config = CpiAccountsConfig {
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            // TODO: add sanity check that account is a cpi context account.
            cpi_context: true,
            // TODO: add sanity check that account is a sol_pool_pda account.
            sol_pool_pda: false,
            sol_compression_recipient: false,
        };
        msg!(
            "crate::LIGHT_CPI_SIGNER, {:?}",
            anchor_lang::prelude::Pubkey::new_from_array(crate::LIGHT_CPI_SIGNER.cpi_signer)
        );
        msg!(
            "system_accounts_start_offset {}",
            system_accounts_start_offset
        );
        let (token_account_infos, system_account_infos) = ctx
            .remaining_accounts
            .split_at(system_accounts_start_offset as usize);
        msg!("token_account_infos: {:?}", token_account_infos);
        msg!("system_account_infos: {:?}", system_account_infos);
        // TODO: figure out why the offsets are wrong.
        // Could add with pre account infos Option<u8>
        let light_cpi_accounts = CpiAccounts::try_new_with_config(
            ctx.accounts.signer.as_ref(),
            system_account_infos,
            config,
        )
        .unwrap();
        msg!(
            "light_cpi_accounts {:?}",
            light_cpi_accounts.authority().unwrap()
        );
        let recipient = ctx.accounts.authority.key();
        deposit_additional_tokens(
            &light_cpi_accounts,
            depositing_token_metas,
            escrowed_token_meta,
            output_tree_index,
            output_tree_queue_index,
            mint,
            recipient,
            recipient_bump,
            deposit_amount,
            account_meta.address,
            ctx.remaining_accounts,
            ctx.accounts.authority.to_account_info(),
        )?;
        process_update_escrow_pda(
            light_cpi_accounts,
            account_meta,
            proof,
            existing_amount,
            deposit_amount,
        )
    }
}

#[derive(Accounts)]
pub struct Generic<'info> {
    // fee payer and authority are the same
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct GenericWithAuthority<'info> {
    // fee payer and authority are the same
    #[account(mut)]
    pub signer: Signer<'info>,
    pub authority: AccountInfo<'info>,
}
