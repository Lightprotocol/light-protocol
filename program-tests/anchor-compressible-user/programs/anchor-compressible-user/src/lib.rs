#![allow(unexpected_cfgs)]

use anchor_lang::{prelude::*, Discriminator};
use light_sdk::{
    account::LightAccount,
    address::v1::derive_address,
    cpi::{CpiAccounts, CpiInputs, CpiSigner},
    derive_light_cpi_signer,
    instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo, ValidityProof},
    pda::{compress_pda, compress_pda_new, decompress_idempotent, PdaTimingData},
    LightDiscriminator, LightHasher,
};

declare_id!("CompUser11111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("CompUser11111111111111111111111111111111111");

#[program]
pub mod anchor_compressible_user {
    use super::*;

    /// Creates a new compressed user record based on the signer's pubkey
    pub fn create_user_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecord<'info>>,
        proof: ValidityProof,
        address_tree_info: PackedAddressTreeInfo,
        output_tree_index: u8,
        name: String,
        bio: String,
    ) -> Result<()> {
        let user = ctx.accounts.user.key();

        // Derive address using user's pubkey as seed
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        let (address, address_seed) = derive_address(
            &[b"user_record", user.as_ref()],
            &address_tree_info
                .get_tree_pubkey(&light_cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
            &crate::ID,
        );

        let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

        // Create the compressed user record
        let mut user_record =
            LightAccount::<'_, UserRecord>::new_init(&crate::ID, Some(address), output_tree_index);

        user_record.owner = user;
        user_record.name = name;
        user_record.bio = bio;
        user_record.score = 0;
        user_record.created_at = Clock::get()?.unix_timestamp;
        user_record.updated_at = Clock::get()?.unix_timestamp;
        user_record.last_written_slot = Clock::get()?.slot;
        user_record.slots_until_compression = 100; // Can be compressed after 100 slots

        let cpi_inputs = CpiInputs::new_with_address(
            proof,
            vec![user_record.to_account_info().map_err(ProgramError::from)?],
            vec![new_address_params],
        );

        cpi_inputs
            .invoke_light_system_program(light_cpi_accounts)
            .map_err(ProgramError::from)?;

        emit!(UserRecordCreated {
            user,
            address,
            name: user_record.name.clone(),
        });

        Ok(())
    }

    /// Updates an existing compressed user record
    pub fn update_user_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateUserRecord<'info>>,
        proof: ValidityProof,
        account_meta: CompressedAccountMeta,
        current_record: UserRecord,
        new_name: Option<String>,
        new_bio: Option<String>,
        score_delta: Option<i64>,
    ) -> Result<()> {
        let user = ctx.accounts.user.key();

        // Verify ownership
        require!(current_record.owner == user, ErrorCode::ConstraintOwner);

        let mut user_record =
            LightAccount::<'_, UserRecord>::new_mut(&crate::ID, &account_meta, current_record)
                .map_err(ProgramError::from)?;

        // Update fields if provided
        if let Some(name) = new_name {
            user_record.name = name;
        }
        if let Some(bio) = new_bio {
            user_record.bio = bio;
        }
        if let Some(delta) = score_delta {
            user_record.score = user_record.score.saturating_add_signed(delta);
        }
        user_record.updated_at = Clock::get()?.unix_timestamp;
        user_record.last_written_slot = Clock::get()?.slot;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        let cpi_inputs = CpiInputs::new(
            proof,
            vec![user_record.to_account_info().map_err(ProgramError::from)?],
        );

        cpi_inputs
            .invoke_light_system_program(light_cpi_accounts)
            .map_err(ProgramError::from)?;

        emit!(UserRecordUpdated {
            user,
            new_score: user_record.score,
            updated_at: user_record.updated_at,
        });

        Ok(())
    }

    /// Decompresses a user record to a regular PDA (for compatibility or migration)
    pub fn decompress_user_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressUserRecord<'info>>,
        proof: ValidityProof,
        account_meta: CompressedAccountMeta,
        compressed_record: UserRecord,
    ) -> Result<()> {
        let user = ctx.accounts.user.key();

        // Verify ownership
        require!(compressed_record.owner == user, ErrorCode::ConstraintOwner);

        let compressed_account =
            LightAccount::<'_, UserRecord>::new_mut(&crate::ID, &account_meta, compressed_record)
                .map_err(ProgramError::from)?;

        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        // Use the SDK helper for idempotent decompression
        decompress_idempotent::<UserRecord>(
            &ctx.accounts.user_record_pda.to_account_info(),
            compressed_account,
            proof,
            light_cpi_accounts,
            &crate::ID,
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            Clock::get()?.slot,
            Rent::get()?.minimum_balance(std::mem::size_of::<UserRecordPda>() + 8),
        )
        .map_err(|_| error!(ErrorCode::CompressionError))?;

        emit!(UserRecordDecompressed {
            user,
            pda: ctx.accounts.user_record_pda.key(),
        });

        Ok(())
    }

    /// Compresses an existing PDA back to a compressed account
    pub fn compress_user_record_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressUserRecordPda<'info>>,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.fee_payer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        // Use the SDK helper to compress the PDA
        compress_pda::<UserRecord>(
            &ctx.accounts.user_record_pda.to_account_info(),
            &compressed_account_meta,
            proof,
            light_cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient.to_account_info(),
            Clock::get()?.slot,
        )
        .map_err(|_| error!(ErrorCode::CompressionError))?;

        emit!(UserRecordCompressed {
            user: ctx.accounts.user_record_pda.owner,
            pda: ctx.accounts.user_record_pda.key(),
        });

        Ok(())
    }

    /// Compresses a PDA into a new compressed account with a specific address
    pub fn compress_user_record_pda_new<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressUserRecordPdaNew<'info>>,
        proof: ValidityProof,
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let light_cpi_accounts = CpiAccounts::new(
            ctx.accounts.fee_payer.as_ref(),
            ctx.remaining_accounts,
            crate::LIGHT_CPI_SIGNER,
        );

        // Derive the address for the compressed account
        let (address, address_seed) = derive_address(
            &[b"user_record", ctx.accounts.user_record_pda.owner.as_ref()],
            &address_tree_info
                .get_tree_pubkey(&light_cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
            &crate::ID,
        );

        let new_address_params = address_tree_info.into_new_address_params_packed(address_seed);

        // Use the SDK helper to compress the PDA into a new compressed account
        compress_pda_new::<UserRecord>(
            &ctx.accounts.user_record_pda.to_account_info(),
            address,
            new_address_params,
            output_state_tree_index,
            proof,
            light_cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient.to_account_info(),
            &address_tree_info
                .get_tree_pubkey(&light_cpi_accounts)
                .map_err(|_| ErrorCode::AccountNotEnoughKeys)?,
            Clock::get()?.slot,
        )
        .map_err(|_| error!(ErrorCode::CompressionError))?;

        emit!(UserRecordCompressedNew {
            user: ctx.accounts.user_record_pda.owner,
            pda: ctx.accounts.user_record_pda.key(),
            compressed_address: address,
        });

        Ok(())
    }
}

