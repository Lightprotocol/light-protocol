#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
// Using the new `compressible` alias (equivalent to add_compressible_instructions)
use light_sdk_macros::compressible;
// LightFinalize approach imports
use light_sdk_macros::light_instruction;
use light_sdk_types::CpiSigner;

pub mod errors;
pub mod instruction_accounts;
pub mod state;

pub use instruction_accounts::*;
pub use state::{
    GameSession, PackedGameSession, PackedPlaceholderRecord, PackedUserRecord, PlaceholderRecord,
    UserRecord,
};

// Re-export for test usage
pub use light_ctoken_sdk::ctoken::CompressedMintWithContext;

// Example helper expression usable in seeds
#[inline]
pub fn max_key(left: &Pubkey, right: &Pubkey) -> [u8; 32] {
    if left > right {
        left.to_bytes()
    } else {
        right.to_bytes()
    }
}

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

/// Derive a program-owned rent sponsor PDA (version = 1 by default).
pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah", 1);

/// Returns the program's rent sponsor PDA as a Pubkey.
#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

// Using the new `#[compressible]` attribute (alias for add_compressible_instructions)
#[compressible(
    // Complex PDA account types with seed specifications using BOTH ctx.accounts.* AND data.*
    // UserRecord: uses ctx accounts (authority, mint_authority) + data fields (owner, category_id)
    UserRecord = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes()),
    // GameSession: uses max_key expression with ctx.accounts + data.session_id
    GameSession = ("game_session", max_key(&ctx.user.key(), &ctx.authority.key()), data.session_id.to_le_bytes()),
    // PlaceholderRecord: mixes ctx accounts and data for seeds
    PlaceholderRecord = ("placeholder_record", ctx.authority, ctx.some_account, data.placeholder_id.to_le_bytes(), data.counter.to_le_bytes()),
    // Token variant (CToken account) with authority for compression signing
    CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint, authority = LIGHT_CPI_SIGNER),
    // Program-owned CToken vault: seeds = "vault" + cmint pubkey (like cp-swap token vaults)
    // Authority = vault_authority PDA that owns the vault (like cp-swap's authority)
    Vault = (is_token, "vault", ctx.cmint, authority = ("vault_authority")),
    // NOTE: For user-owned ATAs, use standard LightAta type (no #[compressible] declaration needed)
    // NOTE: For CMint decompression, use standard LightMint type (no #[compressible] declaration needed)
    // Instruction data fields used in seed expressions above
    owner = Pubkey,
    category_id = u64,
    session_id = u64,
    placeholder_id = u64,
    counter = u32,
)]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]

    use super::*;
    use crate::{
        instruction_accounts::{
            CreatePdasAndMintAuto, DecompressAtas, DecompressAtasParams, DecompressCMints,
            DecompressCMintsParams,
        },
        state::{GameSession, PlaceholderRecord, UserRecord},
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };
    /// FULL AUTOMATIC WITH MINT: Creates 2 PDAs + 1 CMint + vault + user_ata in ONE instruction.
    /// - 2 PDAs with #[compressible] (UserRecord, GameSession)
    /// - 1 CMint with #[light_mint] (creates + decompresses atomically in pre_init)
    /// - 1 Program-owned CToken vault (created in instruction body)
    /// - 1 User CToken ATA (created in instruction body)
    /// - MintTo both vault and user_ata (in instruction body)
    ///
    /// All batched together with a single proof execution!
    ///
    /// This is the pattern used by protocols like Raydium cp-swap:
    /// - Pool state PDA (compressible)
    /// - Observation state PDA (compressible)
    /// - LP mint (light_mint - created and immediately decompressed)
    /// - Token vaults (CToken accounts)
    /// - Creator LP token (user's ATA)
    #[light_instruction(params)]
    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;
        use light_ctoken_sdk::ctoken::{
            CTokenMintToCpi, CreateAssociatedCTokenAccountCpi, CreateCTokenAccountCpi,
        };

        // Populate UserRecord - compression handled by macro
        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto Created User With Mint".to_string();
        user_record.score = 0;
        user_record.category_id = params.category_id;

        // Populate GameSession - compression handled by macro
        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game With Mint".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // At this point, the CMint is already created and decompressed ("hot")
        // by the #[light_instruction] macro's pre_init phase.
        // Now we can use it to create CToken accounts and mint tokens.

        // 1. Create program-owned CToken vault (like cp-swap's token vaults)
        // Pattern: owner = vault_authority PDA, compress_to_account_pubkey derived from signer seeds
        // This ensures compressed TokenData.owner = vault address (not authority)
        let cmint_key = ctx.accounts.cmint.key();
        CreateCTokenAccountCpi::new_v2_signed(
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.vault_authority.key(), // Authority owns vault (like cp-swap)
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
            &[
                crate::instruction_accounts::VAULT_SEED,
                cmint_key.as_ref(),
                &[params.vault_bump],
            ],
        )?;

        // 2. Create user1's ATA
        // Args: owner, mint, payer, ata, bump, system_program, compressible_config, rent_sponsor
        CreateAssociatedCTokenAccountCpi::new_v2_idempotent(
            ctx.accounts.user1.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.user1_ata.to_account_info(),
            params.user1_ata_bump,
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        )?;

        // 3. Create user2's ATA
        CreateAssociatedCTokenAccountCpi::new_v2_idempotent(
            ctx.accounts.user2.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.user2_ata.to_account_info(),
            params.user2_ata_bump,
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        )?;

        // 4. Mint tokens to vault
        if params.vault_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                amount: params.vault_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // 5. Mint tokens to user1's ATA
        if params.user1_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user1_ata.to_account_info(),
                amount: params.user1_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // 6. Mint tokens to user2's ATA
        if params.user2_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user2_ata.to_account_info(),
                amount: params.user2_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // The #[light_instruction] macro handles:
        // - pre_init(): Created+Decompressed CMint (already done by now)
        // - finalize(): no-op (all work done above and in pre_init)
        //
        // After this instruction:
        // - UserRecord and GameSession PDAs have compressed addresses registered
        // - LP mint is created AND decompressed (hot/active state)
        // - Vault exists with vault_mint_amount tokens, owned by vault_authority
        // - User1 ATA exists with user1_ata_mint_amount tokens
        // - User2 ATA exists with user2_ata_mint_amount tokens
        Ok(())
    }

    /// Decompress compressed mints via CPI to ctoken program.
    ///
    /// At most 1 mint allowed (validated client-side).
    /// Works for both prove_by_index=true and prove_by_index=false.
    ///
    /// Remaining accounts (in order, starting at system_accounts_offset):
    /// - ctoken_program (required for CPI)
    /// - light_system_program
    /// - cpi_authority_pda (ctoken's CPI authority)
    /// - registered_program_pda  
    /// remaining_accounts: packed accounts (indices reference into this).
    /// System accounts at 0-5, then tree accounts, mint accounts.
    pub fn decompress_cmints<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressCMints<'info>>,
        params: DecompressCMintsParams,
    ) -> Result<()> {
        use crate::instruction_accounts::PackedMintVariant;
        use light_ctoken_interface::instructions::mint_action::{
            CompressedMintInstructionData, CompressedMintWithContext,
        };
        use light_ctoken_interface::state::mint::CompressedMintMetadata;
        use light_ctoken_sdk::ctoken::{find_cmint_address, DecompressCMint, SystemAccountInfos};

        let remaining = ctx.remaining_accounts;
        let offset = params.system_accounts_offset as usize;

        // Find max index referenced to validate remaining_accounts length
        let mut max_index: u8 = 5; // minimum: system accounts at 0-5
        for cmint_account_data in params.compressed_accounts.iter() {
            let PackedMintVariant::Standard(packed) = &cmint_account_data.data;
            // Note: compressed_address is raw data, not an account index
            max_index = max_index
                .max(packed.mint_seed_index)
                .max(packed.cmint_pda_index);
            if packed.has_mint_authority {
                max_index = max_index.max(packed.mint_authority_index);
            }
            if packed.has_freeze_authority {
                max_index = max_index.max(packed.freeze_authority_index);
            }
            // Tree indices from meta
            max_index = max_index
                .max(cmint_account_data.meta.tree_info.merkle_tree_pubkey_index)
                .max(cmint_account_data.meta.tree_info.queue_pubkey_index)
                .max(cmint_account_data.meta.output_state_tree_index);
        }

        let min_accounts = offset + (max_index as usize) + 1;
        if remaining.len() < min_accounts {
            return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
        }

        // System accounts at fixed indices 0-5
        // Note: ctoken_program at offset+0 is in remaining_accounts but NOT passed to CPI
        // (invoke() gets program_id from the instruction itself)
        let light_system_program = &remaining[offset + 1];
        let cpi_authority_pda = &remaining[offset + 2];
        let registered_program_pda = &remaining[offset + 3];
        let account_compression_authority = &remaining[offset + 4];
        let account_compression_program = &remaining[offset + 5];

        let system_accounts = SystemAccountInfos {
            light_system_program: light_system_program.clone(),
            cpi_authority_pda: cpi_authority_pda.clone(),
            registered_program_pda: registered_program_pda.clone(),
            account_compression_authority: account_compression_authority.clone(),
            account_compression_program: account_compression_program.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        // Process each compressed mint - UNPACK indices to pubkeys
        for cmint_account_data in params.compressed_accounts.iter() {
            let PackedMintVariant::Standard(packed) = &cmint_account_data.data;

            // Unpack indices to AccountInfos
            let mint_seed = &remaining[offset + packed.mint_seed_index as usize];
            let cmint = &remaining[offset + packed.cmint_pda_index as usize];

            // Tree accounts from meta
            let state_tree_idx =
                cmint_account_data.meta.tree_info.merkle_tree_pubkey_index as usize;
            let input_queue_idx = cmint_account_data.meta.tree_info.queue_pubkey_index as usize;
            let output_queue_idx = cmint_account_data.meta.output_state_tree_index as usize;
            let state_tree = &remaining[offset + state_tree_idx];
            let input_queue = &remaining[offset + input_queue_idx];
            let output_queue = &remaining[offset + output_queue_idx];

            // Unpack pubkeys
            let mint_seed_pubkey = *mint_seed.key;
            let cmint_pda_pubkey = *cmint.key;
            // compressed_address is raw data, not an account
            let compressed_address: [u8; 32] = packed.compressed_address;

            // Unpack optional authorities
            let mint_authority = if packed.has_mint_authority {
                Some(*remaining[offset + packed.mint_authority_index as usize].key)
            } else {
                None
            };
            let freeze_authority = if packed.has_freeze_authority {
                Some(*remaining[offset + packed.freeze_authority_index as usize].key)
            } else {
                None
            };

            // Verify cmint PDA derivation
            let (expected_cmint, _) = find_cmint_address(&mint_seed_pubkey);
            if cmint_pda_pubkey != expected_cmint {
                return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
            }

            // Reconstruct CompressedMintWithContext from packed data
            // Note: Some fields require .into() for Pubkey type conversion
            let compressed_mint_with_context = CompressedMintWithContext {
                leaf_index: packed.leaf_index,
                prove_by_index: packed.prove_by_index,
                root_index: packed.root_index,
                address: compressed_address,
                mint: Some(CompressedMintInstructionData {
                    supply: packed.supply,
                    decimals: packed.decimals,
                    metadata: CompressedMintMetadata {
                        version: packed.version,
                        cmint_decompressed: packed.cmint_decompressed,
                        mint: cmint_pda_pubkey.into(),
                        compressed_address,
                    },
                    mint_authority: mint_authority.map(|p| p.into()),
                    freeze_authority: freeze_authority.map(|p| p.into()),
                    extensions: packed.extensions.clone(),
                }),
            };

            // Build the DecompressCMint instruction
            let instruction = DecompressCMint {
                mint_seed_pubkey,
                payer: ctx.accounts.fee_payer.key(),
                authority: ctx.accounts.authority.key(),
                state_tree: *state_tree.key,
                input_queue: *input_queue.key,
                output_queue: *output_queue.key,
                compressed_mint_with_context,
                proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(
                    params.proof.0,
                ),
                rent_payment: packed.rent_payment,
                write_top_up: packed.write_top_up,
            }
            .instruction()
            .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

            // Build account infos for CPI
            // NOTE: Do NOT include ctoken_program here - invoke() gets it from instruction.program_id
            // Account order must match MintActionMetaConfig::to_account_metas() exactly:
            // 1. light_system_program, 2. mint_signer, 3. authority, 4. compressible_config,
            // 5. cmint, 6. rent_sponsor, 7. fee_payer, 8. cpi_authority, 9. registered_program,
            // 10. acc_compression_authority, 11. acc_compression_program, 12. system_program,
            // 13. output_queue, 14. state_tree, 15. input_queue
            let account_infos = vec![
                system_accounts.light_system_program.clone(),
                mint_seed.clone(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.ctoken_compressible_config.to_account_info(),
                cmint.clone(),
                ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                ctx.accounts.fee_payer.to_account_info(),
                system_accounts.cpi_authority_pda.clone(),
                system_accounts.registered_program_pda.clone(),
                system_accounts.account_compression_authority.clone(),
                system_accounts.account_compression_program.clone(),
                system_accounts.system_program.clone(),
                output_queue.clone(),
                state_tree.clone(),
                input_queue.clone(),
            ];

            anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;
        }

        Ok(())
    }

    /// Decompress compressed ATAs via CPI to ctoken program.
    ///
    /// Key difference from CMints: ATAs CAN be batched in ONE CPI call.
    /// All ATAs are decompressed using `decompress_full_ctoken_accounts_with_indices`.
    ///
    /// Remaining accounts (in order, starting at system_accounts_offset):
    /// - ctoken_program
    /// - light_system_program
    /// - ctoken_cpi_authority
    /// - registered_program_pda
    /// - account_compression_authority
    /// - account_compression_program
    /// - state_tree
    /// - input_queue
    /// - output_queue
    /// - For each ATA: [wallet (signer), mint, ata_pubkey]
    pub fn decompress_atas<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAtas<'info>>,
        params: DecompressAtasParams,
    ) -> Result<()> {
        use crate::instruction_accounts::PackedAtaVariant;
        use light_compressed_account::compressed_account::PackedMerkleContext;
        use light_ctoken_interface::instructions::{
            extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
            transfer2::MultiInputTokenDataWithContext,
        };
        use light_ctoken_sdk::{
            compressed_token::decompress_full::{
                decompress_full_ctoken_accounts_with_indices, DecompressFullIndices,
            },
            ctoken::{derive_ctoken_ata, CompressibleParamsCpi, CreateAssociatedCTokenAccountCpi},
        };

        let remaining = ctx.remaining_accounts;
        let offset = params.system_accounts_offset as usize;

        // remaining_accounts layout:
        // [0] ctoken_program - needed for CPI invoke
        // [1-5] system accounts
        // [6+] packed_accounts (arbitrary order, referenced by indices in params)
        //
        // Minimum: 6 (ctoken + system) + at least some packed_accounts
        // The actual required accounts depend on the max index used in params
        if remaining.len() < offset + 6 {
            return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
        }

        // ctoken_program is at offset for CPI
        let ctoken_program = &remaining[offset];

        // Packed accounts start after ctoken_program + 5 system accounts
        let packed_accounts_start = offset + 6;
        let packed_accounts = &remaining[packed_accounts_start..];

        // Build DecompressFullIndices for ALL ATAs
        let mut decompress_indices: Vec<DecompressFullIndices> =
            Vec::with_capacity(params.compressed_accounts.len());

        // Create ATAs and build indices - UNPACK indices to get pubkeys
        for (i, ata_account_data) in params.compressed_accounts.iter().enumerate() {
            let PackedAtaVariant::Standard(packed_data) = &ata_account_data.data;

            // UNPACK: get pubkeys from indices (same pattern as ctoken's Unpack trait)
            let wallet_idx = packed_data.wallet_index as usize;
            let mint_idx = packed_data.mint_index as usize;
            let ata_idx = packed_data.ata_index as usize;

            // Bounds check all indices
            let max_idx = wallet_idx.max(mint_idx).max(ata_idx);
            if max_idx >= packed_accounts.len() {
                return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
            }

            let wallet_account = &packed_accounts[wallet_idx];
            let mint_account = &packed_accounts[mint_idx];
            let ata_account = &packed_accounts[ata_idx];

            // UNPACK pubkeys from remaining_accounts using indices
            let wallet_pubkey = wallet_account.key;
            let mint_pubkey = mint_account.key;

            // Derive and verify ATA matches the account at ata_index
            let (expected_ata, bump) = derive_ctoken_ata(wallet_pubkey, mint_pubkey);
            if *ata_account.key != expected_ata {
                return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
            }

            // Create ATA on-chain (idempotent) with compression_only=true
            CreateAssociatedCTokenAccountCpi {
                owner: wallet_account.clone(),
                mint: mint_account.clone(),
                payer: ctx.accounts.fee_payer.to_account_info(),
                associated_token_account: ata_account.clone(),
                system_program: ctx.accounts.system_program.to_account_info(),
                bump,
                compressible: CompressibleParamsCpi::new_ata(
                    ctx.accounts.ctoken_compressible_config.to_account_info(),
                    ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ),
                idempotent: true,
            }
            .invoke()?;

            // Build CompressedOnly TLV extension for ATA
            // compression_index must be unique per input in the batch
            let tlv = vec![ExtensionInstructionData::CompressedOnly(
                CompressedOnlyExtensionInstructionData {
                    delegated_amount: 0,
                    withheld_transfer_fee: 0,
                    is_frozen: packed_data.is_frozen,
                    compression_index: i as u8,
                    is_ata: true,
                    bump,
                    owner_index: packed_data.wallet_index,
                },
            )];

            // Build source using packed indices directly
            let source = MultiInputTokenDataWithContext {
                owner: packed_data.ata_index, // ATA address index (compressed account's owner)
                amount: packed_data.amount,
                has_delegate: packed_data.has_delegate,
                delegate: packed_data.delegate_index,
                mint: packed_data.mint_index,
                version: 3, // ShaFlat version
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: ata_account_data
                        .meta
                        .tree_info
                        .merkle_tree_pubkey_index,
                    queue_pubkey_index: ata_account_data.meta.tree_info.queue_pubkey_index,
                    prove_by_index: ata_account_data.meta.tree_info.prove_by_index,
                    leaf_index: ata_account_data.meta.tree_info.leaf_index,
                },
                root_index: ata_account_data.meta.tree_info.root_index,
            };

            decompress_indices.push(DecompressFullIndices {
                source,
                destination_index: packed_data.ata_index,
                tlv: Some(tlv),
                is_ata: true,
            });
        }

        // Build ONE instruction to decompress ALL ATAs
        // Pass only packed_accounts (starting after system accounts)
        let instruction = decompress_full_ctoken_accounts_with_indices(
            ctx.accounts.fee_payer.key(),
            light_compressed_account::instruction_data::compressed_proof::ValidityProof(
                params.proof.0,
            ),
            None, // No CPI context for direct execution
            &decompress_indices,
            &remaining[packed_accounts_start..], // Only packed_accounts
        )
        .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

        // Build account infos for CPI - invoke matches by pubkey
        // Include:
        // - ctoken_program (for invoke to find the program)
        // - fee_payer and system_program from named accounts
        // - remaining_accounts (system accounts + packed_accounts)
        let mut account_infos: Vec<anchor_lang::prelude::AccountInfo<'info>> =
            Vec::with_capacity(remaining.len() + 3);
        account_infos.push(ctoken_program.clone());
        account_infos.push(ctx.accounts.fee_payer.to_account_info());
        account_infos.push(ctx.accounts.system_program.to_account_info());
        // Skip ctoken_program (already added), include system accounts + packed_accounts
        for acc in remaining[offset + 1..].iter() {
            account_infos.push(acc.clone());
        }

        anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;

        Ok(())
    }

    /// Unified decompression for ATAs and CMints.
    /// - Any number of ATAs allowed
    /// - At most 1 CMint allowed (error if >1)
    /// - Uses CPI context when mixing types
    ///
    /// NOTE: For cPDA + cToken decompression, use decompress_accounts_idempotent.
    /// cPDAs can write to CPI context (no "compressions" restriction), so:
    /// - cPDA + Mint: WORKS via decompress_accounts_idempotent
    /// - cPDA + ATA: WORKS via decompress_accounts_idempotent
    /// - Mint + ATA: FAILS (both modify on-chain state)
    pub fn decompress_unified<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressUnified<'info>>,
        params: DecompressUnifiedParams,
    ) -> Result<()> {
        use crate::instruction_accounts::DecompressVariant;
        use light_ctoken_sdk::ctoken::SystemAccountInfos;

        let remaining = ctx.remaining_accounts;
        let offset = params.system_accounts_offset as usize;

        // Separate accounts by type
        let mut ata_accounts: Vec<(
            &crate::instruction_accounts::PackedAtaTokenData,
            &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        )> = Vec::new();
        let mut mint_account: Option<(
            &crate::instruction_accounts::PackedMintTokenData,
            &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        )> = None;

        for account in params.compressed_accounts.iter() {
            match &account.data {
                DecompressVariant::Ata(data) => {
                    ata_accounts.push((data, &account.meta));
                }
                DecompressVariant::Mint(data) => {
                    if mint_account.is_some() {
                        // Error: at most 1 mint allowed
                        return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
                    }
                    mint_account = Some((data, &account.meta));
                }
            }
        }

        let has_atas = !ata_accounts.is_empty();
        let has_mint = mint_account.is_some();

        if !has_atas && !has_mint {
            return Ok(()); // Nothing to do
        }

        // System accounts at fixed indices 0-5
        let light_system_program = &remaining[offset + 1];
        let cpi_authority_pda = &remaining[offset + 2];
        let registered_program_pda = &remaining[offset + 3];
        let account_compression_authority = &remaining[offset + 4];
        let account_compression_program = &remaining[offset + 5];

        let system_accounts = SystemAccountInfos {
            light_system_program: light_system_program.clone(),
            cpi_authority_pda: cpi_authority_pda.clone(),
            registered_program_pda: registered_program_pda.clone(),
            account_compression_authority: account_compression_authority.clone(),
            account_compression_program: account_compression_program.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        // Determine execution mode based on what we have
        let needs_cpi_context = has_mint && has_atas;

        // CPI context is at index 6 if needed
        let cpi_context_idx = if needs_cpi_context {
            Some(offset + 6)
        } else {
            None
        };
        let packed_accounts_start = if needs_cpi_context {
            offset + 7
        } else {
            offset + 6
        };

        // CASE 1: Only ATAs - direct execution (reuse existing logic)
        if has_atas && !has_mint {
            return decompress_atas_only(
                &ctx,
                &params,
                &ata_accounts,
                offset,
                packed_accounts_start,
                remaining,
            );
        }

        // CASE 2: Only Mint - direct execution (no CPI context)
        if has_mint && !has_atas {
            let (packed, meta) = mint_account.unwrap();
            return decompress_mint_direct(
                &ctx,
                &params.proof,
                packed,
                meta,
                offset,
                &system_accounts,
                remaining,
            );
        }

        // CASE 3: Mint + ATAs - use CPI context
        // NOTE: This case is NOT SUPPORTED because:
        // - DecompressMint cannot write to CPI context (error 6035)
        // - Transfer2 with compressions cannot write to CPI context (error 18001)
        // Both operations modify on-chain state, so neither can be in write mode.
        let cpi_context_account = &remaining[cpi_context_idx.unwrap()];

        // First: mint writes to CPI context (will fail with error 6035)
        {
            let (packed, meta) = mint_account.unwrap();
            decompress_mint_with_context(
                &ctx,
                &params.proof,
                packed,
                meta,
                offset,
                &system_accounts,
                remaining,
                cpi_context_account,
                true, // has_atas_after = true (write mode - will fail)
            )?;
        }

        // Then: ATAs execute and consume CPI context (never reached)
        decompress_atas_with_context(
            &ctx,
            &params,
            &ata_accounts,
            offset,
            packed_accounts_start,
            remaining,
            cpi_context_account,
            false, // execute mode
        )
    }
}

