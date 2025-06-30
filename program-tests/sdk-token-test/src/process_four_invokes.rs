use anchor_lang::prelude::*;
use light_compressed_token_sdk::TokenAccountMeta;
use light_sdk::{cpi::CpiAccounts, instruction::ValidityProof as LightValidityProof};
use light_sdk_types::CpiAccountsConfig;

use crate::{
    process_update_deposit::{process_update_escrow_pda, transfer_tokens_to_escrow_pda},
    PdaParams,
};
use anchor_lang::solana_program::program::invoke;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::instructions::transfer::{
    instruction::{compress, CompressInputs, TransferConfig},
    TransferAccountInfos,
};

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TransferParams {
    pub mint: Pubkey,
    pub transfer_amount: u64,
    pub token_metas: Vec<TokenAccountMeta>,
    pub recipient: Pubkey,
    pub recipient_bump: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressParams {
    pub mint: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub recipient_bump: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct FourInvokesParams {
    pub mint1: CompressParams,
    pub mint2: TransferParams,
    pub mint3: TransferParams,
}

pub fn process_four_invokes<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::GenericWithAuthority<'info>>,
    output_tree_index: u8,
    output_tree_queue_index: u8,
    proof: LightValidityProof,
    system_accounts_start_offset: u8,
    four_invokes_params: FourInvokesParams,
    pda_params: PdaParams,
) -> Result<()> {
    // Parse CPI accounts once
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_start_offset as usize);

    let cpi_accounts = CpiAccounts::try_new_with_config(
        ctx.accounts.signer.as_ref(),
        system_account_infos,
        config,
    )
    .unwrap();

    let address = pda_params.account_meta.address;

    // Invocation 1: Compress mint 1 (writes to CPI context)
    compress_tokens_with_cpi_context(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.mint1.mint,
        four_invokes_params.mint1.recipient,
        four_invokes_params.mint1.amount,
        output_tree_index,
    )?;

    // Invocation 2: Transfer mint 2 (writes to CPI context)
    transfer_tokens_to_escrow_pda(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.mint2.mint,
        four_invokes_params.mint2.transfer_amount,
        &four_invokes_params.mint2.recipient,
        output_tree_index,
        output_tree_queue_index,
        address,
        four_invokes_params.mint2.recipient_bump,
        four_invokes_params.mint2.token_metas,
    )?;

    // Invocation 3: Transfer mint 3 (writes to CPI context)
    transfer_tokens_to_escrow_pda(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.mint3.mint,
        four_invokes_params.mint3.transfer_amount,
        &four_invokes_params.mint3.recipient,
        output_tree_index,
        output_tree_queue_index,
        address,
        four_invokes_params.mint3.recipient_bump,
        four_invokes_params.mint3.token_metas,
    )?;

    // Invocation 4: Execute CPI context with system program
    process_update_escrow_pda(cpi_accounts, pda_params, proof, 0)?;

    Ok(())
}

fn compress_tokens_with_cpi_context<'info>(
    cpi_accounts: &CpiAccounts<'_, 'info>,
    remaining_accounts: &[AccountInfo<'info>],
    mint: Pubkey,
    recipient: Pubkey,
    amount: u64,
    output_tree_index: u8,
) -> Result<()> {
    let light_cpi_accounts = TransferAccountInfos::new_compress(
        cpi_accounts.fee_payer(),
        cpi_accounts.fee_payer(),
        remaining_accounts,
    );

    let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
    let compress_inputs = CompressInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        authority: *cpi_accounts.fee_payer().key,
        mint,
        recipient,
        sender_token_account: *light_cpi_accounts.sender_token_account().unwrap().key,
        amount,
        output_tree_index,
        output_queue_pubkey: *light_cpi_accounts.tree_accounts().unwrap()[0].key,
        token_pool_pda: *light_cpi_accounts.token_pool_pda().unwrap().key,
        transfer_config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: true,
                cpi_context_account_index: 0,
            }),
            cpi_context_pubkey: Some(cpi_context_pubkey),
            ..Default::default()
        }),
        spl_token_program: *light_cpi_accounts.spl_token_program().unwrap().key,
    };

    let instruction = compress(compress_inputs).map_err(ProgramError::from)?;
    let account_infos = light_cpi_accounts.to_account_infos();
    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
