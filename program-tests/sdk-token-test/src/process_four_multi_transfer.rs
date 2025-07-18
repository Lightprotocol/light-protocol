use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    account2::CTokenAccount2,
    instructions::{
        multi_transfer::{
            account_metas::MultiTransferAccountsMetaConfig, create_multi_transfer_instruction_raw,
            MultiTransferConfig, MultiTransferInputsRaw,
        },
        transfer::instruction::{
            compress, transfer, CompressInputs, TransferConfig, TransferInputs,
        },
    },
    TokenAccountMeta,
};
use light_ctoken_types::instructions::multi_transfer::MultiInputTokenDataWithContext;
use light_sdk::{
    cpi::CpiAccounts, instruction::ValidityProof as LightValidityProof,
    light_account_checks::AccountInfoTrait,
};
use light_sdk_types::CpiAccountsConfig;

use crate::{process_update_deposit::process_update_escrow_pda, PdaParams};

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TransferParams {
    pub transfer_amount: u64,
    pub token_metas: Vec<MultiInputTokenDataWithContext>,
    pub recipient: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressParams {
    pub mint: u8,
    pub amount: u64,
    pub recipient: u8,
    pub spl_token_account: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct FourMultiTransferParams {
    pub compress_1: CompressParams,
    pub transfer_2: TransferParams,
    pub transfer_3: TransferParams,
}

pub fn process_four_multi_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    output_tree_index: u8,
    proof: LightValidityProof,
    system_accounts_start_offset: u8,
    packed_accounts_start_offset: u8,
    four_invokes_params: FourMultiTransferParams,
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

    {
        let mut token_account_compress = CTokenAccount2::new_empty(
            four_invokes_params.compress_1.recipient,
            four_invokes_params.compress_1.mint,
            output_tree_index,
        );
        token_account_compress
            .compress(
                four_invokes_params.compress_1.amount,
                four_invokes_params.compress_1.spl_token_account,
            )
            .map_err(ProgramError::from)?;

        let mut token_account_transfer_2 = CTokenAccount2::new(
            four_invokes_params.transfer_2.token_metas,
            output_tree_index,
        )
        .map_err(ProgramError::from)?;
        let transfer_recipient2 = token_account_transfer_2
            .transfer(
                four_invokes_params.transfer_2.recipient,
                four_invokes_params.transfer_2.transfer_amount,
                None,
            )
            .map_err(ProgramError::from)?;

        let mut token_account_transfer_3 = CTokenAccount2::new(
            four_invokes_params.transfer_3.token_metas,
            output_tree_index,
        )
        .map_err(ProgramError::from)?;
        let transfer_recipient3 = token_account_transfer_3
            .transfer(
                four_invokes_params.transfer_3.recipient,
                four_invokes_params.transfer_3.transfer_amount,
                None,
            )
            .map_err(ProgramError::from)?;

        let packed_account_infos = &ctx.remaining_accounts[packed_accounts_start_offset as usize..];

        let mut packed_accounts = Vec::with_capacity(packed_account_infos.len());
        for account_info in packed_account_infos {
            packed_accounts.push(account_meta_from_account_info(account_info));
        }
        msg!("packed_accounts {:?}", packed_accounts);

        let inputs = MultiTransferInputsRaw {
            validity_proof: proof,
            transfer_config: MultiTransferConfig {
                cpi_context: Some(CompressedCpiContext {
                    set_context: true,
                    first_set_context: true,
                    cpi_context_account_index: 0,
                }),
                ..Default::default()
            },
            meta_config: MultiTransferAccountsMetaConfig {
                fee_payer: Some(*ctx.accounts.signer.key),
                packed_accounts: Some(packed_accounts), // TODO: test that if we were to set the cpi context we don't have to pass packed accounts. (only works with transfers)
                cpi_context: Some(*cpi_accounts.cpi_context().unwrap().key),
                ..Default::default()
            },
            in_lamports: None,
            out_lamports: None,
            token_accounts: vec![
                token_account_compress,
                token_account_transfer_2,
                token_account_transfer_3,
                transfer_recipient2,
                transfer_recipient3,
            ],
        };
        let instruction =
            create_multi_transfer_instruction_raw(inputs).map_err(ProgramError::from)?;

        let account_infos = [
            &[cpi_accounts.fee_payer().clone()][..],
            ctx.remaining_accounts,
        ]
        .concat();
        invoke(&instruction, account_infos.as_slice())?;
    }
    // TODO: reverse order to 1. process_update_escrow_pda, 2. create_multi_transfer_instruction_raw

    // Invocation 4: Execute CPI context with system program
    process_update_escrow_pda(cpi_accounts, pda_params, proof, 0)?;

    Ok(())
}

#[inline]
pub fn account_meta_from_account_info(account_info: &AccountInfo) -> AccountMeta {
    AccountMeta {
        pubkey: *account_info.key,
        is_signer: account_info.is_signer,
        is_writable: account_info.is_writable,
    }
}
