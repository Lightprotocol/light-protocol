// #![cfg_attr(target_os = "solana", allow(unused_variables, unused_mut))]

use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke, pubkey::Pubkey},
};
use light_compressed_token_sdk::instructions::create_compressed_mint::{
    create_compressed_mint, CreateCompressedMintInputs,
};
use light_compressed_token_types::constants::CPI_AUTHORITY_PDA;
use light_ctoken_types::{
    instructions::extensions::{ExtensionInstructionData, TokenMetadataInstructionData},
    state::{AdditionalMetadata, Metadata},
    COMPRESSED_MINT_SEED,
};
use light_sdk_types::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, C_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};

use light_sdk::{
    account::Size,
    compressible::{
        compress_account, compress_account_on_init, compress_empty_account_on_init,
        prepare_accounts_for_compression_on_init, prepare_accounts_for_decompress_idempotent,
        process_initialize_compression_config_checked, process_update_compression_config,
        CompressAs, CompressibleConfig, CompressionInfo, HasCompressionInfo,
    },
    cpi::{CpiAccounts, CpiInputs},
    derive_light_cpi_signer,
    instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo, ValidityProof},
    light_hasher::{DataHasher, Hasher},
    sha::LightAccount,
    LightDiscriminator, LightHasher,
};
use light_sdk_types::CpiSigner;

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible {

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        // 1. Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        user_record.owner = ctx.accounts.user.key();
        user_record.name = name;
        user_record.score = 11;

        // 2. Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // 3. Create CPI accounts
        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        let new_address_params =
            address_tree_info.into_new_address_params_packed(user_record.key().to_bytes());

        compress_account_on_init::<UserRecord>(
            user_record,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
            proof,
        )?;

        Ok(())
    }

    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        // 1. Must manually set compression info
        user_record.compression_info_mut().set_last_written_slot()?;

        Ok(())
    }

    pub fn update_game_session(
        ctx: Context<UpdateGameSession>,
        _session_id: u64,
        new_score: u64,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        game_session.score = new_score;
        game_session.end_time = Some(Clock::get()?.unix_timestamp as u64);

        // Must manually set compression info
        game_session
            .compression_info_mut()
            .set_last_written_slot()?;

        Ok(())
    }

    // auto-derived via macro.
    pub fn initialize_compression_config(
        ctx: Context<InitializeCompressionConfig>,
        compression_delay: u32,
        rent_recipient: Pubkey,
        address_space: Vec<Pubkey>,
    ) -> Result<()> {
        process_initialize_compression_config_checked(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.program_data.to_account_info(),
            &rent_recipient,
            address_space,
            compression_delay,
            0, // one global config for now, so bump is 0.
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )?;

        Ok(())
    }

    // auto-derived via macro.
    pub fn update_compression_config(
        ctx: Context<UpdateCompressionConfig>,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        process_update_compression_config(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            new_update_authority.as_ref(),
            new_rent_recipient.as_ref(),
            new_address_space,
            new_compression_delay,
            &crate::ID,
        )?;

        Ok(())
    }

    // auto-derived via macro. takes the tagged account structs via
    // add_compressible_accounts macro and derives the relevant variant type and
    // dispatcher. The instruction can be used with any number of any of the
    // tagged account structs. It's idempotent; it will not fail if the accounts
    // are already decompressed.
    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        bumps: Vec<u8>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        // Get PDA accounts from remaining accounts
        let pda_accounts_end = system_accounts_offset as usize;
        let solana_accounts = &ctx.remaining_accounts[..pda_accounts_end];

        // Validate we have matching number of PDAs, compressed accounts, and bumps
        if solana_accounts.len() != compressed_accounts.len()
            || solana_accounts.len() != bumps.len()
        {
            return err!(ErrorCode::InvalidAccountCount);
        }

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.fee_payer,
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            LIGHT_CPI_SIGNER,
        );

        // Get address space from config checked.
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
        let address_space = config.address_space[0];

        let mut all_compressed_infos = Vec::with_capacity(compressed_accounts.len());

        for (i, (compressed_data, &bump)) in compressed_accounts
            .into_iter()
            .zip(bumps.iter())
            .enumerate()
        {
            let bump_slice = [bump];

            match compressed_data.data {
                CompressedAccountVariant::UserRecord(data) => {
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct UserRecord discriminator
                    let light_account = LightAccount::<'_, UserRecord>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single UserRecord account
                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<UserRecord>(
                        &[&solana_accounts[i]],
                        vec![light_account],
                        &[seeds_refs.as_slice()],
                        &cpi_accounts,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;

                    all_compressed_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::GameSession(data) => {
                    // Build seeds refs without cloning - pre-allocate capacity
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct GameSession discriminator
                    let light_account = LightAccount::<'_, GameSession>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single GameSession account
                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<GameSession>(
                        &[&solana_accounts[i]],
                        vec![light_account],
                        &[seeds_refs.as_slice()],
                        &cpi_accounts,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;
                    all_compressed_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::PlaceholderRecord(data) => {
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct PlaceholderRecord discriminator
                    let light_account = LightAccount::<'_, PlaceholderRecord>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single PlaceholderRecord account
                    let compressed_infos =
                        prepare_accounts_for_decompress_idempotent::<PlaceholderRecord>(
                            &[&solana_accounts[i]],
                            vec![light_account],
                            &[seeds_refs.as_slice()],
                            &cpi_accounts,
                            &ctx.accounts.rent_payer,
                            address_space,
                        )?;

                    all_compressed_infos.extend(compressed_infos);
                }
            }
        }

        if all_compressed_infos.is_empty() {
            msg!("No compressed accounts to decompress");
        } else {
            let cpi_inputs = CpiInputs::new(proof, all_compressed_infos);
            cpi_inputs.invoke_light_system_program(cpi_accounts)?;
        }
        Ok(())
    }

    // Must be manually implemented.
    pub fn create_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateGameSession<'info>>,
        session_id: u64,
        game_type: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Set your account data.
        game_session.session_id = session_id;
        game_session.player = ctx.accounts.player.key();
        game_session.game_type = game_type;
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // Check that rent recipient matches your config.
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Create CPI accounts.
        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.player,
            ctx.remaining_accounts,
            LIGHT_CPI_SIGNER,
        );

        // Prepare new address params. The cpda takes the address of the
        // compressible pda account as seed.
        let new_address_params =
            address_tree_info.into_new_address_params_packed(game_session.key().to_bytes());

        // Call at the end of your init instruction to compress the pda account
        // safely. This also closes the pda account. The account can then be
        // decompressed by anyone at any time via the
        // decompress_accounts_idempotent instruction. Creates a unique cPDA to
        // ensure that the account cannot be re-inited only decompressed.
        compress_account_on_init::<GameSession>(
            game_session,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
            proof,
        )?;

        Ok(())
    }

    // Must be manually implemented.
    pub fn create_user_record_and_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
        account_data: AccountCreationData,
        compression_params: CompressionParams,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;
        let game_session = &mut ctx.accounts.game_session;

        // Load your config checked.
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Check that rent recipient matches your config.
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Set your account data.
        user_record.owner = ctx.accounts.user.key();
        user_record.name = account_data.user_name.clone();
        user_record.score = 11;
        game_session.session_id = account_data.session_id;
        game_session.player = ctx.accounts.user.key();
        game_session.game_type = account_data.game_type.clone();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // Log compressed mint creation intent with metadata
        msg!(
            "Creating compressed mint with metadata: name={}, symbol={}, uri={}, decimals={}, supply={}",
            account_data.mint_name,
            account_data.mint_symbol,
            account_data.mint_uri,
            account_data.mint_decimals,
            account_data.mint_supply
        );

        // Log mint authorities
        msg!("Mint authority: {:?}", ctx.accounts.user.key());
        if let Some(freeze_auth) = account_data.mint_freeze_authority {
            msg!("Freeze authority: {:?}", freeze_auth);
        }
        if let Some(update_auth) = account_data.mint_update_authority {
            msg!("Update authority: {:?}", update_auth);
        }

        // Log additional metadata if provided
        if let Some(metadata) = &account_data.additional_metadata {
            msg!("Additional metadata:");
            for (key, value) in metadata {
                msg!("  {}: {}", key, value);
            }
        }

        // Find mint PDA
        let compressed_token_program_id = Pubkey::new_from_array(C_TOKEN_PROGRAM_ID);
        let (mint_pda, mint_bump) = Pubkey::find_program_address(
            &[
                COMPRESSED_MINT_SEED,
                ctx.accounts.mint_signer.key().as_ref(),
            ],
            &compressed_token_program_id,
        );

        // Derive the compressed mint address using the PDA as seed
        let address_seed = mint_pda.to_bytes();
        let mint_address = light_compressed_account::address::derive_address(
            &address_seed,
            &ctx.accounts.address_merkle_tree.key().to_bytes(),
            &compressed_token_program_id.to_bytes(),
        );

        msg!("Mint PDA: {:?}, bump: {}", mint_pda, mint_bump);
        msg!("Compressed mint address: {:?}", mint_address);
        msg!("Address tree: {:?}", ctx.accounts.address_merkle_tree.key());

        // Convert additional metadata to the correct format
        let additional_metadata_converted = account_data.additional_metadata.map(|metadata| {
            metadata
                .into_iter()
                .map(|(key, value)| AdditionalMetadata {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                })
                .collect()
        });

        // Create token metadata extension
        let token_metadata = TokenMetadataInstructionData {
            update_authority: account_data
                .mint_update_authority
                .map(|ua| light_compressed_account::Pubkey::new_from_array(ua.to_bytes())),
            metadata: Metadata {
                name: account_data.mint_name.into_bytes(),
                symbol: account_data.mint_symbol.into_bytes(),
                uri: account_data.mint_uri.into_bytes(),
            },
            additional_metadata: additional_metadata_converted,
            version: 0,
        };

        // Create extension instruction data
        let extensions = vec![ExtensionInstructionData::TokenMetadata(token_metadata)];

        // Convert the proof to the correct type for the SDK
        let compressed_proof = compression_params.proof.0.unwrap();

        // Create the compressed mint inputs
        let mint_inputs = CreateCompressedMintInputs {
            decimals: account_data.mint_decimals,
            mint_authority: ctx.accounts.user.key(),
            freeze_authority: account_data.mint_freeze_authority,
            proof: compressed_proof,
            mint_bump: compression_params.mint_signer_bump,
            address_merkle_tree_root_index: compression_params.mint_address_tree_info.root_index,
            mint_signer: ctx.accounts.mint_signer.key(),
            payer: ctx.accounts.user.key(),
            address_tree_pubkey: ctx.accounts.address_merkle_tree.key(),
            output_queue: ctx.accounts.output_queue.key(),
            extensions: Some(extensions),
            version: 0,
        };

        // Create the compressed mint instruction using the SDK
        let mint_instruction =
            create_compressed_mint(mint_inputs).map_err(|_| ErrorCode::MintCreationFailed)?;

        // Validate program IDs before CPI
        require_keys_eq!(
            ctx.accounts.light_system_program.key(),
            Pubkey::new_from_array(LIGHT_SYSTEM_PROGRAM_ID),
            ErrorCode::MintCreationFailed
        );
        require_keys_eq!(
            ctx.accounts.account_compression_program.key(),
            Pubkey::new_from_array(ACCOUNT_COMPRESSION_PROGRAM_ID),
            ErrorCode::MintCreationFailed
        );
        require_keys_eq!(
            ctx.accounts.compressed_token_program.key(),
            compressed_token_program_id,
            ErrorCode::MintCreationFailed
        );

        // Create account infos for the CPI call in expected order
        let mut mint_account_infos = vec![
            ctx.accounts.mint_signer.to_account_info(),
            ctx.accounts.user.to_account_info(), // payer
            ctx.accounts.cpi_authority_pda.to_account_info(),
            ctx.accounts.light_system_program.to_account_info(),
            ctx.accounts.account_compression_program.to_account_info(),
            ctx.accounts.registered_program_pda.to_account_info(),
            ctx.accounts.noop_program.to_account_info(),
            ctx.accounts.account_compression_authority.to_account_info(),
            ctx.accounts.compressed_token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.address_merkle_tree.to_account_info(),
            ctx.accounts.output_queue.to_account_info(),
        ];

        // Add remaining accounts to the instruction
        for remaining_account in ctx.remaining_accounts {
            mint_account_infos.push(remaining_account.to_account_info());
        }

        invoke(&mint_instruction, &mint_account_infos)?;
        msg!("Compressed mint with metadata created successfully");

        // Now continue with the original logic for user record and game session
        // Create CPI accounts.
        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        // Prepare new address params. One per pda account.
        let user_new_address_params = compression_params
            .user_address_tree_info
            .into_new_address_params_packed(user_record.key().to_bytes());
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_packed(game_session.key().to_bytes());

        let mut all_compressed_infos = Vec::new();

        // Prepares the firstpda account for compression. compress the pda
        // account safely. This also closes the pda account. safely. This also
        // closes the pda account. The account can then be decompressed by
        // anyone at any time via the decompress_accounts_idempotent
        // instruction. Creates a unique cPDA to ensure that the account cannot
        // be re-inited only decompressed.
        let user_compressed_infos = prepare_accounts_for_compression_on_init::<UserRecord>(
            &mut [user_record],
            &[compression_params.user_compressed_address],
            &[user_new_address_params],
            &[compression_params.user_output_state_tree_index],
            &cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;

        all_compressed_infos.extend(user_compressed_infos);

        // Process GameSession for compression. compress the pda account safely.
        // This also closes the pda account. The account can then be
        // decompressed by anyone at any time via the
        // decompress_accounts_idempotent instruction. Creates a unique cPDA to
        // ensure that the account cannot be re-inited only decompressed.
        let game_compressed_infos = prepare_accounts_for_compression_on_init::<GameSession>(
            &mut [game_session],
            &[compression_params.game_compressed_address],
            &[game_new_address_params],
            &[compression_params.game_output_state_tree_index],
            &cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;
        all_compressed_infos.extend(game_compressed_infos);

        // Create CPI inputs with all compressed accounts and new addresses
        let cpi_inputs = CpiInputs::new_with_address(
            compression_params.proof,
            all_compressed_infos,
            vec![user_new_address_params, game_new_address_params],
        );

        // Invoke light system program to create all compressed accounts in one
        // CPI. Call at the end of your init instruction.
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;

        Ok(())
    }

    // Auto-derived via macro. Based on target account type, it will compress
    // the pda account safely. This also closes the pda account. The account can
    // then be decompressed by anyone at any time via the
    // decompress_accounts_idempotent instruction. Does not create a new cPDA.
    // but requires the existing (empty) compressed account to be passed in.
    pub fn compress_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressRecord<'info>>,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        compress_account::<UserRecord>(
            user_record,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        Ok(())
    }

    /// Compresses a GameSession PDA with custom data using config values.
    /// This demonstrates the custom compression feature which allows resetting
    /// some fields (start_time, end_time, score) while keeping others (session_id, player, game_type).
    pub fn compress_game_session_with_custom_data<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressGameSession<'info>>,
        _session_id: u64,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.player,
            ctx.remaining_accounts,
            LIGHT_CPI_SIGNER,
        );

        compress_account::<GameSession>(
            game_session,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        Ok(())
    }

    /// Creates an empty compressed account while keeping the PDA intact.
    /// This demonstrates the compress_empty_account_on_init functionality.
    pub fn create_placeholder_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePlaceholderRecord<'info>>,
        placeholder_id: u64,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let placeholder_record = &mut ctx.accounts.placeholder_record;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        placeholder_record.owner = ctx.accounts.user.key();
        placeholder_record.name = name;
        placeholder_record.placeholder_id = placeholder_id;

        // Initialize compression_info for the PDA
        *placeholder_record.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);
        placeholder_record
            .compression_info_mut()
            .set_last_written_slot()?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Create CPI accounts
        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        let new_address_params =
            address_tree_info.into_new_address_params_packed(placeholder_record.key().to_bytes());

        // Use the new compress_empty_account_on_init function
        // This creates an empty compressed account but does NOT close the PDA
        compress_empty_account_on_init::<PlaceholderRecord>(
            placeholder_record,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            proof,
        )?;

        Ok(())
    }

    /// Compresses a PlaceholderRecord PDA using config values.
    pub fn compress_placeholder_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressPlaceholderRecord<'info>>,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let placeholder_record = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        compress_account::<PlaceholderRecord>(
            placeholder_record,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + owner + string len + name + score +
        // option<compression_info>. Note that in the onchain space
        // CompressionInfo is always Some.
        space = 8 + 32 + 4 + 32 + 8 + 10,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(placeholder_id: u64)]
