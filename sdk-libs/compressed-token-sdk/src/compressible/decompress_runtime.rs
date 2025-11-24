//! Runtime helpers for token decompression.
use light_ctoken_types::instructions::transfer2::MultiInputTokenDataWithContext;
use light_sdk::{cpi::v2::CpiAccounts, instruction::ValidityProof};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compat::PackedCTokenData;

/// Trait for getting token account seeds.
pub trait CTokenSeedProvider: Copy {
    /// Type of accounts struct needed for seed derivation.
    type Accounts<'info>;

    /// Get seeds for the token account PDA (used for decompression).
    fn get_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;

    /// Get authority seeds for signing during compression.
    ///
    /// TODO: consider removing.
    fn get_authority_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}

/// Token decompression processor.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_decompress_tokens_runtime<'info, 'a, 'b, V, A>(
    accounts_for_seeds: &A,
    remaining_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    ctoken_program: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    ctoken_cpi_authority: &AccountInfo<'info>,
    ctoken_config: &AccountInfo<'info>,
    config: &AccountInfo<'info>,
    ctoken_accounts: Vec<(
        PackedCTokenData<V>,
        CompressedAccountMetaNoLamportsNoAddress,
    )>,
    proof: ValidityProof,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    post_system_accounts: &[AccountInfo<'info>],
    has_pdas: bool,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    V: CTokenSeedProvider<Accounts<'info> = A>,
    A: 'info,
{
    let mut token_decompress_indices: Vec<
        crate::compressed_token::decompress_full::DecompressFullIndices,
    > = Vec::with_capacity(ctoken_accounts.len());
    let mut token_signers_seed_groups: Vec<Vec<Vec<u8>>> =
        Vec::with_capacity(ctoken_accounts.len());
    let packed_accounts = post_system_accounts;

    let authority = cpi_accounts
        .authority()
        .map_err(|_| ProgramError::MissingRequiredSignature)?;
    let cpi_context_pubkey = if has_pdas {
        Some(
            *cpi_accounts
                .cpi_context()
                .map_err(|_| ProgramError::MissingRequiredSignature)?
                .key,
        )
    } else {
        None
    };

    for (token_data, meta) in ctoken_accounts.into_iter() {
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

        // Use trait method to get seeds (program-specific)
        let (ctoken_signer_seeds, derived_token_account_address) = token_data
            .variant
            .get_seeds(accounts_for_seeds, remaining_accounts)?;

        if derived_token_account_address != *owner_info.key {
            msg!(
                "derived_token_account_address: {:?}",
                derived_token_account_address
            );
            msg!("owner_info.key: {:?}", owner_info.key);
            return Err(ProgramError::InvalidAccountData);
        }

        let seed_refs: Vec<&[u8]> = ctoken_signer_seeds.iter().map(|s| s.as_slice()).collect();
        let seeds_slice: &[&[u8]] = &seed_refs;

        // Build CompressToPubkey from the signer seeds if bump is present
        let compress_to_pubkey = ctoken_signer_seeds
            .last()
            .and_then(|b| b.first().copied())
            .map(|bump| {
                let seeds_without_bump: Vec<Vec<u8>> = ctoken_signer_seeds
                    .iter()
                    .take(ctoken_signer_seeds.len().saturating_sub(1))
                    .cloned()
                    .collect();
                light_ctoken_types::instructions::extensions::compressible::CompressToPubkey {
                    bump,
                    program_id: program_id.to_bytes(),
                    seeds: seeds_without_bump,
                }
            });

        crate::ctoken::CreateCTokenAccountInfos {
            payer: fee_payer.clone(),
            account: (*owner_info).clone(),
            mint: (*mint_info).clone(),
            owner: *authority.key,
            compressible: Some(crate::ctoken::CompressibleParamsInfos {
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                system_program: cpi_accounts
                    .system_program()
                    .map_err(|_| ProgramError::InvalidAccountData)?
                    .clone(),
                pre_pay_num_epochs: 2,
                lamports_per_write: None,
                compress_to_account_pubkey: compress_to_pubkey,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            }),
        }
        .invoke_signed(&[seeds_slice])?;

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
        };
        token_decompress_indices.push(decompress_index);
        token_signers_seed_groups.push(ctoken_signer_seeds);
    }

    let ctoken_ix =
        crate::compressed_token::decompress_full::decompress_full_ctoken_accounts_with_indices(
            *fee_payer.key,
            proof,
            cpi_context_pubkey,
            &token_decompress_indices,
            packed_accounts,
        )
        .map_err(ProgramError::from)?;

    let mut all_account_infos: Vec<AccountInfo<'info>> =
        Vec::with_capacity(1 + post_system_accounts.len() + 3);
    all_account_infos.push(fee_payer.clone());
    all_account_infos.push(ctoken_cpi_authority.clone());
    all_account_infos.push(ctoken_program.clone());
    all_account_infos.push(ctoken_rent_sponsor.clone());
    all_account_infos.push(config.clone());
    all_account_infos.extend_from_slice(post_system_accounts);

    let signer_seed_refs: Vec<Vec<&[u8]>> = token_signers_seed_groups
        .iter()
        .map(|group| group.iter().map(|s| s.as_slice()).collect())
        .collect();
    let signer_seed_slices: Vec<&[&[u8]]> = signer_seed_refs.iter().map(|g| g.as_slice()).collect();

    solana_cpi::invoke_signed(
        &ctoken_ix,
        all_account_infos.as_slice(),
        signer_seed_slices.as_slice(),
    )?;

    Ok(())
}
