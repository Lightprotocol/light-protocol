use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::{
    instructions::transfer::{
        instruction::{decompress, DecompressInputs},
        TransferAccountInfos,
    },
    TokenAccountMeta, ValidityProof,
};

use crate::Generic;

pub fn process_decompress_tokens<'info>(
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
        spl_token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
        config: None,
    };

    let instruction = decompress(inputs).unwrap();
    let account_infos = light_cpi_accounts.to_account_infos();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