pub struct CreatePlaceholderRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + compression_info + owner + string len + name + placeholder_id
        space = 8 + 10 + 32 + 4 + 32 + 8,
        seeds = [b"placeholder_record", placeholder_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub placeholder_record: Account<'info, PlaceholderRecord>,
    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(account_data: AccountCreationData, compression_params: CompressionParams)]
pub struct CreateUserRecordAndGameSession<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + owner + string len + name + score +
        // option<compression_info>. Note that in the onchain space
        // CompressionInfo is always Some.
        space = 8 + 32 + 4 + 32 + 8 + 10,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    #[account(
        init,
        payer = user,
        // discriminator + option<compression_info> + session_id + player +
        // string len + game_type + start_time + end_time(Option) + score
        space = 8 + 10 + 8 + 32 + 4 + 32 + 8 + 9 + 8,
        seeds = [b"game_session", account_data.session_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,

    // Compressed mint creation accounts
    /// The mint signer used for PDA derivation
    pub mint_signer: Signer<'info>,

    /// CPI authority for compressed account creation
    /// CHECK: Validated by compressed-token program
    pub cpi_authority_pda: AccountInfo<'info>,

    /// Light system program for compressed account creation
    /// CHECK: Program ID validated using LIGHT_SYSTEM_PROGRAM_ID constant
    pub light_system_program: UncheckedAccount<'info>,

    /// Account compression program
    /// CHECK: Program ID validated using ACCOUNT_COMPRESSION_PROGRAM_ID constant
    pub account_compression_program: UncheckedAccount<'info>,

    /// Registered program PDA for light system program
    /// CHECK: Validated by light-system-program
    pub registered_program_pda: AccountInfo<'info>,

    /// NoOp program for event emission
    /// CHECK: Validated by light-system-program
    pub noop_program: UncheckedAccount<'info>,

    /// Authority for account compression
    /// CHECK: Validated by light-system-program
    pub account_compression_authority: UncheckedAccount<'info>,

    /// Compressed token program
    /// CHECK: Program ID validated using COMPRESSED_TOKEN_PROGRAM_ID constant
    pub compressed_token_program: UncheckedAccount<'info>,

    /// Address merkle tree for compressed account creation
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub address_merkle_tree: AccountInfo<'info>,

    /// Output queue account where compressed mint will be stored
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,

    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct CreateGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        init,
        payer = player,
        space = 8 + 9 + 8 + 32 + 4 + 32 + 8 + 9 + 8, // discriminator + compression_info + session_id + player + string len + game_type + start_time + end_time(Option) + score
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = user_record.owner == user.key()
    )]
    pub user_record: Account<'info, UserRecord>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct UpdateGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
        constraint = game_session.player == player.key()
    )]
    pub game_session: Account<'info, GameSession>,
}

