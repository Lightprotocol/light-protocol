use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::instructions::transfer::{
    instruction::{compress, CompressInputs},
    TransferAccountInfos,
};

use crate::Generic;

pub fn process_compress_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    output_tree_index: u8,
    recipient: Pubkey,
    mint: Pubkey,
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
        output_tree_index,
        token_pool_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
        transfer_config: None,
        spl_token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
        tree_accounts: light_cpi_accounts.tree_pubkeys().unwrap(),
    };

    let instruction = compress(compress_inputs).map_err(ProgramError::from)?;
    msg!("instruction {:?}", instruction);
    let account_infos = light_cpi_accounts.to_account_infos();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
