//! Runtime helpers for token decompression.
// Re-export TokenSeedProvider from sdk (canonical definition).
pub use light_sdk::interface::TokenSeedProvider;
use light_sdk::{cpi::v2::CpiAccounts, instruction::ValidityProof};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use light_token_interface::instructions::{
    extensions::CompressToPubkey, transfer2::MultiInputTokenDataWithContext,
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{compat::PackedCTokenData, pack::Unpack};

/// Token decompression processor.
///
/// Handles both program-owned tokens and ATAs in unified flow.
/// - Program-owned tokens: program signs via CPI with seeds
/// - ATAs: wallet owner signs on transaction (no program signing needed)
///
/// CPI context usage:
/// - has_prior_context=true: PDAs/Mints already wrote to CPI context, tokens CONSUME it
/// - has_prior_context=false: tokens-only flow, no CPI context needed
///
/// After Phase 8 refactor: V is `PackedTokenAccountVariant` which unpacks to
/// `TokenAccountVariant` containing resolved seed Pubkeys. No accounts struct needed.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_decompress_tokens_runtime<'info, 'b, V>(
    _remaining_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    token_rent_sponsor: &AccountInfo<'info>,
    token_cpi_authority: &AccountInfo<'info>,
    token_config: &AccountInfo<'info>,
    config: &AccountInfo<'info>,
    token_accounts: Vec<(
        PackedCTokenData<V>,
        CompressedAccountMetaNoLamportsNoAddress,
    )>,
    proof: ValidityProof,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    post_system_accounts: &[AccountInfo<'info>],
    has_prior_context: bool,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    V: Unpack + Copy,
    V::Unpacked: TokenSeedProvider,
{
    if token_accounts.is_empty() {
        return Ok(());
    }

    let mut token_decompress_indices: Vec<
        crate::compressed_token::decompress_full::DecompressFullIndices,
    > = Vec::with_capacity(token_accounts.len());
    // Only program-owned tokens need signer seeds
    let mut token_signers_seed_groups: Vec<Vec<Vec<u8>>> = Vec::with_capacity(token_accounts.len());
    let packed_accounts = post_system_accounts;

    // CPI context usage for token decompression:
    // - If has_prior_context: PDAs/Mints already wrote to CPI context, tokens CONSUME it
    // - If !has_prior_context: tokens-only flow, execute directly without CPI context
    //
    // Note: CPI context supports cross-tree batching. Writes from different trees
    // are stored without validation. The only constraint is the executor's first
    // input/output must match the CPI context account's associated_merkle_tree.
    let cpi_context_pubkey = if has_prior_context {
        // PDAs/Mints wrote to context, tokens consume it
        cpi_accounts.cpi_context().ok().map(|ctx| *ctx.key)
    } else {
        // Tokens-only: execute directly without CPI context
        None
    };

    for (token_data, meta) in token_accounts.into_iter() {
        let owner_index: u8 = token_data.token_data.owner;
        let mint_index: u8 = token_data.token_data.mint;

        let mint_index_usize = mint_index as usize;
        if mint_index_usize >= packed_accounts.len() {
            msg!(
                "mint_index {} out of bounds (len: {})",
                mint_index_usize,
                packed_accounts.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }
        let mint_info = &packed_accounts[mint_index_usize];

        let owner_index_usize = owner_index as usize;
        if owner_index_usize >= packed_accounts.len() {
            msg!(
                "owner_index {} out of bounds (len: {})",
                owner_index_usize,
                packed_accounts.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }
        let owner_info = &packed_accounts[owner_index_usize];

        // Unpack the variant to get resolved seed Pubkeys
        let unpacked_variant = token_data.variant.unpack(post_system_accounts)?;

        // Program-owned token: use program-derived seeds
        let (ctoken_signer_seeds, derived_token_account_address) =
            unpacked_variant.get_seeds(program_id)?;

        if derived_token_account_address != *owner_info.key {
            msg!(
                "derived_token_account_address: {:?}",
                derived_token_account_address
            );
            msg!("owner_info.key: {:?}", owner_info.key);
            return Err(ProgramError::InvalidAccountData);
        }

        // Derive the authority PDA that will own this CToken account (like cp-swap's vault_authority)
        let (_authority_seeds, derived_authority_pda) =
            unpacked_variant.get_authority_seeds(program_id)?;

        let seed_refs: Vec<&[u8]> = ctoken_signer_seeds.iter().map(|s| s.as_slice()).collect();
        let seeds_slice: &[&[u8]] = &seed_refs;

        // Build CompressToPubkey from the token account seeds
        // This ensures compressed TokenData.owner = token account address (not authority)
        let compress_to_pubkey = ctoken_signer_seeds
            .last()
            .and_then(|b| b.first().copied())
            .map(|bump| {
                let seeds_without_bump: Vec<Vec<u8>> = ctoken_signer_seeds
                    .iter()
                    .take(ctoken_signer_seeds.len().saturating_sub(1))
                    .cloned()
                    .collect();
                CompressToPubkey {
                    bump,
                    program_id: program_id.to_bytes(),
                    seeds: seeds_without_bump,
                }
            });

        crate::token::CreateTokenAccountCpi {
            payer: fee_payer.clone(),
            account: (*owner_info).clone(),
            mint: (*mint_info).clone(),
            owner: derived_authority_pda, // Use derived authority PDA (like cp-swap's vault_authority)
        }
        .invoke_signed_with(
            crate::token::CompressibleParamsCpi {
                compressible_config: token_config.clone(),
                rent_sponsor: token_rent_sponsor.clone(),
                system_program: cpi_accounts
                    .system_program()
                    .map_err(|_| ProgramError::InvalidAccountData)?
                    .clone(),
                pre_pay_num_epochs: 2,
                lamports_per_write: None,
                compress_to_account_pubkey: compress_to_pubkey,
                token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
                compression_only: false,
            },
            &[seeds_slice],
        )?;

        let source = MultiInputTokenDataWithContext {
            owner: token_data.token_data.owner,
            amount: token_data.token_data.amount,
            has_delegate: token_data.token_data.has_delegate,
            delegate: token_data.token_data.delegate,
            mint: token_data.token_data.mint,
            version: token_data.token_data.version,
            merkle_context: meta.tree_info.into(),
            root_index: meta.tree_info.root_index,
        };
        let decompress_index = crate::compressed_token::decompress_full::DecompressFullIndices {
            source,
            destination_index: owner_index,
            tlv: None,
            is_ata: false, // Program-owned token: owner is a signer (via CPI seeds)
        };
        token_decompress_indices.push(decompress_index);
        token_signers_seed_groups.push(ctoken_signer_seeds);
    }

    if token_decompress_indices.is_empty() {
        return Ok(());
    }

    let ctoken_ix =
        crate::compressed_token::decompress_full::decompress_full_token_accounts_with_indices(
            *fee_payer.key,
            proof,
            cpi_context_pubkey,
            &token_decompress_indices,
            packed_accounts,
        )
        .map_err(ProgramError::from)?;

    // Build account infos for CPI. Must include all accounts needed by the transfer2 instruction:
    // - System accounts (light_system_program, registered_program_pda, etc.)
    // - Fee payer, ctoken accounts
    // - CPI context (if present)
    // - All packed accounts (post_system_accounts)
    let mut all_account_infos: Vec<AccountInfo<'info>> =
        Vec::with_capacity(12 + post_system_accounts.len());
    all_account_infos.push(fee_payer.clone());
    all_account_infos.push(token_cpi_authority.clone());
    all_account_infos.push(token_program.clone());
    all_account_infos.push(token_rent_sponsor.clone());
    all_account_infos.push(config.clone());

    // Add required system accounts for transfer2 instruction
    // Light system program is at index 0 in the cpi_accounts slice
    all_account_infos.push(
        cpi_accounts
            .account_infos()
            .first()
            .ok_or(ProgramError::NotEnoughAccountKeys)?
            .clone(),
    );
    all_account_infos.push(
        cpi_accounts
            .registered_program_pda()
            .map_err(|_| ProgramError::InvalidAccountData)?
            .clone(),
    );
    all_account_infos.push(
        cpi_accounts
            .account_compression_authority()
            .map_err(|_| ProgramError::InvalidAccountData)?
            .clone(),
    );
    all_account_infos.push(
        cpi_accounts
            .account_compression_program()
            .map_err(|_| ProgramError::InvalidAccountData)?
            .clone(),
    );
    all_account_infos.push(
        cpi_accounts
            .system_program()
            .map_err(|_| ProgramError::InvalidAccountData)?
            .clone(),
    );

    // Add CPI context if present
    if let Ok(cpi_context) = cpi_accounts.cpi_context() {
        all_account_infos.push(cpi_context.clone());
    }

    all_account_infos.extend_from_slice(post_system_accounts);

    // Only include signer seeds for program-owned tokens
    if token_signers_seed_groups.is_empty() {
        // All tokens were ATAs - no program signing needed
        solana_cpi::invoke(&ctoken_ix, all_account_infos.as_slice())?;
    } else {
        let signer_seed_refs: Vec<Vec<&[u8]>> = token_signers_seed_groups
            .iter()
            .map(|group| group.iter().map(|s| s.as_slice()).collect())
            .collect();
        let signer_seed_slices: Vec<&[&[u8]]> =
            signer_seed_refs.iter().map(|g| g.as_slice()).collect();

        solana_cpi::invoke_signed(
            &ctoken_ix,
            all_account_infos.as_slice(),
            signer_seed_slices.as_slice(),
        )?;
    }

    Ok(())
}