#[derive(Accounts)]
pub struct CompressRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = pda_to_compress.owner == user.key()
    )]
    pub pda_to_compress: Account<'info, UserRecord>,
    // pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct CompressGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
        constraint = pda_to_compress.player == player.key()
    )]
    pub pda_to_compress: Account<'info, GameSession>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CompressPlaceholderRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        constraint = pda_to_compress.owner == user.key()
    )]
    pub pda_to_compress: Account<'info, PlaceholderRecord>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// UNCHECKED: Anyone can pay to init.
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// The global config account
    /// CHECK: load_checked.
    pub config: AccountInfo<'info>,
    // Remaining accounts:
    // - First N accounts: PDA accounts to decompress into
    // - After system_accounts_offset: Light Protocol system accounts for CPI
}

#[derive(Accounts)]
pub struct InitializeCompressionConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Config PDA is created and validated by the SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// The program's data account
    /// CHECK: Program data account is validated by the SDK
    pub program_data: AccountInfo<'info>,
    /// The program's upgrade authority (must sign)
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCompressionConfig<'info> {
    /// CHECK: Config PDA is created and validated by the SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// Must match the update authority stored in config
    pub authority: Signer<'info>,
}

/// Auto-derived via macro. Unified enum that can hold any account type. Crucial
/// for dispatching multiple compressed accounts of different types in
/// decompress_accounts_idempotent.
/// Implements: Default, DataHasher, LightDiscriminator, HasCompressionInfo.
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    GameSession(GameSession),
    PlaceholderRecord(PlaceholderRecord),
}

