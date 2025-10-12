use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::instruction::{
        compress, transfer, CompressInputs, TransferConfig, TransferInputs,
    },
    TokenAccountMeta,
};
use light_sdk::{
    cpi::v2::CpiAccounts, instruction::ValidityProof as LightValidityProof,
    light_account_checks::AccountInfoTrait,
};
use light_sdk_types::cpi_accounts::CpiAccountsConfig;

use crate::{process_update_deposit::process_update_escrow_pda, PdaParams};

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
    pub token_account: Pubkey,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct FourInvokesParams {
    pub compress_1: CompressParams,
    pub transfer_2: TransferParams,
    pub transfer_3: TransferParams,
}

pub fn process_four_invokes<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    output_tree_index: u8,
    proof: LightValidityProof,
    system_accounts_start_offset: u8,
    four_invokes_params: FourInvokesParams,
    pda_params: PdaParams,
) -> Result<()> {
    // Parse CPI accounts once for the final system program invocation
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_start_offset as usize);

    let cpi_accounts =
        CpiAccounts::new_with_config(ctx.accounts.signer.as_ref(), system_account_infos, config);

    // Invocation 1: Compress mint 1 (writes to CPI context)
    compress_tokens_with_cpi_context(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.compress_1.mint,
        four_invokes_params.compress_1.recipient,
        four_invokes_params.compress_1.amount,
        output_tree_index,
    )?;

    // Invocation 2: Transfer mint 2 (writes to CPI context)
    transfer_tokens_with_cpi_context(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.transfer_2.mint,
        four_invokes_params.transfer_2.transfer_amount,
        four_invokes_params.transfer_2.recipient,
        output_tree_index,
        four_invokes_params.transfer_2.token_metas,
    )?;

    // Invocation 3: Transfer mint 3 (writes to CPI context)
    transfer_tokens_with_cpi_context(
        &cpi_accounts,
        ctx.remaining_accounts,
        four_invokes_params.transfer_3.mint,
        four_invokes_params.transfer_3.transfer_amount,
        four_invokes_params.transfer_3.recipient,
        output_tree_index,
        four_invokes_params.transfer_3.token_metas,
    )?;

    // Invocation 4: Execute CPI context with system program
    process_update_escrow_pda(cpi_accounts, pda_params, proof, 0)?;

    Ok(())
}

fn transfer_tokens_with_cpi_context<'a, 'info>(
    cpi_accounts: &CpiAccounts<'a, 'info>,
    remaining_accounts: &[AccountInfo<'info>],
    mint: Pubkey,
    amount: u64,
    recipient: Pubkey,
    output_tree_index: u8,
    token_metas: Vec<TokenAccountMeta>,
) -> Result<()> {
    let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;

    // Create sender account from token metas using CTokenAccount::new
    let sender_account = CTokenAccount::new(
        mint,
        *cpi_accounts.fee_payer().key,
        token_metas,
        output_tree_index,
    );

    // Get tree pubkeys excluding the CPI context account (first account)
    // We already pass the cpi context pubkey separately.
    let tree_account_infos = cpi_accounts.tree_accounts().unwrap();
    let tree_account_infos = &tree_account_infos[1..];
    let tree_pubkeys = tree_account_infos
        .iter()
        .map(|x| x.pubkey())
        .collect::<Vec<Pubkey>>();

    let transfer_inputs = TransferInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        validity_proof: None.into(),
        sender_account,
        amount,
        recipient,
        tree_pubkeys,
        config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: false,
                cpi_context_account_index: 0,
            }),
            cpi_context_pubkey: Some(cpi_context_pubkey),
            ..Default::default()
        }),
    };

    let instruction = transfer(transfer_inputs).map_err(ProgramError::from)?;

    let account_infos = [&[cpi_accounts.fee_payer().clone()][..], remaining_accounts].concat();
    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}

fn compress_tokens_with_cpi_context<'a, 'info>(
    cpi_accounts: &CpiAccounts<'a, 'info>,
    remaining_accounts: &[AccountInfo<'info>],
    mint: Pubkey,
    recipient: Pubkey,
    amount: u64,
    output_tree_index: u8,
) -> Result<()> {
    let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
    let compress_inputs = CompressInputs {
        fee_payer: *cpi_accounts.fee_payer().key,
        authority: *cpi_accounts.fee_payer().key,
        mint,
        recipient,
        sender_token_account: *remaining_accounts[0].key,
        amount,
        output_tree_index,
        // output_queue_pubkey: *cpi_accounts.tree_accounts().unwrap()[0].key,
        token_pool_pda: *remaining_accounts[1].key,
        transfer_config: Some(TransferConfig {
            cpi_context: Some(CompressedCpiContext {
                set_context: true,
                first_set_context: true,
                cpi_context_account_index: 0,
            }),
            cpi_context_pubkey: Some(cpi_context_pubkey),
            ..Default::default()
        }),
        spl_token_program: *remaining_accounts[2].key,
        tree_accounts: cpi_accounts.tree_pubkeys().unwrap(),
    };

    let instruction = compress(compress_inputs).map_err(ProgramError::from)?;

    // order doesn't matter in account infos with solana program only with pinocchio it matters.
    let account_infos = [&[cpi_accounts.fee_payer().clone()][..], remaining_accounts].concat();
    invoke(&instruction, account_infos.as_slice())?;

    Ok(())
}
