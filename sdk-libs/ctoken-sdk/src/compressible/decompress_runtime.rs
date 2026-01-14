//! Runtime helpers for token decompression.
use light_ctoken_interface::instructions::{
    extensions::CompressToPubkey, mint_action::CpiContext,
    transfer2::MultiInputTokenDataWithContext,
};
use light_sdk::{
    cpi::v2::CpiAccounts,
    instruction::{PackedAccounts, ValidityProof},
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compat::{CompressedMintData, PackedCTokenData, TokenData};
use crate::ctoken::{
    find_cmint_address, get_associated_ctoken_address_and_bump, DecompressCMintCpiWithContext,
    SystemAccountInfos,
};
use crate::pack::{compat::InputTokenDataCompressible, Pack};

// Re-export CTokenSeedProvider from sdk (canonical definition).
pub use light_sdk::compressible::CTokenSeedProvider;

/// Pack an ATA for unified decompression with program PDAs.
///
/// This function ensures that ATAs can be included in the same `decompress_accounts_idempotent`
/// instruction as PDAs, with correctly packed indices.
///
/// # Key behaviors:
/// 1. Derives the ATA address from (owner=wallet, mint) and inserts it into packed_accounts
/// 2. Inserts wallet and mint pubkeys into packed_accounts with correct indices
/// 3. Returns `InputTokenDataCompressible` ready to wrap with your program's variant
///
/// # Arguments
/// * `token_data` - TokenData with actual pubkeys (from indexer)
/// * `packed_accounts` - MUST be the same PackedAccounts used for the entire instruction
///
/// # Returns
/// `InputTokenDataCompressible` with correctly packed indices
///
/// # Example
/// ```rust,ignore
/// use light_ctoken_sdk::compressible::pack_ata_for_unified_decompress;
/// use light_ctoken_sdk::pack::compat::{TokenData, PackedCTokenData};
///
/// // Pack ATA using the SAME PackedAccounts as the instruction
/// let packed_token_data = pack_ata_for_unified_decompress(&token_data, &mut packed_accounts);
///
/// // Wrap with your program-specific variant
/// let packed_ata = PackedCTokenData {
///     variant: CTokenAccountVariant::UserAta,
///     token_data: packed_token_data,
/// };
/// ```
pub fn pack_ata_for_unified_decompress(
    token_data: &TokenData,
    packed_accounts: &mut PackedAccounts,
) -> InputTokenDataCompressible {
    // Derive ATA address from (wallet=owner, mint) and ensure it's in packed_accounts.
    // The runtime's process_decompress_tokens_runtime will derive the same address
    // and look it up in packed_accounts.
    let (ata_address, _bump) =
        get_associated_ctoken_address_and_bump(&token_data.owner, &token_data.mint);
    packed_accounts.insert_or_get(ata_address);

    // Use existing Pack trait to convert pubkeys to indices
    token_data.pack(packed_accounts)
}

/// Token decompression processor.
///
/// Handles both program-owned tokens and ATAs in unified flow.
/// - Program-owned tokens: program signs via CPI with seeds
/// - ATAs: wallet owner signs on transaction (no program signing needed)
///
/// CPI context usage:
/// - has_prior_context=true: PDAs/Mints already wrote to CPI context, tokens CONSUME it
/// - has_prior_context=false: tokens-only flow, no CPI context needed
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
    has_prior_context: bool,
    program_id: &Pubkey,
) -> Result<(), ProgramError>
where
    V: CTokenSeedProvider<Accounts<'info> = A>,
    A: 'info,
{
    if ctoken_accounts.is_empty() {
        return Ok(());
    }

    let mut token_decompress_indices: Vec<
        crate::compressed_token::decompress_full::DecompressFullIndices,
    > = Vec::with_capacity(ctoken_accounts.len());
    // Only program-owned tokens need signer seeds
    let mut token_signers_seed_groups: Vec<Vec<Vec<u8>>> =
        Vec::with_capacity(ctoken_accounts.len());
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

    for (token_data, meta) in ctoken_accounts.into_iter() {
        let owner_index: u8 = token_data.token_data.owner;
        let mint_index: u8 = token_data.token_data.mint;
        let is_ata = token_data.variant.is_ata();

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

        if is_ata {
            // ATA: The token data's owner is the ATA ADDRESS (from compressed token).
            // We need to find the WALLET that derives to this ATA.
            // The wallet must be a signer on the transaction.
            let ata_address = *owner_info.key;
            let mint_pubkey = *mint_info.key;

            // Search for a signer wallet where derive_ctoken_ata(wallet, mint) == ata_address
            let wallet_info = packed_accounts
                .iter()
                .find(|acc| {
                    if !acc.is_signer {
                        return false;
                    }
                    let (derived, _) = crate::ctoken::derive_ctoken_ata(acc.key, &mint_pubkey);
                    derived == ata_address
                })
                .ok_or_else(|| {
                    msg!(
                        "No signer wallet found that derives to ATA: ata={:?}, mint={:?}",
                        ata_address,
                        mint_pubkey
                    );
                    ProgramError::MissingRequiredSignature
                })?;

            let wallet_pubkey = *wallet_info.key;
            let (derived_ata_address, _bump) =
                crate::ctoken::derive_ctoken_ata(&wallet_pubkey, &mint_pubkey);

            // Find the ATA account in packed_accounts
            let ata_account_index = packed_accounts
                .iter()
                .position(|a| *a.key == derived_ata_address)
                .ok_or_else(|| {
                    msg!(
                        "ATA account not found: wallet={:?}, mint={:?}, derived_ata={:?}",
                        wallet_pubkey,
                        mint_pubkey,
                        derived_ata_address
                    );
                    ProgramError::NotEnoughAccountKeys
                })?;
            let ata_account = &packed_accounts[ata_account_index];

            // Create ATA if needed (idempotent)
            // ATAs MUST have compression_only: true per ctoken program requirements
            crate::ctoken::CreateAssociatedCTokenAccountCpi {
                payer: fee_payer.clone(),
                associated_token_account: ata_account.clone(),
                owner: (*wallet_info).clone(), // wallet owner (NOT the ATA address from token data)
                mint: (*mint_info).clone(),
                system_program: cpi_accounts
                    .system_program()
                    .map_err(|_| ProgramError::InvalidAccountData)?
                    .clone(),
                bump: _bump,
                compressible: crate::ctoken::CompressibleParamsCpi {
                    compressible_config: ctoken_config.clone(),
                    rent_sponsor: ctoken_rent_sponsor.clone(),
                    system_program: cpi_accounts
                        .system_program()
                        .map_err(|_| ProgramError::InvalidAccountData)?
                        .clone(),
                    pre_pay_num_epochs: 2,
                    lamports_per_write: None,
                    compress_to_account_pubkey: None,
                    token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
                    compression_only: true, // ATAs require compression_only
                },
                idempotent: true, // Don't fail if ATA already exists
            }
            .invoke()?; // No signing needed - wallet is already a tx signer

            // Build decompress indices for ATA
            // For ATAs, we keep owner_index pointing to the ATA address (as stored in compressed token).
            // This is because the cToken Transfer2 instruction validates the owner against what's
            // in the merkle tree, and the compressed token has owner = ATA address.
            // The wallet signing is handled separately by the transaction signature.

            // Find the wallet's index in packed_accounts (needed for TLV)
            let wallet_account_index = packed_accounts
                .iter()
                .position(|a| *a.key == wallet_pubkey)
                .ok_or_else(|| {
                    msg!("Wallet account not found in packed_accounts for TLV");
                    ProgramError::NotEnoughAccountKeys
                })? as u8;

            let source = MultiInputTokenDataWithContext {
                owner: owner_index, // ATA address index (matches compressed token's owner for hash verification)
                amount: token_data.token_data.amount,
                has_delegate: token_data.token_data.has_delegate,
                delegate: token_data.token_data.delegate,
                mint: token_data.token_data.mint,
                version: token_data.token_data.version,
                merkle_context: meta.tree_info.into(),
                root_index: meta.tree_info.root_index,
            };

            // Build TLV with CompressedOnly extension for ATA
            // This tells the cToken program:
            // 1. is_ata=true: check wallet_owner signer instead of owner
            // 2. owner_index: which account is the wallet (for signer check)
            // 3. bump: for ATA derivation verification
            use light_ctoken_interface::instructions::extensions::{
                CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
            };
            let tlv = vec![ExtensionInstructionData::CompressedOnly(
                CompressedOnlyExtensionInstructionData {
                    delegated_amount: 0,
                    withheld_transfer_fee: 0,
                    is_frozen: false,
                    compression_index: 0,
                    is_ata: true,
                    bump: _bump,
                    owner_index: wallet_account_index,
                },
            )];

            let decompress_index =
                crate::compressed_token::decompress_full::DecompressFullIndices {
                    source,
                    destination_index: ata_account_index as u8,
                    tlv: Some(tlv),
                    is_ata: true, // ATA: owner is the ATA address, not a signer
                };
            token_decompress_indices.push(decompress_index);
            // Wallet is already a signer on the transaction
        } else {
            // Program-owned token: use program-derived seeds
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

            // Derive the authority PDA that will own this CToken account (like cp-swap's vault_authority)
            let (_authority_seeds, derived_authority_pda) = token_data
                .variant
                .get_authority_seeds(accounts_for_seeds, remaining_accounts)?;

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

            crate::ctoken::CreateCTokenAccountCpi {
                payer: fee_payer.clone(),
                account: (*owner_info).clone(),
                mint: (*mint_info).clone(),
                owner: derived_authority_pda, // Use derived authority PDA (like cp-swap's vault_authority)
                compressible: crate::ctoken::CompressibleParamsCpi {
                    compressible_config: ctoken_config.clone(),
                    rent_sponsor: ctoken_rent_sponsor.clone(),
                    system_program: cpi_accounts
                        .system_program()
                        .map_err(|_| ProgramError::InvalidAccountData)?
                        .clone(),
                    pre_pay_num_epochs: 2,
                    lamports_per_write: None,
                    compress_to_account_pubkey: compress_to_pubkey,
                    token_account_version: light_ctoken_interface::state::TokenDataVersion::ShaFlat,
                    compression_only: false,
                },
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
            let decompress_index =
                crate::compressed_token::decompress_full::DecompressFullIndices {
                    source,
                    destination_index: owner_index,
                    tlv: None,
                    is_ata: false, // Program-owned token: owner is a signer (via CPI seeds)
                };
            token_decompress_indices.push(decompress_index);
            token_signers_seed_groups.push(ctoken_signer_seeds);
        }
    }

    if token_decompress_indices.is_empty() {
        return Ok(());
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

    // Build account infos for CPI. Must include all accounts needed by the transfer2 instruction:
    // - System accounts (light_system_program, registered_program_pda, etc.)
    // - Fee payer, ctoken accounts
    // - CPI context (if present)
    // - All packed accounts (post_system_accounts)
    let mut all_account_infos: Vec<AccountInfo<'info>> =
        Vec::with_capacity(12 + post_system_accounts.len());
    all_account_infos.push(fee_payer.clone());
    all_account_infos.push(ctoken_cpi_authority.clone());
    all_account_infos.push(ctoken_program.clone());
    all_account_infos.push(ctoken_rent_sponsor.clone());
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

/// Mint decompression processor.
///
/// Decompresses compressed mints to CMint accounts.
///
/// CPI context usage:
/// - has_prior_context=true: PDAs already wrote to CPI context, mints add to it
/// - has_prior_context=false && has_tokens=true: mints write to CPI context first
/// - has_prior_context=false && has_tokens=false: mints-only flow, no CPI context needed
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_decompress_mints_runtime<'info, 'a, 'b, A>(
    accounts_for_config: &A,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    cmint_accounts: Vec<(CompressedMintData, CompressedAccountMetaNoLamportsNoAddress)>,
    proof: ValidityProof,
    has_prior_context: bool,
    has_tokens: bool,
) -> Result<(), ProgramError>
where
    A: MintDecompressContext<'info>,
{
    if cmint_accounts.is_empty() {
        return Ok(());
    }

    // Mints-only flow: no CPI context needed (execute directly like tokens-only)
    let mints_only = !has_prior_context && !has_tokens;

    // CPI context is only needed if we have prior context or subsequent tokens
    let cpi_context_account: Option<AccountInfo<'info>> = if mints_only {
        None
    } else {
        Some(
            cpi_accounts
                .cpi_context()
                .map_err(|_| ProgramError::MissingRequiredSignature)?
                .clone(),
        )
    };

    let mint_count = cmint_accounts.len();
    let last_mint_idx = mint_count.saturating_sub(1);

    // Build system accounts once
    let system_accounts = SystemAccountInfos {
        light_system_program: cpi_accounts
            .get_account_info(0)
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
        cpi_authority_pda: cpi_accounts
            .authority()
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
        registered_program_pda: cpi_accounts
            .registered_program_pda()
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
        account_compression_authority: cpi_accounts
            .account_compression_authority()
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
        account_compression_program: cpi_accounts
            .account_compression_program()
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
        system_program: cpi_accounts
            .system_program()
            .map_err(|_| ProgramError::NotEnoughAccountKeys)?
            .clone(),
    };

    // Get tree accounts
    let state_tree = cpi_accounts
        .get_tree_account_info(0)
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;
    let input_queue = cpi_accounts
        .get_tree_account_info(1)
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;
    let output_queue = cpi_accounts
        .get_tree_account_info(2)
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;

    let fee_payer = accounts_for_config.fee_payer();
    // Use ctoken's config, rent_sponsor, and CPI authority for CMint decompression CPI
    let ctoken_config = accounts_for_config.ctoken_config().ok_or_else(|| {
        msg!("ctoken_config is required for CMint decompression");
        ProgramError::NotEnoughAccountKeys
    })?;
    let ctoken_rent_sponsor = accounts_for_config.ctoken_rent_sponsor().ok_or_else(|| {
        msg!("ctoken_rent_sponsor is required for CMint decompression");
        ProgramError::NotEnoughAccountKeys
    })?;
    let ctoken_cpi_authority = accounts_for_config.ctoken_cpi_authority().ok_or_else(|| {
        msg!("ctoken_cpi_authority is required for CMint decompression");
        ProgramError::NotEnoughAccountKeys
    })?;
    // Use cmint_authority if provided, otherwise fall back to fee_payer
    let authority = accounts_for_config
        .cmint_authority()
        .unwrap_or_else(|| fee_payer.clone());

    // Get account infos for lookups
    let all_infos = cpi_accounts.account_infos();

    for (idx, (mint_data, _meta)) in cmint_accounts.into_iter().enumerate() {
        // Derive CMint PDA
        let (cmint_pda, _) = find_cmint_address(&mint_data.mint_seed_pubkey);

        // Get mint_seed AccountInfo from remaining accounts
        let mint_seed_info = all_infos
            .iter()
            .find(|a| *a.key == mint_data.mint_seed_pubkey)
            .cloned()
            .ok_or_else(|| {
                msg!(
                    "Mint seed pubkey not found in remaining accounts: {:?}",
                    mint_data.mint_seed_pubkey
                );
                ProgramError::NotEnoughAccountKeys
            })?;

        // Get CMint AccountInfo
        let cmint_info = all_infos
            .iter()
            .find(|a| *a.key == cmint_pda)
            .cloned()
            .ok_or_else(|| {
                msg!("CMint PDA not found in remaining accounts: {:?}", cmint_pda);
                ProgramError::NotEnoughAccountKeys
            })?;

        if mints_only {
            // Mints-only: execute directly without CPI context (like DecompressCMintCpi)
            crate::ctoken::DecompressCMintCpi {
                mint_seed: mint_seed_info,
                authority: authority.clone(),
                payer: fee_payer.clone(),
                cmint: cmint_info,
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                state_tree: state_tree.clone(),
                input_queue: input_queue.clone(),
                output_queue: output_queue.clone(),
                system_accounts: system_accounts.clone(),
                compressed_mint_with_context: mint_data.compressed_mint_with_context,
                proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(
                    proof.0,
                ),
                rent_payment: mint_data.rent_payment,
                write_top_up: mint_data.write_top_up,
            }
            .invoke()?;
        } else {
            // Multi-type flow: use CPI context batching
            let is_first_operation = !has_prior_context && idx == 0;
            let is_last_mint = idx == last_mint_idx;
            let should_execute = is_last_mint && !has_tokens;

            // For decompression, the address tree pubkey is typically not needed
            let address_tree_pubkey = [0u8; 32];

            // Determine CPI context flags
            let cpi_ctx = if should_execute {
                // Execute: consume CPI context (both flags false)
                CpiContext {
                    first_set_context: false,
                    set_context: false,
                    in_tree_index: 0,
                    in_queue_index: 0,
                    out_queue_index: 0,
                    token_out_queue_index: 0,
                    assigned_account_index: 0,
                    read_only_address_trees: [0; 4],
                    address_tree_pubkey,
                }
            } else if is_first_operation {
                // First write to CPI context
                CpiContext {
                    first_set_context: true,
                    set_context: false,
                    in_tree_index: 0,
                    in_queue_index: 0,
                    out_queue_index: 0,
                    token_out_queue_index: 0,
                    assigned_account_index: 0,
                    read_only_address_trees: [0; 4],
                    address_tree_pubkey,
                }
            } else {
                // Subsequent write to CPI context
                CpiContext {
                    first_set_context: false,
                    set_context: true,
                    in_tree_index: 0,
                    in_queue_index: 0,
                    out_queue_index: 0,
                    token_out_queue_index: 0,
                    assigned_account_index: 0,
                    read_only_address_trees: [0; 4],
                    address_tree_pubkey,
                }
            };

            DecompressCMintCpiWithContext {
                mint_seed: mint_seed_info,
                authority: authority.clone(),
                payer: fee_payer.clone(),
                cmint: cmint_info,
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                state_tree: state_tree.clone(),
                input_queue: input_queue.clone(),
                output_queue: output_queue.clone(),
                cpi_context_account: cpi_context_account
                    .as_ref()
                    .expect("CPI context required for multi-type flow")
                    .clone(),
                system_accounts: system_accounts.clone(),
                ctoken_cpi_authority: ctoken_cpi_authority.clone(),
                compressed_mint_with_context: mint_data.compressed_mint_with_context,
                proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(
                    proof.0,
                ),
                rent_payment: mint_data.rent_payment,
                write_top_up: mint_data.write_top_up,
                cpi_context: cpi_ctx,
            }
            .invoke()?;
        }
    }

    Ok(())
}

/// Trait for getting mint decompression context accounts.
pub trait MintDecompressContext<'info> {
    fn fee_payer(&self) -> AccountInfo<'info>;
    fn config(&self) -> AccountInfo<'info>;
    fn rent_sponsor(&self) -> AccountInfo<'info>;

    /// Returns the CMint authority for decompression.
    /// If None, fee_payer is used as the authority.
    /// This enables unified decompression when the mint authority is different from fee_payer.
    fn cmint_authority(&self) -> Option<AccountInfo<'info>> {
        None // Default: use fee_payer
    }

    /// Returns the ctoken compressible config account for CMint decompression CPI.
    /// This is the ctoken program's config, not the calling program's config.
    /// Required for unified CMint decompression via decompress_accounts_idempotent.
    fn ctoken_config(&self) -> Option<AccountInfo<'info>> {
        None // Default: not available
    }

    /// Returns the ctoken rent sponsor account for CMint decompression CPI.
    /// This is the ctoken program's rent sponsor, not the calling program's rent sponsor.
    /// Required for unified CMint decompression via decompress_accounts_idempotent.
    fn ctoken_rent_sponsor(&self) -> Option<AccountInfo<'info>> {
        None // Default: not available
    }

    /// Returns the ctoken program's CPI authority for CMint decompression CPI.
    /// This is the ctoken program's CPI authority PDA (GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy),
    /// not the calling program's CPI authority.
    /// Required for unified CMint decompression via decompress_accounts_idempotent.
    fn ctoken_cpi_authority(&self) -> Option<AccountInfo<'info>> {
        None // Default: not available
    }
}