/// Compressed user record that lives in the compressed state
#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct UserRecord {
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub bio: String,
    pub score: i64,
    pub created_at: i64,
    pub updated_at: i64,
    // PDA timing data for compression/decompression
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}

// Implement the PdaTimingData trait for UserRecord
impl PdaTimingData for UserRecord {
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

/// Regular on-chain PDA for decompressed records
#[account]
pub struct UserRecordPda {
    pub owner: Pubkey,
    pub name: String,
    pub bio: String,
    pub score: i64,
    pub created_at: i64,
    pub updated_at: i64,
    // PDA timing data
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}

#[derive(Accounts)]
pub struct CreateUserRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateUserRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct DecompressUserRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 4 + 64 + 4 + 256 + 8 + 8 + 8 + 8 + 8, // discriminator + owner + string lens + strings + timestamps + timing
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record_pda: Account<'info, UserRecordPda>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CompressUserRecordPda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user_record_pda.owner.as_ref()],
        bump,
        constraint = user_record_pda.owner == fee_payer.key(),
    )]
    pub user_record_pda: Account<'info, UserRecordPda>,
    /// CHECK: Rent recipient can be any account
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CompressUserRecordPdaNew<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user_record_pda.owner.as_ref()],
        bump,
        constraint = user_record_pda.owner == fee_payer.key(),
    )]
    pub user_record_pda: Account<'info, UserRecordPda>,
    /// CHECK: Rent recipient can be any account
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

// Events
#[event]
pub struct UserRecordCreated {
    pub user: Pubkey,
    pub address: [u8; 32],
    pub name: String,
}

#[event]
pub struct UserRecordUpdated {
    pub user: Pubkey,
    pub new_score: i64,
    pub updated_at: i64,
}

#[event]
pub struct UserRecordDecompressed {
    pub user: Pubkey,
    pub pda: Pubkey,
}

#[event]
pub struct UserRecordCompressed {
    pub user: Pubkey,
    pub pda: Pubkey,
}

#[event]
pub struct UserRecordCompressedNew {
    pub user: Pubkey,
    pub pda: Pubkey,
    pub compressed_address: [u8; 32],
}

// Error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Compression operation failed")]
    CompressionError,
}
