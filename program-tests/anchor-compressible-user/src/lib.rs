use anchor_lang::prelude::*;
use light_sdk::{
    account::LightAccount,
    compressible::{
        compress_account_on_init, compress_pda, prepare_accounts_for_compression_on_init,
        prepare_accounts_for_decompress_idempotent, process_initialize_compression_config_checked,
        process_update_compression_config,
    },
    cpi::CpiInputs,
};
use light_sdk::{
    compressible::{CompressibleConfig, CompressionInfo, HasCompressionInfo},
    cpi::CpiAccounts,
    derive_light_cpi_signer,
    instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo, ValidityProof},
    light_hasher::{DataHasher, Hasher},
    LightDiscriminator, LightHasher,
};
use light_sdk_types::CpiSigner;

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK")];
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const COMPRESSION_DELAY: u32 = 100;
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible_user {
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
        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );

        let new_address_params =
            address_tree_info.into_new_address_params_packed(user_record.key().to_bytes());

        compress_account_on_init::<UserRecord>(
            user_record,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &crate::ID,
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

        Ok(())
    }

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
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )?;

        Ok(())
    }

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

    pub fn decompress_multiple_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressMultipleAccountsIdempotent<'info>>,
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        bumps: Vec<u8>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        // Get PDA accounts from remaining accounts
        let pda_accounts_end = system_accounts_offset as usize;
        let pda_accounts = &ctx.remaining_accounts[..pda_accounts_end];

        // Validate we have matching number of PDAs, compressed accounts, and bumps
        if pda_accounts.len() != compressed_accounts.len() || pda_accounts.len() != bumps.len() {
            return err!(ErrorCode::InvalidAccountCount);
        }

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.fee_payer,
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            LIGHT_CPI_SIGNER,
        );

        let mut all_compressed_infos = Vec::new();

        // Process accounts individually but collect CompressedAccountInfo for batched CPI
        for (i, (compressed_data, bump)) in compressed_accounts
            .into_iter()
            .zip(bumps.iter())
            .enumerate()
        {
            match compressed_data.data {
                CompressedAccountVariant::UserRecord(mut data) => {
                    // Create LightAccount with correct UserRecord discriminator
                    let light_account = LightAccount::<'_, UserRecord>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data.clone(),
                    )?;

                    // Build signer seeds with owned data
                    let user_record_seed = b"user_record".to_vec();
                    let owner_bytes = data.owner.to_bytes().to_vec();
                    let bump_vec = vec![*bump];
                    let seeds = vec![
                        user_record_seed.as_slice(),
                        owner_bytes.as_slice(),
                        bump_vec.as_slice(),
                    ];

                    // Process this single UserRecord account
                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<UserRecord>(
                        &[&pda_accounts[i]],
                        vec![light_account],
                        &[&seeds],
                        &cpi_accounts,
                        &crate::ID,
                        &ctx.accounts.rent_payer,
                        ADDRESS_SPACE[0],
                    )?;

                    all_compressed_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::GameSession(mut data) => {
                    // Create LightAccount with correct GameSession discriminator
                    let light_account = LightAccount::<'_, GameSession>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data.clone(),
                    )?;

                    // Build signer seeds with owned data
                    let game_session_seed = b"game_session".to_vec();
                    let session_id_bytes = data.session_id.to_le_bytes().to_vec();
                    let bump_vec = vec![*bump];
                    let seeds = vec![
                        game_session_seed.as_slice(),
                        session_id_bytes.as_slice(),
                        bump_vec.as_slice(),
                    ];

                    // Process this single GameSession account
                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<GameSession>(
                        &[&pda_accounts[i]],
                        vec![light_account],
                        &[&seeds],
                        &cpi_accounts,
                        &crate::ID,
                        &ctx.accounts.rent_payer,
                        ADDRESS_SPACE[0],
                    )?;
                    all_compressed_infos.extend(compressed_infos);
                }
            }
        }

        if !all_compressed_infos.is_empty() {
            let cpi_inputs = CpiInputs::new(proof, all_compressed_infos);
            cpi_inputs.invoke_light_system_program(cpi_accounts)?;
        }

        Ok(())
    }

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

        // Initialize game session data
        game_session.session_id = session_id;
        game_session.player = ctx.accounts.player.key();
        game_session.game_type = game_type;
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.player,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );

        let new_address_params =
            address_tree_info.into_new_address_params_packed(game_session.key().to_bytes());

        compress_account_on_init::<GameSession>(
            game_session,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &crate::ID,
            &config.address_space,
            &ctx.accounts.rent_recipient,
            proof,
        )?;

        Ok(())
    }

    /// Creates both a user record and game session and compresses them
    #[allow(clippy::too_many_arguments)]
    pub fn create_user_record_and_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
        user_name: String,
        session_id: u64,
        game_type: String,
        proof: ValidityProof,
        user_compressed_address: [u8; 32],
        user_address_tree_info: PackedAddressTreeInfo,
        user_output_state_tree_index: u8,
        game_compressed_address: [u8; 32],
        game_address_tree_info: PackedAddressTreeInfo,
        game_output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;
        let game_session = &mut ctx.accounts.game_session;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Initialize user record data
        user_record.owner = ctx.accounts.user.key();
        user_record.name = user_name;
        user_record.score = 11;

        // Initialize game session data
        game_session.session_id = session_id;
        game_session.player = ctx.accounts.user.key();
        game_session.game_type = game_type;
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );

        // Prepare new address params
        let user_new_address_params =
            user_address_tree_info.into_new_address_params_packed(user_record.key().to_bytes());
        let game_new_address_params =
            game_address_tree_info.into_new_address_params_packed(game_session.key().to_bytes());

        let mut all_compressed_infos = Vec::new();

        // Process UserRecord for compression
        let user_compressed_infos = prepare_accounts_for_compression_on_init::<UserRecord>(
            &mut [user_record],
            &[user_compressed_address],
            &[user_new_address_params],
            &[user_output_state_tree_index],
            &cpi_accounts,
            &crate::ID,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;

        all_compressed_infos.extend(user_compressed_infos);

        // Process GameSession for compression
        let game_compressed_infos = prepare_accounts_for_compression_on_init::<GameSession>(
            &mut [game_session],
            &[game_compressed_address],
            &[game_new_address_params],
            &[game_output_state_tree_index],
            &cpi_accounts,
            &crate::ID,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;
        all_compressed_infos.extend(game_compressed_infos);

        // Create CPI inputs with all compressed accounts and new addresses
        let cpi_inputs = CpiInputs::new_with_address(
            proof,
            all_compressed_infos,
            vec![user_new_address_params, game_new_address_params],
        );

        // Invoke light system program to create all compressed accounts
        cpi_inputs.invoke_light_system_program(cpi_accounts)?;

        Ok(())
    }

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

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );

        compress_pda::<UserRecord>(
            user_record,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &crate::ID,
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
        space = 8 + 32 + 4 + 32 + 8 + 9, // discriminator + owner + string len + name + score + compression_info
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
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
#[instruction(user_name: String, session_id: u64)]
pub struct CreateUserRecordAndGameSession<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 4 + 32 + 8 + 9, // discriminator + owner + string len + name + score + compression_info
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    #[account(
        init,
        payer = user,
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
pub struct DecompressMultipleAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    // Remaining accounts:
    // - First N accounts: PDA accounts to decompress into
    // - After system_accounts_offset: Light Protocol system accounts for CPI
}

#[derive(Accounts)]
pub struct InitializeCompressionConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The config PDA to be created
    /// CHECK: Config PDA is created with the correct seeds
    #[account(
        mut,
        seeds = [b"compressible_config"],
        bump
    )]
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
    /// CHECK: Config is validated by the SDK's load_checked method
    #[account(
        mut,
        seeds = [b"compressible_config"],
        bump,
    )]
    pub config: AccountInfo<'info>,
    /// Must match the update authority stored in config
    pub authority: Signer<'info>,
}

/// Unified enum that can hold any account type - perfect for derive macro later
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    GameSession(GameSession),
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
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::GameSession(data) => data.compression_info_mut(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::GameSession(data) => data.set_compression_info_none(),
        }
    }
}

/// Client-side data structures
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMeta,
    pub data: CompressedAccountVariant,
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid account count: PDAs and compressed accounts must match")]
    InvalidAccountCount,
    #[msg("Rent recipient does not match config")]
    InvalidRentRecipient,
}
