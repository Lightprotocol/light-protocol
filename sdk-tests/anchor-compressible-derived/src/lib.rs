use anchor_lang::prelude::*;
use light_sdk::{
    compressible::{
        compress_account_on_init, prepare_accounts_for_compression_on_init, CompressibleConfig,
        CompressionInfo, HasCompressionInfo,
    },
    cpi::{CpiAccounts, CpiInputs},
    derive_light_cpi_signer,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightDiscriminator, LightHasher,
};
use light_sdk_macros::{add_compressible_instructions, HasCompressionInfo};
use light_sdk_types::CpiSigner;

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[add_compressible_instructions(UserRecord, GameSession)]
#[program]
pub mod anchor_compressible_derived {

    use super::*;
    /// Creates a new compressed user record using global config.
    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        proof: ValidityProof,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        user_record.owner = ctx.accounts.user.key();
        user_record.name = name;
        user_record.score = 11;
        // Initialize compression info with current slot
        user_record.compression_info = Some(
            CompressionInfo::new_decompressed()
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?,
        );

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

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

    pub fn update_record(
        ctx: Context<UpdateRecord>,
        name: String,
        score: u64,
    ) -> anchor_lang::Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        // Update the record data
        user_record.name = name;
        user_record.score = score;

        // MANUALLY set the last written slot using the trait
        user_record
            .compression_info_mut()
            .set_last_written_slot()
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        Ok(())
    }

    /// Creates both a user record and game session in one instruction.
    /// Must be manually implemented.
    pub fn create_record_and_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecordAndSession<'info>>,
        account_data: AccountCreationData,
        compression_params: CompressionParams,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;
        let game_session = &mut ctx.accounts.game_session;

        // Load config checked
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        // Check that rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Set user record data
        user_record.owner = ctx.accounts.user.key();
        user_record.name = account_data.user_name;
        user_record.score = 11;
        // Initialize compression info with current slot
        user_record.compression_info = Some(
            CompressionInfo::new_decompressed()
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?,
        );

        // Set game session data
        game_session.session_id = account_data.session_id;
        game_session.player = ctx.accounts.user.key();
        game_session.game_type = account_data.game_type;
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;
        // Initialize compression info with current slot
        game_session.compression_info = Some(
            CompressionInfo::new_decompressed()
                .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?,
        );

        // Create CPI accounts
        let cpi_accounts =
            CpiAccounts::new(&ctx.accounts.user, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        // Prepare new address params for both accounts
        let user_new_address_params = compression_params
            .user_address_tree_info
            .into_new_address_params_packed(user_record.key().to_bytes());
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_packed(game_session.key().to_bytes());

        let mut all_compressed_infos = Vec::new();

        // Prepare user record for compression
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

        // Prepare game session for compression
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

        // Invoke light system program to create all compressed accounts in one CPI
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;

        Ok(())
    }

    // The add_compressible_instructions macro will generate:
    // - initialize_compression_config (config management)
    // - update_compression_config (config management)
    // - compress_record (compress existing PDA)
    // - compress_session (compress existing PDA)
    // - decompress_accounts_idempotent (decompress compressed accounts)
    // Plus all the necessary structs and enums

    #[derive(Accounts)]
    pub struct CreateRecord<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(
            init,
            payer = user,
            // Manually add 10 bytes! Discriminator + owner + string len + name +
            // score + option<compression_info>
            space = 8 + 32 + 4 + 32 + 8 + 10,
            seeds = [b"user_record", user.key().as_ref()],
            bump,
        )]
        pub user_record: Account<'info, UserRecord>,
        /// UNCHECKED: checked via config.
        #[account(mut)]
        pub rent_recipient: AccountInfo<'info>,
        /// The global config account
        /// UNCHECKED: checked via load_checked.
        pub config: AccountInfo<'info>,
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    #[instruction(account_data: AccountCreationData)]
    pub struct CreateRecordAndSession<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(
            init,
            payer = user,
            // discriminator + owner + string len + name + score + option<compression_info>
            space = 8 + 32 + 4 + 32 + 8 + 10,
            seeds = [b"user_record", user.key().as_ref()],
            bump,
        )]
        pub user_record: Account<'info, UserRecord>,
        #[account(
            init,
            payer = user,
            // discriminator + option<compression_info> + session_id + player + string len + game_type + start_time + end_time(Option) + score
            space = 8 + 10 + 8 + 32 + 4 + 32 + 8 + 9 + 8,
            seeds = [b"game_session", account_data.session_id.to_le_bytes().as_ref()],
            bump,
        )]
        pub game_session: Account<'info, GameSession>,
        pub system_program: Program<'info, System>,
        /// The global config account
        /// UNCHECKED: checked via load_checked.
        pub config: AccountInfo<'info>,
        /// UNCHECKED: checked via config.
        #[account(mut)]
        pub rent_recipient: AccountInfo<'info>,
    }
}

// Re-export the generated types for client access Explicitly re-export only the
// macro-generated types you need to expose. This avoids any name clash with the
// module itself.
pub use crate::anchor_compressible_derived::{CompressedAccountData, CompressedAccountVariant};

#[derive(Debug, LightHasher, LightDiscriminator, HasCompressionInfo, Default, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace, HasCompressionInfo)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[hash]
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
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

#[error_code]
pub enum ErrorCode {
    #[msg("Rent recipient does not match config")]
    InvalidRentRecipient,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AccountCreationData {
    pub user_name: String,
    pub session_id: u64,
    pub game_type: String,
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
}
