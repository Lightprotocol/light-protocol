#![allow(unexpected_cfgs)]

use anchor_lang::{prelude::*, solana_program::program::invoke, Discriminator};
use light_compressed_token_sdk::instructions::transfer::instruction::DecompressInputs;
use light_compressed_token_sdk::instructions::Recipient;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::{
        batch_compress::{create_batch_compress_instruction, BatchCompressInputs},
        transfer::{
            instruction::{compress, decompress, transfer, CompressInputs, TransferInputs},
            TransferAccountInfos,
        },
    },
    TokenAccountMeta, ValidityProof,
};

declare_id!("5p1t1GAaKtK1FKCh5Hd2Gu8JCu3eREhJm4Q2qYfTEPYK");

#[program]
pub mod sdk_token_test {

    use super::*;

    pub fn compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        output_tree_index: u8,
        recipient: Pubkey, // TODO: make recpient pda
        mint: Pubkey,      // TODO: deserialize from token account.
        amount: u64,
    ) -> Result<()> {
        let light_cpi_accounts = TransferAccountInfos::new_compress(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );

        let compress_inputs = CompressInputs {
            fee_payer: *ctx.accounts.signer.key,
            authority: *ctx.accounts.signer.key,
            mint,
            recipient,
            sender_token_account: *light_cpi_accounts.sender_token_account().unwrap().key,
            amount,
            // can be hardcoded as 0, exposed for flexibility
            // and as marker that a tree has to be provided.
            output_tree_index,
            output_queue_pubkey: *light_cpi_accounts.tree_accounts().unwrap()[0].key,
            token_pool_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
            transfer_config: None,
            spl_token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
        };

        let instruction = compress(compress_inputs).map_err(ProgramError::from)?;
        msg!("instruction {:?}", instruction);
        let account_infos = light_cpi_accounts.to_account_infos();

        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    pub fn transfer_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_metas: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
        recipient: Pubkey,
    ) -> Result<()> {
        let light_cpi_accounts = TransferAccountInfos::new(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );
        let sender_account = CTokenAccount::new(
            mint,
            ctx.accounts.signer.key(),
            token_metas,
            // We pack the accounts offchain.
            output_tree_index,
        );
        let transfer_inputs = TransferInputs {
            fee_payer: ctx.accounts.signer.key(),
            // This way we can use CTokenAccount as anchor account type
            sender_account,
            validity_proof,
            recipient,
            // This is necessary for on and offchain compatibility.
            // This is not an optimal solution because we collect pubkeys into a vector.
            tree_pubkeys: light_cpi_accounts.tree_pubkeys().unwrap(),
            config: None,
            amount: 10,
        };
        let instruction = transfer(transfer_inputs).unwrap();

        let account_infos = light_cpi_accounts.to_account_infos();

        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    pub fn decompress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        validity_proof: ValidityProof,
        token_data: Vec<TokenAccountMeta>,
        output_tree_index: u8,
        mint: Pubkey,
    ) -> Result<()> {
        let sender_account = light_compressed_token_sdk::account::CTokenAccount::new(
            mint,
            ctx.accounts.signer.key(),
            token_data,
            output_tree_index,
        );

        let light_cpi_accounts = TransferAccountInfos::new_decompress(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );

        let inputs = DecompressInputs {
            fee_payer: *ctx.accounts.signer.key,
            validity_proof,
            sender_account,
            amount: 10,
            tree_pubkeys: light_cpi_accounts.tree_pubkeys().unwrap(),
            token_pool_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
            recipient_token_account: *light_cpi_accounts.decompression_recipient().unwrap().key,
            // TODO: consider replacing with token program id
            spl_token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
            config: None,
        };

        let instruction = decompress(inputs).unwrap();
        let account_infos = light_cpi_accounts.to_account_infos();

        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }

    pub fn batch_compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
        recipients: Vec<Recipient>,
        _output_tree_index: u8,
        _mint: Pubkey,
        token_pool_index: u8,
        token_pool_bump: u8,
    ) -> Result<()> {
        let light_cpi_accounts = TransferAccountInfos::new_compress(
            ctx.accounts.signer.as_ref(),
            ctx.accounts.signer.as_ref(),
            ctx.remaining_accounts,
        );

        // Convert local Recipient to SDK Recipient
        let sdk_recipients: Vec<
            light_compressed_token_sdk::instructions::batch_compress::Recipient,
        > = recipients
            .into_iter()
            .map(
                |r| light_compressed_token_sdk::instructions::batch_compress::Recipient {
                    pubkey: r.pubkey,
                    amount: r.amount,
                },
            )
            .collect();

        let batch_compress_inputs = BatchCompressInputs {
            fee_payer: *ctx.accounts.signer.key,
            authority: *ctx.accounts.signer.key,
            token_pool_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
            sender_token_account: *light_cpi_accounts.sender_token_account().unwrap().key,
            token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
            merkle_tree: *light_cpi_accounts.tree_accounts().unwrap()[0].key,
            recipients: sdk_recipients,
            lamports: None,
            token_pool_index,
            token_pool_bump,
            sol_pool_pda: None,
        };

        let instruction =
            create_batch_compress_instruction(batch_compress_inputs).map_err(ProgramError::from)?;
        msg!("batch compress instruction {:?}", instruction);
        let account_infos = light_cpi_accounts.to_account_infos();

        invoke(&instruction, account_infos.as_slice())?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Generic<'info> {
    // fee payer and authority are the same
    #[account(mut)]
    pub signer: Signer<'info>,
}