// Helper functions for decompress_unified (outside the program module)

fn decompress_atas_only<'info>(
    ctx: &Context<'_, '_, '_, 'info, DecompressUnified<'info>>,
    params: &DecompressUnifiedParams,
    ata_accounts: &[(
        &crate::instruction_accounts::PackedAtaTokenData,
        &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    )],
    offset: usize,
    packed_accounts_start: usize,
    remaining: &[anchor_lang::prelude::AccountInfo<'info>],
) -> anchor_lang::Result<()> {
    use light_compressed_account::compressed_account::PackedMerkleContext;
    use light_ctoken_interface::instructions::extensions::{
        CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
    };
    use light_ctoken_interface::instructions::transfer2::MultiInputTokenDataWithContext;
    use light_ctoken_sdk::compressed_token::decompress_full::{
        decompress_full_ctoken_accounts_with_indices, DecompressFullIndices,
    };
    use light_ctoken_sdk::ctoken::{
        derive_ctoken_ata, CompressibleParamsCpi, CreateAssociatedCTokenAccountCpi,
    };

    let ctoken_program = &remaining[offset];
    // packed_accounts excludes system accounts - instruction builder will add them
    let packed_accounts = &remaining[packed_accounts_start..];

    let mut decompress_indices = Vec::with_capacity(ata_accounts.len());

    for (i, (packed_data, meta)) in ata_accounts.iter().enumerate() {
        // Absolute indices for local account access
        let wallet_idx = packed_data.wallet_index as usize;
        let mint_idx = packed_data.mint_index as usize;
        let ata_idx = packed_data.ata_index as usize;

        let max_idx = wallet_idx.max(mint_idx).max(ata_idx);
        if max_idx >= remaining.len() {
            return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
        }

        let wallet_account = &remaining[wallet_idx];
        let mint_account = &remaining[mint_idx];
        let ata_account = &remaining[ata_idx];

        // Derive ATA
        let (expected_ata, bump) = derive_ctoken_ata(wallet_account.key, mint_account.key);
        if *ata_account.key != expected_ata {
            return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
        }

        // Create ATA on-chain (idempotent)
        CreateAssociatedCTokenAccountCpi {
            owner: wallet_account.clone(),
            mint: mint_account.clone(),
            payer: ctx.accounts.fee_payer.to_account_info(),
            associated_token_account: ata_account.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
            bump,
            compressible: CompressibleParamsCpi::new_ata(
                ctx.accounts.ctoken_compressible_config.to_account_info(),
                ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ),
            idempotent: true,
        }
        .invoke()?;

        // Convert to relative indices for CPI (relative to packed_accounts_start)
        let rel_wallet_idx = (wallet_idx - packed_accounts_start) as u8;
        let rel_mint_idx = (mint_idx - packed_accounts_start) as u8;
        let rel_ata_idx = (ata_idx - packed_accounts_start) as u8;
        let rel_delegate_idx = if packed_data.has_delegate {
            (packed_data.delegate_index as usize - packed_accounts_start) as u8
        } else {
            0
        };
        // Tree indices are also absolute - convert them
        let rel_tree_idx = meta.tree_info.merkle_tree_pubkey_index - packed_accounts_start as u8;
        let rel_queue_idx = meta.tree_info.queue_pubkey_index - packed_accounts_start as u8;

        // Build TLV extension (uses relative indices for CPI)
        let tlv = vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: packed_data.is_frozen,
                compression_index: i as u8,
                is_ata: true,
                bump,
                owner_index: rel_wallet_idx,
            },
        )];

        // Build source data (uses relative indices for CPI)
        let source = MultiInputTokenDataWithContext {
            owner: rel_ata_idx, // ATA address index (compress_only)
            amount: packed_data.amount,
            has_delegate: packed_data.has_delegate,
            delegate: rel_delegate_idx,
            mint: rel_mint_idx,
            version: 3,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rel_tree_idx,
                queue_pubkey_index: rel_queue_idx,
                prove_by_index: meta.tree_info.prove_by_index,
                leaf_index: meta.tree_info.leaf_index,
            },
            root_index: meta.tree_info.root_index,
        };

        decompress_indices.push(DecompressFullIndices {
            source,
            destination_index: rel_ata_idx,
            tlv: Some(tlv),
            is_ata: true,
        });
    }

    // Build instruction - instruction builder adds system accounts, so pass only packed_accounts
    let instruction = decompress_full_ctoken_accounts_with_indices(
        ctx.accounts.fee_payer.key(),
        light_compressed_account::instruction_data::compressed_proof::ValidityProof(params.proof.0),
        None,
        &decompress_indices,
        packed_accounts,
    )
    .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

    // Build account infos for CPI - need all accounts including system accounts
    let mut account_infos: Vec<anchor_lang::prelude::AccountInfo<'info>> =
        Vec::with_capacity(remaining.len() + 3);
    account_infos.push(ctoken_program.clone());
    account_infos.push(ctx.accounts.fee_payer.to_account_info());
    account_infos.push(ctx.accounts.system_program.to_account_info());
    // Add all remaining accounts (including system accounts) for CPI to find them
    for acc in remaining.iter() {
        account_infos.push(acc.clone());
    }

    anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;
    Ok(())
}

