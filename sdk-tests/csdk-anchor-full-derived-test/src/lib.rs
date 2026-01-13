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
    AccountCreationData, CompressionParams, E2eTestData, E2eTestParams, GameSession,
    NewStyleRecord, PackedGameSession, PackedNewStyleRecord, PackedPlaceholderRecord,
    PackedUserRecord, PlaceholderRecord, UserRecord,
};

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
    use anchor_lang::solana_program::{program::invoke, sysvar::clock::Clock};
    use light_compressed_account::instruction_data::traits::LightInstructionData;
    use light_ctoken_interface::instructions::mint_action::{
        MintActionCompressedInstructionData, MintToCompressedAction, Recipient,
    };
    use light_ctoken_sdk::compressed_token::{
        create_compressed_mint::find_cmint_address, mint_action::MintActionMetaConfig,
    };
    use light_sdk::{
        compressible::{
            compress_account_on_init::prepare_compressed_account_on_init, CompressibleConfig,
        },
        cpi::{
            v2::{CpiAccounts, LightSystemProgramCpi},
            InvokeLightSystemProgram, LightCpiInstruction,
        },
    };
    use light_sdk_types::{
        cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts,
    };

    use super::*;
    use crate::{
        errors::ErrorCode,
        instruction_accounts::{
            CreateMultiplePdas, CreatePdasAndMintAuto, CreateSimplePda,
            CreateUserRecordAndGameSessionAuto, FullAutoParams, FullAutoWithMintParams,
            MultiPdaParams, SimplePdaParams,
        },
        state::{GameSession, PlaceholderRecord, UserRecord},
        LIGHT_CPI_SIGNER,
    };

    // =========================================================================
    // APPROACH 2: Automatic Compression with LightFinalize (RECOMMENDED)
    // =========================================================================
    // These instructions use #[light_instruction] which auto-calls light_finalize
    // at the end. The compression happens automatically - no manual CPI code needed!

    /// Create a single PDA with automatic compression.
    /// The #[light_instruction] macro auto-calls light_pre_init at start
    /// and light_finalize at end - zero manual compression code!
    #[light_instruction(params)]
    pub fn create_simple_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateSimplePda<'info>>,
        params: SimplePdaParams,
    ) -> Result<()> {
        // Just populate the PDA data - compression is automatic!
        let pda = &mut ctx.accounts.my_pda;
        pda.owner = ctx.accounts.fee_payer.key();
        pda.metadata = "Created with LightFinalize".to_string();
        pda.version = 1;
        pda.flags = 0;

        // That's it! No manual compression code needed.
        // The #[light_instruction] macro handles:
        // 1. Calling light_pre_init() at start (for mints)
        // 2. Calling light_finalize() at end (compresses PDAs)
        Ok(())
    }

    /// Create multiple PDAs with automatic compression.
    /// Both user_record and game_session are compressed automatically.
    #[light_instruction(params)]
    pub fn create_multiple_pdas<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMultiplePdas<'info>>,
        params: MultiPdaParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;

        // Populate UserRecord
        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto User".to_string();
        user_record.score = 100;
        user_record.category_id = params.category_id;

        // Populate GameSession
        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // Both PDAs are compressed automatically at instruction end!
        Ok(())
    }

    /// FULL AUTOMATIC: Creates 2 PDAs in ONE instruction using LightFinalize.
    /// - 2 PDAs with #[compressible] (UserRecord, GameSession)
    /// All batched together with a single proof execution!
    #[light_instruction(params)]
    pub fn create_user_record_and_game_session_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSessionAuto<'info>>,
        params: FullAutoParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;

        // Populate UserRecord - compression handled by macro
        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto Created User".to_string();
        user_record.score = 0;
        user_record.category_id = params.category_id;

        // Populate GameSession - compression handled by macro
        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // That's it! The #[light_instruction] macro handles:
        // light_finalize() - compresses both PDAs with shared proof
        // No manual CPI code needed!
        Ok(())
    }

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

        // 2. Create user's ATA (like cp-swap's creator_lp_token)
        CreateAssociatedCTokenAccountCpi::new_v2_idempotent(
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.user_ata.to_account_info(),
            params.user_ata_bump,
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        )?;

        // 3. Mint tokens to vault
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

        // 4. Mint tokens to user's ATA
        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
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
        // - User ATA exists with user_ata_mint_amount tokens, owned by fee_payer
        Ok(())
    }

    // =========================================================================
    // APPROACH 1: Manual Compression (kept for comparison)
    // =========================================================================

    pub fn create_user_record_and_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
        account_data: AccountCreationData,
        compression_params: CompressionParams,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;
        let game_session = &mut ctx.accounts.game_session;

        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
            return Err(ErrorCode::RentRecipientMismatch.into());
        }

        // Populate UserRecord
        user_record.owner = account_data.owner;
        user_record.name = account_data.user_name.clone();
        user_record.score = 11;
        user_record.category_id = account_data.category_id;

        // Populate GameSession
        game_session.session_id = account_data.session_id;
        game_session.player = ctx.accounts.user.key();
        game_session.game_type = account_data.game_type.clone();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        let cpi_accounts = CpiAccounts::new_with_config(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
        );
        let cpi_context_pubkey = cpi_accounts.cpi_context().unwrap().key();
        let cpi_context_account = cpi_accounts.cpi_context().unwrap();

        let user_new_address_params = compression_params
            .user_address_tree_info
            .into_new_address_params_assigned_packed(user_record.key().to_bytes().into(), Some(0));
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_assigned_packed(game_session.key().to_bytes().into(), Some(1));

        let mut all_compressed_infos = Vec::new();

        let user_record_info = user_record.to_account_info();
        let user_record_data_mut = &mut **user_record;
        let user_compressed_info = prepare_compressed_account_on_init::<UserRecord>(
            &user_record_info,
            user_record_data_mut,
            &config,
            compression_params.user_compressed_address,
            user_new_address_params,
            compression_params.user_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            false, // with_data=false: only register address, data stays on-chain
        )?;
        all_compressed_infos.push(user_compressed_info);

        let game_session_info = game_session.to_account_info();
        let game_session_data_mut = &mut **game_session;
        let game_compressed_info = prepare_compressed_account_on_init::<GameSession>(
            &game_session_info,
            game_session_data_mut,
            &config,
            compression_params.game_compressed_address,
            game_new_address_params,
            compression_params.game_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            false, // with_data=false: only register address, data stays on-chain
        )?;
        all_compressed_infos.push(game_compressed_info);

        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority().unwrap(),
            cpi_context: cpi_context_account,
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, compression_params.proof)
            .with_new_addresses(&[user_new_address_params, game_new_address_params])
            .with_account_infos(&all_compressed_infos)
            .write_to_cpi_context_first()
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

        let mint = find_cmint_address(&ctx.accounts.mint_signer.key()).0;

        // Use the generated client seed function for CToken signer (generated by add_compressible_instructions macro)
        let (_, token_account_address) = get_ctokensigner_seeds(&ctx.accounts.user.key(), &mint);

        let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key;
        let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key;

        // Build instruction data using the correct API
        let proof = compression_params.proof.0.unwrap_or_default();
        let instruction_data = MintActionCompressedInstructionData::new_mint(
            0, // root_index for new addresses
            proof,
            compression_params.mint_with_context.mint.clone().unwrap(),
        )
        .with_mint_to_compressed(MintToCompressedAction {
            token_account_version: 3,
            recipients: vec![Recipient::new(token_account_address, 1000)],
        })
        .with_cpi_context(
            light_ctoken_interface::instructions::mint_action::CpiContext {
                address_tree_pubkey: address_tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false,
                in_tree_index: 1,
                in_queue_index: 0,
                out_queue_index: 0,
                token_out_queue_index: 0,
                assigned_account_index: 2,
                read_only_address_trees: [0; 4],
            },
        );

        // Build account metas
        let mut config = MintActionMetaConfig::new_create_mint(
            ctx.accounts.user.key(),           // fee_payer
            ctx.accounts.mint_authority.key(), // authority (mint authority)
            ctx.accounts.mint_signer.key(),    // mint_signer
            address_tree_pubkey,
            output_queue,
        )
        .with_mint_compressed_tokens();

        config.cpi_context = Some(cpi_context_pubkey);

        let account_metas = config.to_account_metas();

        // Serialize instruction data
        let data = instruction_data.data().map_err(ProgramError::from)?;

        // Build mint action instruction
        let mint_action_instruction = solana_program::instruction::Instruction {
            program_id: light_ctoken_interface::CTOKEN_PROGRAM_ID.into(),
            accounts: account_metas,
            data,
        };

        let mut account_infos = cpi_accounts.to_account_infos();
        account_infos.push(
            ctx.accounts
                .compress_token_program_cpi_authority
                .to_account_info(),
        );
        account_infos.push(ctx.accounts.ctoken_program.to_account_info());
        account_infos.push(ctx.accounts.mint_authority.to_account_info());
        account_infos.push(ctx.accounts.mint_signer.to_account_info());
        account_infos.push(ctx.accounts.user.to_account_info());

        invoke(&mint_action_instruction, &account_infos)?;

        user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;
        game_session.close(ctx.accounts.rent_sponsor.to_account_info())?;

        Ok(())
    }

    /// E2E test: Create ALL accounts atomically in ONE instruction (like cp-swap):
    /// 1. PlaceholderRecord PDA (compressed immediately)
    /// 2. UserRecord PDA (compressed immediately) - 2nd PDA for multi-PDA coverage
    /// 3. Light mint + decompress to CMint
    /// 4. Create CToken vault via CPI (like cp-swap token vaults)
    /// 5. Create User ATA via CPI (like cp-swap's creator_lp_token)
    /// 6. MintTo vault and user_ata from CMint
    pub fn e2e_create_mint_decompress_and_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, E2eCreateMintDecompressAndToken<'info>>,
        data: E2eTestData,
        params: E2eTestParams,
    ) -> Result<()> {
        use light_ctoken_interface::instructions::mint_action::{
            DecompressMintAction, MintActionCompressedInstructionData,
        };
        use light_ctoken_interface::state::TokenDataVersion;
        use light_ctoken_sdk::{
            compressed_token::{
                create_compressed_mint::find_cmint_address, mint_action::MintActionMetaConfig,
            },
            ctoken::{
                CTokenMintToCpi, CompressibleParamsCpi, CreateAssociatedCTokenAccountCpi,
                CreateCTokenAccountCpi,
            },
        };

        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
            return Err(ErrorCode::RentRecipientMismatch.into());
        }

        // 1. Populate PlaceholderRecord (1st PDA)
        let placeholder_record = &mut ctx.accounts.placeholder_record;
        placeholder_record.owner = ctx.accounts.payer.key();
        placeholder_record.name = data.placeholder_name.clone();
        placeholder_record.placeholder_id = data.placeholder_id;
        placeholder_record.counter = data.counter;

        // 2. Populate UserRecord (2nd PDA)
        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = data.user_record_owner;
        user_record.name = data.user_record_name.clone();
        user_record.score = data.user_record_score;
        user_record.category_id = data.user_record_category_id;

        // 3. Setup CPI accounts
        let cpi_accounts = CpiAccounts::new_with_config(
            ctx.accounts.payer.as_ref(),
            ctx.remaining_accounts,
            CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
        );
        let cpi_context_pubkey = cpi_accounts.cpi_context().unwrap().key();
        let cpi_context_account = cpi_accounts.cpi_context().unwrap();

        // 4. Prepare PlaceholderRecord compressed account
        let placeholder_new_address_params = params
            .placeholder_address_tree_info
            .into_new_address_params_assigned_packed(
                placeholder_record.key().to_bytes().into(),
                Some(0),
            );

        let placeholder_info = placeholder_record.to_account_info();
        let placeholder_data_mut = &mut **placeholder_record;
        let placeholder_compressed_info = prepare_compressed_account_on_init::<PlaceholderRecord>(
            &placeholder_info,
            placeholder_data_mut,
            &config,
            params.placeholder_compressed_address,
            placeholder_new_address_params,
            params.placeholder_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            true, // with_data=true: store data in compressed account (required for decompression)
        )?;

        // 5. Prepare UserRecord compressed account (2nd PDA)
        let user_record_new_address_params = params
            .user_record_address_tree_info
            .into_new_address_params_assigned_packed(
                user_record.key().to_bytes().into(),
                Some(1), // 2nd address slot
            );

        let user_record_info = user_record.to_account_info();
        let user_record_data_mut = &mut **user_record;
        let user_record_compressed_info = prepare_compressed_account_on_init::<UserRecord>(
            &user_record_info,
            user_record_data_mut,
            &config,
            params.user_record_compressed_address,
            user_record_new_address_params,
            params.user_record_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            true, // with_data=true: store data in compressed account (required for decompression)
        )?;

        // 6. Write BOTH PDAs to CPI context
        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority().unwrap(),
            cpi_context: cpi_context_account,
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, params.proof.clone())
            .with_new_addresses(&[
                placeholder_new_address_params,
                user_record_new_address_params,
            ])
            .with_account_infos(&[placeholder_compressed_info, user_record_compressed_info])
            .write_to_cpi_context_first()
            .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

        // 7. Build light mint + decompress instruction
        let (cmint_pda, cmint_bump) = find_cmint_address(&ctx.accounts.mint_signer.key());
        let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key;
        let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key;

        let proof = params.proof.0.unwrap_or_default();
        let instruction_data = MintActionCompressedInstructionData::new_mint(
            params.mint_address_tree_info.root_index,
            proof,
            params.mint_with_context.mint.clone().unwrap(),
        )
        // Add DecompressMint action - decompresses the light mint to a CMint account
        .with_decompress_mint(DecompressMintAction {
            cmint_bump,
            rent_payment: params.rent_payment,
            write_top_up: params.write_top_up,
        })
        .with_cpi_context(
            light_ctoken_interface::instructions::mint_action::CpiContext {
                address_tree_pubkey: address_tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false,
                in_tree_index: 1,
                in_queue_index: 0,
                out_queue_index: 0,
                token_out_queue_index: 0,
                assigned_account_index: 2, // 2 PDAs before mint
                read_only_address_trees: [0; 4],
            },
        );

        // 8. Build account metas with compressible CMint
        let mut meta_config = MintActionMetaConfig::new_create_mint(
            ctx.accounts.payer.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.mint_signer.key(),
            address_tree_pubkey,
            output_queue,
        )
        .with_compressible_cmint(
            cmint_pda,
            ctx.accounts.ctoken_compressible_config.key(),
            ctx.accounts.ctoken_rent_sponsor.key(),
        );

        meta_config.cpi_context = Some(cpi_context_pubkey);

        let account_metas = meta_config.to_account_metas();
        let ix_data = instruction_data.data().map_err(ProgramError::from)?;

        let mint_action_ix = solana_program::instruction::Instruction {
            program_id: light_ctoken_interface::CTOKEN_PROGRAM_ID.into(),
            accounts: account_metas,
            data: ix_data,
        };

        // 9. Build account infos and invoke mint action (creates CMint)
        let mut account_infos = cpi_accounts.to_account_infos();
        account_infos.push(ctx.accounts.ctoken_cpi_authority.to_account_info());
        account_infos.push(ctx.accounts.ctoken_program.to_account_info());
        account_infos.push(ctx.accounts.mint_authority.to_account_info());
        account_infos.push(ctx.accounts.mint_signer.to_account_info());
        account_infos.push(ctx.accounts.payer.to_account_info());
        account_infos.push(ctx.accounts.cmint.to_account_info());
        account_infos.push(ctx.accounts.ctoken_compressible_config.to_account_info());
        account_infos.push(ctx.accounts.ctoken_rent_sponsor.to_account_info());

        invoke(&mint_action_ix, &account_infos)?;

        // 8. Create CToken vault via CPI (like cp-swap's token_0_vault/token_1_vault)
        // This creates a program-owned compressible CToken account
        // IMPORTANT: For decompress_accounts_idempotent to work, the token owner MUST equal
        // the CToken account address. So we set owner = vault_pda, not vault_authority_pda.
        CreateCTokenAccountCpi {
            payer: ctx.accounts.payer.to_account_info(),
            account: ctx.accounts.vault.to_account_info(),
            mint: ctx.accounts.cmint.to_account_info(),
            owner: ctx.accounts.vault.key(),
            compressible: CompressibleParamsCpi {
                compressible_config: ctx.accounts.ctoken_compressible_config.to_account_info(),
                rent_sponsor: ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                pre_pay_num_epochs: 2,
                lamports_per_write: None, // No write top-up so it auto-compresses
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: false, // Program-owned accounts can't have compression_only=true
            },
        }
        .invoke_signed(&[&[
            crate::instruction_accounts::VAULT_SEED,
            ctx.accounts.cmint.key().as_ref(),
            &[ctx.bumps.vault],
        ]])?;

        // 9. Create User ATA via CPI (like cp-swap's creator_lp_token)
        // Note: ATAs MUST have compression_only=true per ctoken program requirements.
        // This means compressed ATA owner = ATA address, preventing direct user decompress.
        // ATAs should be decompressed via separate program-controlled flow.
        CreateAssociatedCTokenAccountCpi::new_v2_idempotent(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.user_ata.to_account_info(),
            params.user_ata_bump,
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        )?;

        // 10. Mint tokens to vault (if vault_mint_amount > 0)
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

        // 11. Mint tokens to user's ATA (if user_ata_mint_amount > 0)
        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // 14. Close BOTH PDAs (compress them)
        placeholder_record.close(ctx.accounts.rent_sponsor.to_account_info())?;
        user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;

        Ok(())
    }
}
