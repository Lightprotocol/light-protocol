//! Runtime processor for compress_accounts_idempotent instruction.
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
pub use light_sdk::compressible::CompressContext;
use light_sdk::cpi::{
    v2::{CpiAccounts, LightSystemProgramCpi},
    InvokeLightSystemProgram, LightCpiInstruction,
};
use light_sdk_types::{
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Processor for compress_accounts_idempotent.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_compress_accounts_idempotent<'info, Ctx>(
    ctx: &Ctx,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    signer_seeds: Vec<Vec<Vec<u8>>>,
    system_accounts_offset: u8,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    Ctx: CompressContext<'info>,
{
    // TODO: pass proof.
    let proof = light_sdk::instruction::ValidityProof::new(None);

    let compression_config =
        light_sdk::compressible::CompressibleConfig::load_checked(ctx.config(), program_id)?;

    if *ctx.rent_sponsor().key != compression_config.rent_sponsor {
        return Err(ProgramError::Custom(0)); // InvalidRentSponsor
    }

    let pda_and_token_accounts_start = remaining_accounts.len() - signer_seeds.len();
    let solana_accounts = &remaining_accounts[pda_and_token_accounts_start..];

    // Check if we have accounts to compress
    let (mut has_tokens, mut has_pdas) = (false, false);
    for account_info in solana_accounts.iter() {
        if account_info.data_is_empty() {
            continue;
        }
        if account_info.owner == &crate::ctoken::CTOKEN_PROGRAM_ID {
            has_tokens = true;
        } else if account_info.owner == program_id {
            has_pdas = true;
        }
        if has_tokens && has_pdas {
            break;
        }
    }

    if !has_tokens && !has_pdas {
        return Ok(());
    }

    let cpi_accounts = CpiAccounts::new(
        ctx.fee_payer(),
        &remaining_accounts[system_accounts_offset as usize..],
        cpi_signer,
    );

    let mut compressed_pda_infos: Vec<CompressedAccountInfo> = Vec::with_capacity(0);
    let mut token_accounts_to_compress: Vec<crate::AccountInfoToCompress<'info>> =
        Vec::with_capacity(0);
    let mut pda_indices_to_close: Vec<usize> = Vec::with_capacity(0);

    // Collect accounts to compress
    let mut pda_meta_index: usize = 0;
    for (i, account_info) in solana_accounts.iter().enumerate() {
        if account_info.data_is_empty() {
            continue;
        }

        if account_info.owner == &crate::ctoken::CTOKEN_PROGRAM_ID {
            // Token account
            let account_signer_seeds = signer_seeds[i].clone();
            token_accounts_to_compress.push(crate::AccountInfoToCompress {
                account_info: account_info.clone(),
                signer_seeds: account_signer_seeds,
            });
        } else if account_info.owner == program_id {
            // PDA account
            let meta = compressed_accounts[pda_meta_index];
            pda_meta_index += 1;

            if let Some(compressed_info) = ctx.compress_pda_account(
                account_info,
                &meta,
                &cpi_accounts,
                &compression_config,
                program_id,
            )? {
                compressed_pda_infos.push(compressed_info);
                pda_indices_to_close.push(i);
            }
        }
    }

    let has_pdas = !compressed_pda_infos.is_empty();
    let has_tokens = !token_accounts_to_compress.is_empty();

    // Compress tokens
    if has_tokens {
        let system_offset = cpi_accounts.system_accounts_end_offset();
        let post_system = &cpi_accounts.to_account_infos()[system_offset..];
        let tree_accounts = cpi_accounts
            .tree_accounts()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let output_queue = &tree_accounts[0];
        let cpi_authority = cpi_accounts
            .authority()
            .map_err(|_| ProgramError::InvalidAccountData)?;

        crate::instructions::compress_and_close::compress_and_close_ctoken_accounts_signed(
            &token_accounts_to_compress,
            ctx.fee_payer().clone(),
            output_queue.clone(),
            ctx.ctoken_rent_sponsor().clone(),
            ctx.ctoken_cpi_authority().clone(),
            cpi_authority.clone(),
            post_system,
            &cpi_accounts.to_account_infos(),
            true,
        )?;
    }

    // Compress PDAs and close
    if has_pdas {
        LightSystemProgramCpi::new_cpi(cpi_signer, proof)
            .with_account_infos(&compressed_pda_infos)
            .invoke(cpi_accounts.clone())?;

        for idx in pda_indices_to_close.into_iter() {
            let mut info = solana_accounts[idx].clone();
            light_sdk::compressible::close::close(&mut info, ctx.rent_sponsor().clone())
                .map_err(ProgramError::from)?;
        }
    }

    Ok(())
}
