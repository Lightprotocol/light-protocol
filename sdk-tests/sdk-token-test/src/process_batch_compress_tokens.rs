use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_ctoken_sdk::compressed_token::batch_compress::{
    create_batch_compress_instruction, BatchCompressInputs, Recipient,
};
use light_token_types::account_infos::BatchCompressAccountInfos;

use crate::Generic;

pub fn process_batch_compress_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    recipients: Vec<Recipient>,
    token_pool_index: u8,
    token_pool_bump: u8,
) -> Result<()> {
    let light_cpi_accounts = BatchCompressAccountInfos::new(
        ctx.accounts.signer.as_ref(),
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
    );

    let sdk_recipients: Vec<Recipient> = recipients
        .into_iter()
        .map(|r| Recipient {
            pubkey: r.pubkey,
            amount: r.amount,
        })
        .collect();

    let batch_compress_inputs = BatchCompressInputs {
        fee_payer: *ctx.accounts.signer.key,
        authority: *ctx.accounts.signer.key,
        spl_interface_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
        sender_token_account: *light_cpi_accounts.sender_token_account().unwrap().key,
        token_program: *light_cpi_accounts.token_program().unwrap().key,
        merkle_tree: *light_cpi_accounts.merkle_tree().unwrap().key,
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