impl Default for CompressedAccountVariant {
    fn default() -> Self {
        Self::UserRecord(UserRecord::default())
    }
}

impl DataHasher for CompressedAccountVariant {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
        match self {
            Self::UserRecord(data) => data.hash::<H>(),
            Self::GameSession(data) => data.hash::<H>(),
            Self::PlaceholderRecord(data) => data.hash::<H>(),
        }
    }
}

impl LightDiscriminator for CompressedAccountVariant {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // This won't be used directly
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl HasCompressionInfo for CompressedAccountVariant {
    fn compression_info(&self) -> &CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info(),
            Self::GameSession(data) => data.compression_info(),
            Self::PlaceholderRecord(data) => data.compression_info(),
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::GameSession(data) => data.compression_info_mut(),
            Self::PlaceholderRecord(data) => data.compression_info_mut(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
            Self::PlaceholderRecord(data) => data.compression_info_mut_opt(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::GameSession(data) => data.set_compression_info_none(),
            Self::PlaceholderRecord(data) => data.set_compression_info_none(),
        }
    }
}

impl Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            Self::UserRecord(data) => data.size(),
            Self::GameSession(data) => data.size(),
            Self::PlaceholderRecord(data) => data.size(),
        }
    }
}

// Auto-derived via macro. Ix data implemented for Variant.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMeta,
    pub data: CompressedAccountVariant,
    pub seeds: Vec<Vec<u8>>,
}

