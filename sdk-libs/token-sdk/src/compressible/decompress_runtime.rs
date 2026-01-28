//! Runtime helpers for token decompression.
// Re-export TokenSeedProvider from sdk (canonical definition).
use light_compressed_token_sdk::compressed_token::decompress_full::{
    decompress_full_token_accounts_with_indices, DecompressFullIndices,
};
pub use light_sdk::interface::TokenSeedProvider;
use light_sdk::cpi::v2::CpiAccounts;
use light_sdk::instruction::ValidityProof;
use light_sdk::Unpack;
use light_token_interface::instructions::extensions::CompressToPubkey;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// Token decompression processor.
///
/// Handles program-owned tokens in unified flow.
/// - Program-owned tokens: program signs via CPI with seeds
///
/// CPI context usage:
/// - has_prior_context=true: PDAs/Mints already wrote to CPI context, tokens CONSUME it
/// - has_prior_context=false: tokens-only flow, no CPI context needed
///
/// V is a packed token variant that unpacks to get seed Pubkeys via TokenSeedProvider.
/// DecompressFullIndices carries the pre-packed token data (MultiInputTokenDataWithContext)
/// plus destination_index, is_ata, and tlv.
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
    token_accounts: Vec<(V, DecompressFullIndices)>,
    proof: ValidityProof,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    post_system_accounts: &[AccountInfo<'info>],
    has_prior_context: bool,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    V: Unpack,
    V::Unpacked: TokenSeedProvider,
{
    if token_accounts.is_empty() {
        return Ok(());
    }

    let mut token_decompress_indices: Vec<DecompressFullIndices> =
        Vec::with_capacity(token_accounts.len());
    let mut token_signers_seed_groups: Vec<Vec<Vec<u8>>> = Vec::with_capacity(token_accounts.len());
    let packed_accounts = post_system_accounts;

    let cpi_context_pubkey = if has_prior_context {
        cpi_accounts.cpi_context().ok().map(|ctx| *ctx.key)
    } else {
        None
    };

    for (variant, indices) in token_accounts.into_iter() {
        let owner_index = indices.source.owner as usize;
        let mint_index = indices.source.mint as usize;

        if mint_index >= packed_accounts.len() {
            msg!(
                "mint_index {} out of bounds (len: {})",
                mint_index,
                packed_accounts.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }
        let mint_info = &packed_accounts[mint_index];

        if owner_index >= packed_accounts.len() {
            msg!(
                "owner_index {} out of bounds (len: {})",
                owner_index,
                packed_accounts.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }
        let owner_info = &packed_accounts[owner_index];

        // Unpack the variant to get resolved seed Pubkeys
        let unpacked_variant = variant.unpack(post_system_accounts)?;

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

        // Derive the authority PDA that will own this CToken account
        let (_authority_seeds, derived_authority_pda) =
            unpacked_variant.get_authority_seeds(program_id)?;

        let seed_refs: Vec<&[u8]> = ctoken_signer_seeds.iter().map(|s| s.as_slice()).collect();
        let seeds_slice: &[&[u8]] = &seed_refs;

        // Build CompressToPubkey from the token account seeds
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

        crate::instruction::CreateTokenAccountCpi {
            payer: fee_payer.clone(),
            account: (*owner_info).clone(),
            mint: (*mint_info).clone(),
            owner: derived_authority_pda,
        }
        .invoke_signed_with(
            crate::instruction::CompressibleParamsCpi {
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

        token_decompress_indices.push(indices);
        token_signers_seed_groups.push(ctoken_signer_seeds);
    }

    if token_decompress_indices.is_empty() {
        return Ok(());
    }

    let ctoken_ix = decompress_full_token_accounts_with_indices(
        *fee_payer.key,
        proof,
        cpi_context_pubkey,
        &token_decompress_indices,
        packed_accounts,
    )
    .map_err(ProgramError::from)?;

    {
        let mut all_account_infos: Vec<AccountInfo<'info>> =
            Vec::with_capacity(12 + post_system_accounts.len());
        all_account_infos.push(fee_payer.clone());
        all_account_infos.push(token_cpi_authority.clone());
        all_account_infos.push(token_program.clone());
        all_account_infos.push(token_rent_sponsor.clone());
        all_account_infos.push(config.clone());

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

        if let Ok(cpi_context) = cpi_accounts.cpi_context() {
            all_account_infos.push(cpi_context.clone());
        }

        all_account_infos.extend_from_slice(post_system_accounts);

        if token_signers_seed_groups.is_empty() {
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
    }

    Ok(())
}
