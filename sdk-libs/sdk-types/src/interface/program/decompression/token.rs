//! Token account decompression.

use alloc::{vec, vec::Vec};

use light_account_checks::AccountInfoTrait;
use light_token_interface::{
    instructions::extensions::ExtensionInstructionData, LIGHT_TOKEN_PROGRAM_ID,
};

use super::create_token_account::{
    build_create_ata_instruction, build_create_token_account_instruction,
};
use crate::{
    error::LightSdkTypesError,
    instruction::PackedStateTreeInfo,
    interface::program::{
        decompression::processor::DecompressCtx, variant::PackedLightAccountVariantTrait,
    },
};

pub fn prepare_token_account_for_decompression<const SEED_COUNT: usize, P, AI>(
    packed: &P,
    tree_info: &PackedStateTreeInfo,
    output_queue_index: u8,
    token_account_info: &AI,
    ctx: &mut DecompressCtx<'_, AI>,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait + Clone,
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
{
    let packed_accounts = ctx.cpi_accounts.packed_accounts()?;
    let token_data = packed.into_in_token_data(tree_info, output_queue_index)?;

    // Get TLV extension early to detect ATA
    let in_tlv: Option<Vec<ExtensionInstructionData>> = packed.into_in_tlv()?;

    // Extract ATA info from TLV if present
    let ata_info = in_tlv.as_ref().and_then(|exts| {
        exts.iter().find_map(|ext| {
            if let ExtensionInstructionData::CompressedOnly(co) = ext {
                if co.is_ata {
                    Some((co.bump, co.owner_index))
                } else {
                    None
                }
            } else {
                None
            }
        })
    });

    // Resolve mint pubkey from packed index
    let mint_key = packed_accounts
        .get(token_data.mint as usize)
        .ok_or(LightSdkTypesError::InvalidInstructionData)?
        .key();

    let fee_payer_key = ctx.cpi_accounts.fee_payer().key();

    // Idempotency: check if token account is already initialized
    // State byte at offset 108: 0=Uninitialized, 1=Initialized, 2=Frozen
    const STATE_OFFSET: usize = 108;
    let is_already_initialized = token_account_info.data_len() > STATE_OFFSET && {
        let data = token_account_info
            .try_borrow_data()
            .map_err(|_| LightSdkTypesError::ConstraintViolation)?;
        data[STATE_OFFSET] != 0
    };

    // Get token-specific references from context
    let ctoken_compressible_config_key = ctx
        .ctoken_compressible_config
        .as_ref()
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?
        .key();
    let ctoken_rent_sponsor_key = ctx
        .ctoken_rent_sponsor
        .as_ref()
        .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?
        .key();

    if let Some((_ata_bump, wallet_owner_index)) = ata_info {
        // ATA path: use invoke() without signer seeds
        let wallet_owner_key = packed_accounts
            .get(wallet_owner_index as usize)
            .ok_or(LightSdkTypesError::InvalidInstructionData)?
            .key();

        // Idempotency: only create ATA if it doesn't exist
        if token_account_info.data_len() == 0 {
            let (data, account_metas) = build_create_ata_instruction(
                &wallet_owner_key,
                &mint_key,
                &fee_payer_key,
                &token_account_info.key(),
                &ctoken_compressible_config_key,
                &ctoken_rent_sponsor_key,
                ctx.light_config.write_top_up,
            )?;

            // Invoke WITHOUT signer seeds - ATA is derived from light token program
            AI::invoke_cpi(
                &LIGHT_TOKEN_PROGRAM_ID,
                &data,
                &account_metas,
                ctx.remaining_accounts,
                &[],
            )
            .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;
        }
        // Don't extend token_seeds for ATAs (invoke, not invoke_signed)
    } else {
        // Regular token vault path: use invoke_signed with PDA seeds
        if is_already_initialized {
            return Ok(());
        }

        let bump = &[packed.bump()];
        let seeds = packed
            .seed_refs_with_bump(packed_accounts, bump)
            .map_err(|_| LightSdkTypesError::InvalidSeeds)?;

        // Derive owner pubkey from constant owner_seeds
        let owner = packed.derive_owner();

        let signer_seeds: Vec<&[u8]> = seeds.iter().copied().collect();

        let (data, account_metas) = build_create_token_account_instruction(
            &token_account_info.key(),
            &mint_key,
            &owner,
            &fee_payer_key,
            &ctoken_compressible_config_key,
            &ctoken_rent_sponsor_key,
            ctx.light_config.write_top_up,
            &signer_seeds,
            ctx.program_id,
        )?;

        // Invoke with PDA seeds
        AI::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            &data,
            &account_metas,
            ctx.remaining_accounts,
            &[signer_seeds.as_slice()],
        )
        .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;

        // Push seeds for the Transfer2 CPI (needed for invoke_signed)
        ctx.token_seeds.extend(seeds.iter().map(|s| s.to_vec()));
    }

    // Push token data for the Transfer2 CPI (common for both ATA and regular paths)
    ctx.in_token_data.push(token_data);

    // Push TLV data
    if let Some(ctx_in_tlv) = ctx.in_tlv.as_mut() {
        ctx_in_tlv.push(in_tlv.unwrap_or_default());
    } else if let Some(in_tlv) = in_tlv {
        let mut ctx_in_tlv = vec![];
        for _ in 0..ctx.in_token_data.len() - 1 {
            ctx_in_tlv.push(vec![]);
        }
        ctx_in_tlv.push(in_tlv);
        ctx.in_tlv = Some(ctx_in_tlv);
    }

    Ok(())
}
