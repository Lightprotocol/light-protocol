use anchor_lang::prelude::*;
use light_sdk::instruction::{PackedAddressTreeInfo, ValidityProof};
use light_sdk::{
    compressible::{compress_pda_new, CompressibleConfig},
    cpi::CpiAccounts,
};
use light_sdk::{
    compressible::{CompressionInfo, HasCompressionInfo},
    derive_light_cpi_signer, LightDiscriminator, LightHasher,
};
use light_sdk_macros::add_compressible_instructions;
use light_sdk_types::CpiSigner;

declare_id!("CompUser11111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[add_compressible_instructions(UserRecord, GameSession)]
#[program]
pub mod anchor_compressible_user_derived {

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
        user_record.score = 0;
        // Initialize compression info with current slot
        user_record.compression_info = CompressionInfo::new()
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let cpi_accounts = CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );
        let new_address_params =
            address_tree_info.into_new_address_params_packed(user_record.key().to_bytes());

        compress_pda_new::<UserRecord>(
            &user_record.to_account_info(),
            compressed_address,
            new_address_params,
            output_state_tree_index,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient,
            &config.address_space,
            None,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

        Ok(())
    }

    /// Can be the same because the PDA will be decompressed in a separate instruction.
    /// Updates an existing user record
    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        Ok(())
    }
    // The add_compressible_instructions macro will generate:
    // - create_compression_config (config management)
    // - update_compression_config (config management)
    // - compress_user_record (compress existing PDA)
    // - compress_game_session (compress existing PDA)
    // - decompress_multiple_pdas (decompress compressed accounts)
    // Plus all the necessary structs and enums
}

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: CompressionInfo,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace)]
#[account]
pub struct GameSession {
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[hash]
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    #[skip]
    pub compression_info: CompressionInfo,
    pub end_time: Option<u64>,
    pub score: u64,
}

impl HasCompressionInfo for GameSession {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
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
    pub config: AccountInfo<'info>,
    /// CHECK: checked in helper
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
