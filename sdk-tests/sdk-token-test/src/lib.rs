#![allow(unexpected_cfgs)]
#![allow(clippy::too_many_arguments)]

use anchor_lang::prelude::*;
use light_compressed_token_sdk::{instructions::Recipient, TokenAccountMeta, ValidityProof};
use light_sdk::instruction::{PackedAddressTreeInfo, ValidityProof as LightValidityProof};

mod ctoken_pda;
mod pda_ctoken;
mod process_batch_compress_tokens;
mod process_compress_full_and_close;
mod process_compress_tokens;
mod process_create_compressed_account;
mod process_create_escrow_pda;
mod process_decompress_tokens;
mod process_four_invokes;
pub mod process_four_transfer2;
mod process_transfer_tokens;
mod process_update_deposit;

use light_sdk::{cpi::CpiAccounts, instruction::account_meta::CompressedAccountMeta};
pub use pda_ctoken::*;
use process_batch_compress_tokens::process_batch_compress_tokens;
use process_compress_full_and_close::process_compress_full_and_close;
use process_compress_tokens::process_compress_tokens;
use process_create_compressed_account::process_create_compressed_account;
use process_create_escrow_pda::process_create_escrow_pda;
use process_decompress_tokens::process_decompress_tokens;
use process_four_invokes::process_four_invokes;
pub use process_four_invokes::{CompressParams, FourInvokesParams, TransferParams};
use process_four_transfer2::process_four_transfer2;
use process_transfer_tokens::process_transfer_tokens;

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TokenParams {
    pub deposit_amount: u64,
    pub depositing_token_metas: Vec<TokenAccountMeta>,
    pub mint: Pubkey,
    pub escrowed_token_meta: TokenAccountMeta,
    pub recipient_bump: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PdaParams {
    pub account_meta: CompressedAccountMeta,
    pub existing_amount: u64,
}
use light_sdk::address::v1::derive_address;
use light_sdk_types::CpiAccountsConfig;

use crate::{
    ctoken_pda::*, process_create_compressed_account::deposit_tokens,
    process_four_transfer2::FourTransfer2Params, process_update_deposit::process_update_deposit,
};

#[program]
pub mod sdk_token_test {

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

    pub fn compress_full_and_close<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient_index: u8,
        mint_index: u8,
        source_index: u8,
        authority_index: u8,
        close_recipient_index: u8,
        system_accounts_offset: u8,
    ) -> Result<()> {
        process_compress_full_and_close(
            ctx,
            output_tree_index,
            recipient_index,
            mint_index,
            source_index,
            authority_index,
            close_recipient_index,
            system_accounts_offset,
        )
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
        let light_cpi_accounts = CpiAccounts::new_with_config(
            ctx.accounts.signer.as_ref(),
            system_account_infos,
            config,
        );
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
        system_accounts_start_offset: u8,
        token_params: TokenParams,
        pda_params: PdaParams,
    ) -> Result<()> {
        process_update_deposit(
            ctx,
            output_tree_index,
            output_tree_queue_index,
            proof,
            system_accounts_start_offset,
            token_params,
            pda_params,
        )
    }

    pub fn four_invokes<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        proof: LightValidityProof,
        system_accounts_start_offset: u8,
        four_invokes_params: FourInvokesParams,
        pda_params: PdaParams,
    ) -> Result<()> {
        process_four_invokes(
            ctx,
            output_tree_index,
            proof,
            system_accounts_start_offset,
            four_invokes_params,
            pda_params,
        )
    }

    pub fn four_transfer2<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        proof: LightValidityProof,
        system_accounts_start_offset: u8,
        packed_accounts_start_offset: u8,
        four_transfer2_params: FourTransfer2Params,
        pda_params: PdaParams,
    ) -> Result<()> {
        process_four_transfer2(
            ctx,
            output_tree_index,
            proof,
            system_accounts_start_offset,
            packed_accounts_start_offset,
            four_transfer2_params,
            pda_params,
        )
    }

    pub fn create_escrow_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        proof: LightValidityProof,
        output_tree_index: u8,
        amount: u64,
        address: [u8; 32],
        new_address_params: light_sdk::address::PackedNewAddressParams,
    ) -> Result<()> {
        process_create_escrow_pda(
            ctx,
            proof,
            output_tree_index,
            amount,
            address,
            new_address_params,
        )
    }

    pub fn pda_ctoken<'info>(
        ctx: Context<'_, '_, '_, 'info, PdaCToken<'info>>,
        input: ChainedCtokenInstructionData,
    ) -> Result<()> {
        process_pda_ctoken(ctx, input)
    }

    pub fn ctoken_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CTokenPda<'info>>,
        input: ChainedCtokenInstructionData,
    ) -> Result<()> {
        process_ctoken_pda(ctx, input)
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
