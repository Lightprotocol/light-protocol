//! Token account decompression.

use light_sdk_types::instruction::PackedStateTreeInfo;
use light_token_interface::instructions::extensions::ExtensionInstructionData;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::create_token_account::{
    build_create_ata_instruction, build_create_token_account_instruction,
};
use crate::interface::{DecompressCtx, PackedLightAccountVariantTrait};

pub fn prepare_token_account_for_decompression<'info, const SEED_COUNT: usize, P>(
    packed: &P,
    tree_info: &PackedStateTreeInfo,
    output_queue_index: u8,
    token_account_info: &AccountInfo<'info>,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
{
    let packed_accounts = ctx
        .cpi_accounts
        .packed_accounts()
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;
    let mut token_data = packed.into_in_token_data(tree_info, output_queue_index)?;

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
    let mint_pubkey = packed_accounts
        .get(token_data.mint as usize)
        .ok_or(ProgramError::InvalidAccountData)?
        .key;

    let fee_payer = ctx.cpi_accounts.fee_payer();

    // Helper to check if token account is already initialized
    // State byte at offset 108: 0=Uninitialized, 1=Initialized, 2=Frozen
    const STATE_OFFSET: usize = 108;
    let is_already_initialized = !token_account_info.data_is_empty()
        && token_account_info.data_len() > STATE_OFFSET
        && token_account_info.try_borrow_data()?[STATE_OFFSET] != 0;

    if let Some((ata_bump, wallet_owner_index)) = ata_info {
        // ATA path: use invoke() without signer seeds
        // Resolve wallet owner pubkey from packed index
        let wallet_owner_pubkey = packed_accounts
            .get(wallet_owner_index as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;

        // Idempotency check: only create ATA if it doesn't exist
        // For ATAs, we still continue with decompression even if account exists
        if token_account_info.data_is_empty() {
            let instruction = build_create_ata_instruction(
                wallet_owner_pubkey,
                mint_pubkey,
                fee_payer.key,
                token_account_info.key,
                ata_bump,
                ctx.ctoken_compressible_config.key,
                ctx.ctoken_rent_sponsor.key,
                ctx.light_config.write_top_up,
            )?;

            // Invoke WITHOUT signer seeds - ATA is derived from light token program, not our program
            anchor_lang::solana_program::program::invoke(&instruction, ctx.remaining_accounts)?;
        }

        // For ATAs, the wallet owner must sign the Transfer2 instruction (not the ATA pubkey).
        // Override token_data.owner to point to the wallet owner index.
        token_data.owner = wallet_owner_index;

        // Don't extend token_seeds for ATAs (invoke, not invoke_signed)
    } else {
        // Regular token vault path: use invoke_signed with PDA seeds
        // For regular vaults, if already initialized, skip BOTH creation AND decompression (full idempotency)
        if is_already_initialized {
            solana_msg::msg!("Token vault is already decompressed, skipping");
            return Ok(());
        }

        let bump = &[packed.bump()];
        let seeds = packed
            .seed_refs_with_bump(packed_accounts, bump)
            .map_err(|_| ProgramError::InvalidSeeds)?;

        // Resolve owner pubkey from packed index
        let owner_pubkey = packed_accounts
            .get(token_data.owner as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;

        let signer_seeds: Vec<&[u8]> = seeds.iter().copied().collect();

        let instruction = build_create_token_account_instruction(
            token_account_info.key,
            mint_pubkey,
            owner_pubkey,
            fee_payer.key,
            ctx.ctoken_compressible_config.key,
            ctx.ctoken_rent_sponsor.key,
            ctx.light_config.write_top_up,
            &signer_seeds,
            ctx.program_id,
        )?;

        // Invoke with PDA seeds
        anchor_lang::solana_program::program::invoke_signed(
            &instruction,
            ctx.remaining_accounts,
            &[signer_seeds.as_slice()],
        )?;

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
