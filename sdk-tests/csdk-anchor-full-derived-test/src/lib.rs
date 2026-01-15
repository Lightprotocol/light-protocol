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
    // User-owned ATA: uses ctoken's standard ATA derivation (wallet + ctoken_program + mint)
    // is_ata flag indicates the wallet signs (not the program)
    UserAta = (is_token, is_ata, ctx.wallet, ctx.cmint),
    // CMint: for decompressing a light mint
    CMint = (is_token, "cmint", ctx.mint_signer, authority = LIGHT_CPI_SIGNER),
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
    /// - account_compression_authority
    /// - account_compression_program
    /// - state_tree
    /// - input_queue
    /// - output_queue
    /// - For each mint: [mint_signer_pda, cmint_pda]
    pub fn decompress_cmints<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressCMints<'info>>,
        params: DecompressCMintsParams,
    ) -> Result<()> {
        use crate::instruction_accounts::CompressedMintVariant;
        use light_ctoken_sdk::ctoken::{find_cmint_address, DecompressCMint, SystemAccountInfos};

        let remaining = ctx.remaining_accounts;
        let offset = params.system_accounts_offset as usize;

        // Validate we have enough remaining accounts
        // At minimum: ctoken_program, light_system_program, cpi_authority, registered_program,
        // account_compression_authority, account_compression_program,
        // state_tree, input_queue, output_queue = 9 base accounts
        // Plus 2 per mint (mint_signer, cmint)
        let min_accounts = 9 + params.compressed_accounts.len() * 2;
        if remaining.len() < min_accounts {
            return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
        }

        // Parse accounts from remaining (starting at offset)
        let ctoken_program = &remaining[offset];
        let light_system_program = &remaining[offset + 1];
        let cpi_authority_pda = &remaining[offset + 2];
        let registered_program_pda = &remaining[offset + 3];
        let account_compression_authority = &remaining[offset + 4];
        let account_compression_program = &remaining[offset + 5];

        // Parse tree accounts
        let state_tree = &remaining[offset + 6];
        let input_queue = &remaining[offset + 7];
        let output_queue = &remaining[offset + 8];

        // Remaining accounts after system+trees: [mint_signer1, cmint1, ...]
        let mint_accounts_start = offset + 9;

        // Build system accounts struct for CPI
        let system_accounts = SystemAccountInfos {
            light_system_program: light_system_program.clone(),
            cpi_authority_pda: cpi_authority_pda.clone(),
            registered_program_pda: registered_program_pda.clone(),
            account_compression_authority: account_compression_authority.clone(),
            account_compression_program: account_compression_program.clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        // Process each compressed mint
        for (i, cmint_account_data) in params.compressed_accounts.iter().enumerate() {
            // Extract the actual mint data from the enum
            let CompressedMintVariant::Standard(mint_data) = &cmint_account_data.data;

            // Get mint_signer and cmint accounts for this mint
            let mint_signer = &remaining[mint_accounts_start + i * 2];
            let cmint = &remaining[mint_accounts_start + i * 2 + 1];

            // Verify mint_signer matches expected
            if *mint_signer.key != mint_data.mint_seed_pubkey {
                return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
            }

            // Verify cmint PDA matches expected derivation
            let (expected_cmint, _) = find_cmint_address(&mint_data.mint_seed_pubkey);
            if *cmint.key != expected_cmint {
                return Err(anchor_lang::error::ErrorCode::ConstraintRaw.into());
            }

            // Build the DecompressCMint instruction
            let instruction = DecompressCMint {
                mint_seed_pubkey: *mint_signer.key,
                payer: ctx.accounts.fee_payer.key(),
                authority: ctx.accounts.authority.key(),
                state_tree: *state_tree.key,
                input_queue: *input_queue.key,
                output_queue: *output_queue.key,
                compressed_mint_with_context: mint_data.compressed_mint_with_context.clone(),
                proof: light_compressed_account::instruction_data::compressed_proof::ValidityProof(
                    params.proof.0,
                ),
                rent_payment: mint_data.rent_payment,
                write_top_up: mint_data.write_top_up,
            }
            .instruction()
            .map_err(|_| anchor_lang::error::ErrorCode::InstructionFallbackNotFound)?;

            // Build account infos for CPI (must include ctoken program for invoke to work)
            let account_infos = vec![
                ctoken_program.clone(),
                system_accounts.light_system_program.clone(),
                mint_signer.clone(),
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
}
