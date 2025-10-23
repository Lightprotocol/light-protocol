use anchor_lang::{prelude::*, solana_program::program::invoke};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
};
use light_ctoken_types::instructions::transfer2::MultiInputTokenDataWithContext;
use light_sdk::{
    account::LightAccount,
    cpi::{v2::LightSystemProgramCpi, InvokeLightSystemProgram, LightCpiInstruction},
    instruction::ValidityProof,
};
use light_sdk_types::{
    cpi_accounts::{v2::CpiAccounts as CpiAccountsSmall, CpiAccountsConfig},
    cpi_context_write::CpiContextWriteAccounts,
};

use crate::{process_update_deposit::CompressedEscrowPda, PdaParams, LIGHT_CPI_SIGNER};

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
    pub solana_token_account: u8,
    pub authority: u8,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct FourTransfer2Params {
    pub compress_1: CompressParams,
    pub transfer_2: TransferParams,
    pub transfer_3: TransferParams,
}

pub fn process_four_transfer2<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    output_tree_index: u8,
    proof: ValidityProof,
    system_accounts_start_offset: u8,
    packed_accounts_start_offset: u8,
    four_invokes_params: FourTransfer2Params,
    pda_params: PdaParams,
) -> Result<()> {
    {
        // Debug prints for CPI struct values
        msg!("=== PROGRAM DEBUG - CPI STRUCT VALUES ===");
        msg!("output_tree_index: {}", output_tree_index);
        msg!(
            "system_accounts_start_offset: {}",
            system_accounts_start_offset
        );
        msg!(
            "packed_accounts_start_offset: {}",
            packed_accounts_start_offset
        );
        msg!("signer: {}", ctx.accounts.signer.key());

        msg!("compress_1.mint: {}", four_invokes_params.compress_1.mint);
        msg!(
            "compress_1.amount: {}",
            four_invokes_params.compress_1.amount
        );
        msg!(
            "compress_1.recipient: {}",
            four_invokes_params.compress_1.recipient
        );
        msg!(
            "compress_1.solana_token_account: {}",
            four_invokes_params.compress_1.solana_token_account
        );

        msg!(
            "transfer_2.transfer_amount: {}",
            four_invokes_params.transfer_2.transfer_amount
        );
        msg!(
            "transfer_2.recipient: {}",
            four_invokes_params.transfer_2.recipient
        );
        msg!(
            "transfer_2.token_metas len: {}",
            four_invokes_params.transfer_2.token_metas.len()
        );
        for (i, meta) in four_invokes_params
            .transfer_2
            .token_metas
            .iter()
            .enumerate()
        {
            msg!("  transfer_2.token_metas[{}].amount: {}", i, meta.amount);
            msg!(
                "  transfer_2.token_metas[{}].merkle_context.merkle_tree_pubkey_index: {}",
                i,
                meta.merkle_context.merkle_tree_pubkey_index
            );
            msg!("  transfer_2.token_metas[{}].mint: {}", i, meta.mint);
            msg!("  transfer_2.token_metas[{}].owner: {}", i, meta.owner);
        }

        msg!(
            "transfer_3.transfer_amount: {}",
            four_invokes_params.transfer_3.transfer_amount
        );
        msg!(
            "transfer_3.recipient: {}",
            four_invokes_params.transfer_3.recipient
        );
        msg!(
            "transfer_3.token_metas len: {}",
            four_invokes_params.transfer_3.token_metas.len()
        );
        for (i, meta) in four_invokes_params
            .transfer_3
            .token_metas
            .iter()
            .enumerate()
        {
            msg!("  transfer_3.token_metas[{}].amount: {}", i, meta.amount);
            msg!(
                "  transfer_3.token_metas[{}].merkle_context.merkle_tree_pubkey_index: {}",
                i,
                meta.merkle_context.merkle_tree_pubkey_index
            );
            msg!("  transfer_3.token_metas[{}].mint: {}", i, meta.mint);
            msg!("  transfer_3.token_metas[{}].owner: {}", i, meta.owner);
        }

        msg!("pda_params.account_meta: {:?}", pda_params.account_meta);
        msg!("pda_params.existing_amount: {}", pda_params.existing_amount);

        // Debug remaining accounts
        msg!("=== REMAINING ACCOUNTS ===");
        for (i, account) in ctx.remaining_accounts.iter().enumerate() {
            msg!("  {}: {}", i, anchor_lang::Key::key(account));
        }
    }
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

    let cpi_accounts = CpiAccountsSmall::new_with_config(
        ctx.accounts.signer.as_ref(),
        system_account_infos,
        config,
    );
    msg!("cpi_accounts fee_payer {:?}", cpi_accounts.fee_payer());
    msg!("cpi_accounts authority {:?}", cpi_accounts.authority());
    msg!("cpi_accounts cpi_context {:?}", cpi_accounts.cpi_context());

    let cpi_context_account_info = CpiContextWriteAccounts {
        fee_payer: ctx.accounts.signer.as_ref(),
        authority: cpi_accounts.authority().unwrap(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: LIGHT_CPI_SIGNER,
    };

    // Invocation 4: Execute CPI context with system program
    process_update_escrow_pda(cpi_context_account_info, pda_params, proof, 0, false)?;

    {
        let mut token_account_compress = CTokenAccount2::new_empty(
            four_invokes_params.compress_1.recipient,
            four_invokes_params.compress_1.mint,
        );
        token_account_compress
            .compress_ctoken(
                four_invokes_params.compress_1.amount,
                four_invokes_params.compress_1.solana_token_account,
                four_invokes_params.compress_1.authority,
            )
            .map_err(ProgramError::from)?;

        let mut token_account_transfer_2 =
            CTokenAccount2::new(four_invokes_params.transfer_2.token_metas)
                .map_err(ProgramError::from)?;
        let transfer_recipient2 = token_account_transfer_2
            .transfer(
                four_invokes_params.transfer_2.recipient,
                four_invokes_params.transfer_2.transfer_amount,
            )
            .map_err(ProgramError::from)?;

        let mut token_account_transfer_3 =
            CTokenAccount2::new(four_invokes_params.transfer_3.token_metas)
                .map_err(ProgramError::from)?;
        let transfer_recipient3 = token_account_transfer_3
            .transfer(
                four_invokes_params.transfer_3.recipient,
                four_invokes_params.transfer_3.transfer_amount,
            )
            .map_err(ProgramError::from)?;

        msg!("tree_pubkeys {:?}", cpi_accounts.tree_pubkeys());
        let tree_accounts = cpi_accounts.tree_accounts().unwrap();
        let mut packed_accounts = Vec::with_capacity(tree_accounts.len());
        for account_info in tree_accounts {
            packed_accounts.push(account_meta_from_account_info(account_info));
        }
        msg!("packed_accounts {:?}", packed_accounts);

        let inputs = Transfer2Inputs {
            validity_proof: proof,
            transfer_config: Transfer2Config {
                cpi_context: Some(
                    light_ctoken_types::instructions::transfer2::CompressedCpiContext {
                        set_context: false,
                        first_set_context: false,
                    },
                ),
                ..Default::default()
            },
            meta_config: Transfer2AccountsMetaConfig {
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
            output_queue: output_tree_index,
        };
        let instruction = create_transfer2_instruction(inputs).map_err(ProgramError::from)?;

        let account_infos = [
            &[cpi_accounts.fee_payer().clone()][..],
            ctx.remaining_accounts,
        ]
        .concat();
        invoke(&instruction, account_infos.as_slice())?;
    }

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

pub fn process_update_escrow_pda(
    cpi_accounts: CpiContextWriteAccounts<AccountInfo>,
    pda_params: PdaParams,
    proof: ValidityProof,
    deposit_amount: u64,
    set_context: bool,
) -> Result<()> {
    let mut my_compressed_account = LightAccount::<CompressedEscrowPda>::new_mut(
        &crate::ID,
        &pda_params.account_meta,
        CompressedEscrowPda {
            owner: *cpi_accounts.fee_payer.key,
            amount: pda_params.existing_amount,
        },
    )
    .unwrap();

    my_compressed_account.amount += deposit_amount;

    if set_context {
        LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke_write_to_cpi_context_set(cpi_accounts)?;
    } else {
        LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
            .with_light_account(my_compressed_account)?
            .invoke_write_to_cpi_context_first(cpi_accounts)?;
    }

    Ok(())
}
