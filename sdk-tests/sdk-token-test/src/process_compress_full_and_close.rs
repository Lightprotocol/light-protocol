use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::{
        close::close_account,
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Inputs,
        },
    },
};
use light_sdk_types::cpi_accounts::{v2::CpiAccounts, CpiAccountsConfig};

use crate::Generic;

pub fn process_compress_full_and_close<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>,
    // All offsets are static and could be hardcoded
    recipient_index: u8,
    mint_index: u8,
    source_index: u8,
    authority_index: u8,
    close_recipient_index: u8,
    system_accounts_offset: u8,
) -> Result<()> {
    // Parse CPI accounts (following four_transfer2 pattern)
    let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    // _token_account_infos should be in the anchor account struct.
    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_offset as usize);

    let cpi_accounts =
        CpiAccounts::new_with_config(ctx.accounts.signer.as_ref(), system_account_infos, config);
    let token_account_info = cpi_accounts
        .get_tree_account_info(source_index as usize)
        .unwrap();
    // should be in the anchor account struct
    let close_recipient_info = cpi_accounts
        .get_tree_account_info(close_recipient_index as usize)
        .unwrap();
    // Create CTokenAccount2 for compression (following four_transfer2 pattern)
    let mut token_account_compress = CTokenAccount2::new_empty(recipient_index, mint_index);

    // Use compress_full method
    token_account_compress
        .compress_full(
            source_index,    // source account index
            authority_index, // authority index
            token_account_info,
        )
        .map_err(ProgramError::from)?;

    msg!(
        "Compressing {} tokens",
        token_account_compress.compression_amount().unwrap_or(0)
    );

    // Create packed accounts for transfer2 instruction (following four_transfer2 pattern)
    let tree_accounts = cpi_accounts.tree_accounts().unwrap();
    let packed_accounts = account_infos_to_metas(tree_accounts);

    // create_transfer2_instruction::compress
    // create_transfer2_instruction::compress_full
    // create_transfer2_instruction::decompress
    // create_transfer2_instruction::transfer, all should hide indices completely
    //
    // Advanced:
    // 1. advanced multi transfer
    // 2. compress full and close
    // 3.
    let inputs = Transfer2Inputs {
        meta_config: Transfer2AccountsMetaConfig::new(*ctx.accounts.signer.key, packed_accounts),
        token_accounts: vec![token_account_compress],
        output_queue: 0,
        ..Default::default()
    };

    let instruction = create_transfer2_instruction(inputs).map_err(ProgramError::from)?;

    // Execute the transfer2 instruction with all accounts
    let account_infos = [
        &[cpi_accounts.fee_payer().clone()][..],
        ctx.remaining_accounts,
    ]
    .concat();
    invoke(&instruction, account_infos.as_slice())?;

    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let close_instruction = close_account(
        &compressed_token_program_id,
        token_account_info.key,
        close_recipient_info.key,
        ctx.accounts.signer.key,
    );

    invoke(
        &close_instruction,
        &[
            token_account_info.clone(),
            close_recipient_info.clone(),
            ctx.accounts.signer.to_account_info(),
        ],
    )?;
    Ok(())
}

pub fn account_infos_to_metas(account_infos: &[AccountInfo]) -> Vec<AccountMeta> {
    let mut packed_accounts = Vec::with_capacity(account_infos.len());
    for account_info in account_infos {
        packed_accounts.push(AccountMeta {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer,
            is_writable: account_info.is_writable,
        });
    }
    packed_accounts
}