fn decompress_mint_direct<'info>(
    ctx: &Context<'_, '_, '_, 'info, DecompressUnified<'info>>,
    proof: &light_sdk::instruction::ValidityProof,
    packed: &crate::instruction_accounts::PackedMintTokenData,
    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    offset: usize,
    system_accounts: &light_ctoken_sdk::ctoken::SystemAccountInfos<'info>,
    remaining: &[anchor_lang::prelude::AccountInfo<'info>],
) -> anchor_lang::Result<()> {
    use light_ctoken_interface::instructions::mint_action::{
        CompressedMintInstructionData, CompressedMintWithContext,
    };
    use light_ctoken_interface::state::mint::CompressedMintMetadata;
    use light_ctoken_sdk::ctoken::{find_cmint_address, DecompressCMint};

    let mint_seed = &remaining[offset + packed.mint_seed_index as usize];
    let cmint = &remaining[offset + packed.cmint_pda_index as usize];
    let state_tree_idx = meta.tree_info.merkle_tree_pubkey_index as usize;
    let input_queue_idx = meta.tree_info.queue_pubkey_index as usize;
    let output_queue_idx = meta.output_state_tree_index as usize;
    let state_tree = &remaining[offset + state_tree_idx];
    let input_queue = &remaining[offset + input_queue_idx];
    let output_queue = &remaining[offset + output_queue_idx];

    let mint_seed_pubkey = *mint_seed.key;
    let cmint_pda_pubkey = *cmint.key;
    let compressed_address = packed.compressed_address;

    // Verify CMint PDA
    let (expected_cmint, _) = find_cmint_address(&mint_seed_pubkey);
    if cmint_pda_pubkey != expected_cmint {
        return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
    }

    // Unpack authorities
    let mint_authority = if packed.has_mint_authority {
        Some(*remaining[offset + packed.mint_authority_index as usize].key)
    } else {
        None
    };
    let freeze_authority = if packed.has_freeze_authority {
        Some(*remaining[offset + packed.freeze_authority_index as usize].key)
    } else {
        None
    };

    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: packed.leaf_index,
        prove_by_index: packed.prove_by_index,
        root_index: packed.root_index,
        address: compressed_address,
        mint: Some(CompressedMintInstructionData {
            supply: packed.supply,
            decimals: packed.decimals,
            metadata: CompressedMintMetadata {
                version: packed.version,
                cmint_decompressed: packed.cmint_decompressed,
                mint: cmint_pda_pubkey.into(),
                compressed_address,
            },
            mint_authority: mint_authority.map(|p| p.into()),
            freeze_authority: freeze_authority.map(|p| p.into()),
            extensions: packed.extensions.clone(),
        }),
    };

    let instruction = DecompressCMint {
        mint_seed_pubkey,
        payer: ctx.accounts.fee_payer.key(),
        authority: ctx.accounts.authority.key(),
        state_tree: *state_tree.key,
        input_queue: *input_queue.key,
        output_queue: *output_queue.key,
        compressed_mint_with_context,
        proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(proof.0),
        rent_payment: packed.rent_payment,
        write_top_up: packed.write_top_up,
    }
    .instruction()
    .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

    let account_infos = vec![
        system_accounts.light_system_program.clone(),
        mint_seed.clone(),
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.ctoken_compressible_config.to_account_info(),
        cmint.clone(),
        ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        ctx.accounts.fee_payer.to_account_info(),
        system_accounts.cpi_authority_pda.clone(),
        system_accounts.registered_program_pda.clone(),
        system_accounts.account_compression_authority.clone(),
        system_accounts.account_compression_program.clone(),
        system_accounts.system_program.clone(),
        output_queue.clone(),
        state_tree.clone(),
        input_queue.clone(),
    ];

    anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;
    Ok(())
}