#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

// Auto-derived via macro.
impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for UserRecord {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for UserRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        // Simple case: return owned data with compression_info = None
        // We can't return Cow::Borrowed because compression_info must always be None for compressed storage
        std::borrow::Cow::Owned(Self {
            compression_info: None, // ALWAYS None for compressed storage
            owner: self.owner,
            name: self.name.clone(),
            score: self.score,
        })
    }
}

// Your existing account structs must be manually extended:
// 1. Add compression_info field to the struct, with type
//    Option<CompressionInfo>.
// 2. add a #[skip] field for the compression_info field.
// 3. Add LightHasher, LightDiscriminator.
// 4. Add #[hash] attribute to ALL fields that can be >31 bytes. (eg Pubkeys,
//    Strings)
#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

// Auto-derived via macro.
impl HasCompressionInfo for GameSession {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for GameSession {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for GameSession {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        // Custom compression: return owned data with modified fields
        std::borrow::Cow::Owned(Self {
            compression_info: None,            // ALWAYS None for compressed storage
            session_id: self.session_id,       // KEEP - identifier
            player: self.player,               // KEEP - identifier
            game_type: self.game_type.clone(), // KEEP - core property
            start_time: 0,                     // RESET - clear timing
            end_time: None,                    // RESET - clear timing
            score: 0,                          // RESET - clear progress
        })
    }
}

// PlaceholderRecord - demonstrates empty compressed account creation
// The PDA remains intact while an empty compressed account is created
#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct PlaceholderRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub placeholder_id: u64,
}

