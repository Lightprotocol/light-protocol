use anchor_lang::prelude::*;
use light_sdk::{
    compressible::compress_pda_new,
    cpi::CpiAccounts,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use light_sdk::{derive_light_cpi_signer, LightDiscriminator, LightHasher};
use light_sdk_macros::compressible;
use light_sdk_types::CpiAccountsConfig;
use light_sdk_types::CpiSigner;

declare_id!("CompUser11111111111111111111111111111111111");
pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const SLOTS_UNTIL_COMPRESSION: u64 = 100;
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible_user {
    use super::*;

    /// Creates a new compressed user record.
    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.owner = ctx.accounts.user.key();
        user_record.name = name;
        user_record.score = 0;
        user_record.slots_until_compression = SLOTS_UNTIL_COMPRESSION;

        let cpi_accounts = CpiAccounts::new_with_config(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            CpiAccountsConfig::new(LIGHT_CPI_SIGNER),
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
            &ADDRESS_SPACE,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

        Ok(())
    }

    /// Updates an existing user record
    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

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
        space = 8 + 32 + 4 + 32 + 8 + 8 + 8, // discriminator + owner + string len + name + score + last_written_slot + slots_until_compression
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    pub system_program: Program<'info, System>,
    /// CHECK: hardcoded RENT_RECIPIENT
    #[account(address = RENT_RECIPIENT)]
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

// Define compressible accounts using the macro
#[compressible]
#[derive(Debug, LightHasher, LightDiscriminator, Default)]
#[account]
pub struct UserRecord {
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}

impl light_sdk::compressible::PdaTimingData for UserRecord {
    fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn slots_until_compression(&self) -> u64 {
        self.slots_until_compression
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}

#[compressible]
#[derive(Debug, LightHasher, LightDiscriminator, Default)]
#[account]
pub struct GameSession {
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}

impl light_sdk::compressible::PdaTimingData for GameSession {
    fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn slots_until_compression(&self) -> u64 {
        self.slots_until_compression
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}