fn decompress_mint_with_context<'info>(
    ctx: &Context<'_, '_, '_, 'info, DecompressUnified<'info>>,
    proof: &light_sdk::instruction::ValidityProof,
    packed: &crate::instruction_accounts::PackedMintTokenData,
    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    offset: usize,
    system_accounts: &light_ctoken_sdk::ctoken::SystemAccountInfos<'info>,
    remaining: &[anchor_lang::prelude::AccountInfo<'info>],
    cpi_context_account: &anchor_lang::prelude::AccountInfo<'info>,
    has_atas_after: bool,
) -> anchor_lang::Result<()> {
    use light_ctoken_interface::instructions::mint_action::{
        CompressedMintInstructionData, CompressedMintWithContext,
    };
    use light_ctoken_interface::state::mint::CompressedMintMetadata;
    use light_ctoken_sdk::ctoken::{
        create_decompress_mint_cpi_context_execute, create_decompress_mint_cpi_context_first,
        find_cmint_address, DecompressCMintCpiWithContext, CTOKEN_CPI_AUTHORITY,
    };

    let mint_seed = &remaining[offset + packed.mint_seed_index as usize];
    let cmint = &remaining[offset + packed.cmint_pda_index as usize];
    let state_tree_idx = meta.tree_info.merkle_tree_pubkey_index as usize;
    let input_queue_idx = meta.tree_info.queue_pubkey_index as usize;
    let output_queue_idx = meta.output_state_tree_index as usize;
    let state_tree = &remaining[offset + state_tree_idx];
    let input_queue = &remaining[offset + input_queue_idx];
    let output_queue = &remaining[offset + output_queue_idx];

    let mint_seed_pubkey = *mint_seed.key;
    let cmint_pda_pubkey = *cmint.key;
    let compressed_address = packed.compressed_address;

    // Verify CMint PDA
    let (expected_cmint, _) = find_cmint_address(&mint_seed_pubkey);
    if cmint_pda_pubkey != expected_cmint {
        return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
    }

    // Unpack authorities
    let mint_authority = if packed.has_mint_authority {
        Some(*remaining[offset + packed.mint_authority_index as usize].key)
    } else {
        None
    };
    let freeze_authority = if packed.has_freeze_authority {
        Some(*remaining[offset + packed.freeze_authority_index as usize].key)
    } else {
        None
    };

    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: packed.leaf_index,
        prove_by_index: packed.prove_by_index,
        root_index: packed.root_index,
        address: compressed_address,
        mint: Some(CompressedMintInstructionData {
            supply: packed.supply,
            decimals: packed.decimals,
            metadata: CompressedMintMetadata {
                version: packed.version,
                cmint_decompressed: packed.cmint_decompressed,
                mint: cmint_pda_pubkey.into(),
                compressed_address,
            },
            mint_authority: mint_authority.map(|p| p.into()),
            freeze_authority: freeze_authority.map(|p| p.into()),
            extensions: packed.extensions.clone(),
        }),
    };

    // CPI context: write first if ATAs after, else execute
    let cpi_ctx = if has_atas_after {
        create_decompress_mint_cpi_context_first([0; 32], 0, 0)
    } else {
        create_decompress_mint_cpi_context_execute([0; 32], 0, 0)
    };

    // Find ctoken CPI authority in remaining accounts
    let ctoken_cpi_authority = remaining
        .iter()
        .find(|a| *a.key == CTOKEN_CPI_AUTHORITY)
        .cloned()
        .ok_or(anchor_lang::error::ErrorCode::AccountNotEnoughKeys)?;

    DecompressCMintCpiWithContext {
        mint_seed: mint_seed.clone(),
        authority: ctx.accounts.authority.to_account_info(),
        payer: ctx.accounts.fee_payer.to_account_info(),
        cmint: cmint.clone(),
        compressible_config: ctx.accounts.ctoken_compressible_config.clone(),
        rent_sponsor: ctx.accounts.ctoken_rent_sponsor.clone(),
        state_tree: state_tree.clone(),
        input_queue: input_queue.clone(),
        output_queue: output_queue.clone(),
        cpi_context_account: cpi_context_account.clone(),
        system_accounts: system_accounts.clone(),
        ctoken_cpi_authority,
        compressed_mint_with_context,
        proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(proof.0),
        rent_payment: packed.rent_payment,
        write_top_up: packed.write_top_up,
        cpi_context: cpi_ctx,
    }
    .invoke()?;

    Ok(())
}