impl HasCompressionInfo for PlaceholderRecord {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for PlaceholderRecord {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for PlaceholderRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        std::borrow::Cow::Owned(Self {
            compression_info: None,
            owner: self.owner,
            name: self.name.clone(),
            placeholder_id: self.placeholder_id,
        })
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid account count: PDAs and compressed accounts must match")]
    InvalidAccountCount,
    #[msg("Rent recipient does not match config")]
    InvalidRentRecipient,
    #[msg("Failed to create compressed mint")]
    MintCreationFailed,
}

// Add these struct definitions before the program module
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AccountCreationData {
    pub user_name: String,
    pub session_id: u64,
    pub game_type: String,
    // TODO: Add mint metadata fields when implementing mint functionality
    pub mint_name: String,
    pub mint_symbol: String,
    pub mint_uri: String,
    pub mint_decimals: u8,
    pub mint_supply: u64,
    pub mint_update_authority: Option<Pubkey>,
    pub mint_freeze_authority: Option<Pubkey>,
    pub additional_metadata: Option<Vec<(String, String)>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressionParams {
    pub proof: ValidityProof,
    pub user_compressed_address: [u8; 32],
    pub user_address_tree_info: PackedAddressTreeInfo,
    pub user_output_state_tree_index: u8,
    pub game_compressed_address: [u8; 32],
    pub game_address_tree_info: PackedAddressTreeInfo,
    pub game_output_state_tree_index: u8,
    // TODO: Add mint compression parameters when implementing mint functionality
    pub mint_compressed_address: [u8; 32],
    pub mint_address_tree_info: PackedAddressTreeInfo,
    pub mint_output_state_tree_index: u8,
    pub mint_signer_bump: u8,
}
