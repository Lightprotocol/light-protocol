use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::{
        instruction::{transfer, TransferInputs},
        TransferAccountInfos,
    },
    TokenAccountMeta, ValidityProof,
};

use crate::Generic;

pub fn process_transfer_tokens<'info>(
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
        output_tree_index,
    );
    let transfer_inputs = TransferInputs {
        fee_payer: ctx.accounts.signer.key(),
        sender_account,
        validity_proof,
        recipient,
        tree_pubkeys: light_cpi_accounts.tree_pubkeys().unwrap(),
        config: None,
        amount: 10,
    };
    let instruction = transfer(transfer_inputs).unwrap();

    let account_infos = light_cpi_accounts.to_account_infos();

    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