fn decompress_atas_with_context<'info>(
    ctx: &Context<'_, '_, '_, 'info, DecompressUnified<'info>>,
    params: &DecompressUnifiedParams,
    ata_accounts: &[(
        &crate::instruction_accounts::PackedAtaTokenData,
        &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    )],
    offset: usize,
    packed_accounts_start: usize,
    remaining: &[anchor_lang::prelude::AccountInfo<'info>],
    cpi_context_account: &anchor_lang::prelude::AccountInfo<'info>,
    _execute_mode: bool,
) -> anchor_lang::Result<()> {
    use light_compressed_account::compressed_account::PackedMerkleContext;
    use light_ctoken_interface::instructions::extensions::{
        CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
    };
    use light_ctoken_interface::instructions::transfer2::MultiInputTokenDataWithContext;
    use light_ctoken_sdk::compressed_token::decompress_full::{
        decompress_full_ctoken_accounts_with_indices, DecompressFullIndices,
    };
    use light_ctoken_sdk::ctoken::{
        derive_ctoken_ata, CompressibleParamsCpi, CreateAssociatedCTokenAccountCpi,
    };

    let ctoken_program = &remaining[offset];
    let packed_accounts = &remaining[packed_accounts_start..];

    let mut decompress_indices = Vec::with_capacity(ata_accounts.len());

    for (i, (packed_data, meta)) in ata_accounts.iter().enumerate() {
        let wallet_idx = packed_data.wallet_index as usize;
        let mint_idx = packed_data.mint_index as usize;
        let ata_idx = meta.output_state_tree_index as usize;

        let max_idx = wallet_idx.max(mint_idx).max(ata_idx);
        if max_idx >= packed_accounts.len() {
            return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
        }

        let wallet_account = &packed_accounts[wallet_idx];
        let mint_account = &packed_accounts[mint_idx];
        let ata_account = &packed_accounts[ata_idx];

        // Derive ATA
        let (expected_ata, bump) = derive_ctoken_ata(wallet_account.key, mint_account.key);
        if *ata_account.key != expected_ata {
            return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
        }

        // Create ATA on-chain (idempotent)
        CreateAssociatedCTokenAccountCpi {
            owner: wallet_account.clone(),
            mint: mint_account.clone(),
            payer: ctx.accounts.fee_payer.to_account_info(),
            associated_token_account: ata_account.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
            bump,
            compressible: CompressibleParamsCpi::new_ata(
                ctx.accounts.ctoken_compressible_config.to_account_info(),
                ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ),
            idempotent: true,
        }
        .invoke()?;

        // Build TLV extension
        let tlv = vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: packed_data.is_frozen,
                compression_index: i as u8,
                is_ata: true,
                bump,
                owner_index: packed_data.wallet_index,
            },
        )];

        let source = MultiInputTokenDataWithContext {
            owner: ata_idx as u8,
            amount: packed_data.amount,
            has_delegate: packed_data.has_delegate,
            delegate: packed_data.delegate_index,
            mint: packed_data.mint_index,
            version: 3,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: meta.tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: meta.tree_info.queue_pubkey_index,
                prove_by_index: meta.tree_info.prove_by_index,
                leaf_index: meta.tree_info.leaf_index,
            },
            root_index: meta.tree_info.root_index,
        };

        decompress_indices.push(DecompressFullIndices {
            source,
            destination_index: ata_idx as u8,
            tlv: Some(tlv),
            is_ata: true,
        });
    }

    // Pass CPI context pubkey to consume context written by mint
    let instruction = decompress_full_ctoken_accounts_with_indices(
        ctx.accounts.fee_payer.key(),
        light_compressed_account::instruction_data::compressed_proof::ValidityProof(params.proof.0),
        Some(*cpi_context_account.key), // Consume CPI context
        &decompress_indices,
        packed_accounts,
    )
    .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

    let mut account_infos: Vec<anchor_lang::prelude::AccountInfo<'info>> =
        Vec::with_capacity(remaining.len() + 3);
    account_infos.push(ctoken_program.clone());
    account_infos.push(ctx.accounts.fee_payer.to_account_info());
    account_infos.push(ctx.accounts.system_program.to_account_info());
    for acc in remaining[offset + 1..].iter() {
        account_infos.push(acc.clone());
    }

    anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;
    Ok(())
}
