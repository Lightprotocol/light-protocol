use anchor_lang::prelude::*;
use light_sdk::{
    compressible::PdaTimingData,
    cpi::CpiAccounts,
    instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo, ValidityProof},
    light_hasher::{DataHasher, Hasher},
};
use light_sdk::{derive_light_cpi_signer, LightDiscriminator, LightHasher};
use light_sdk_types::CpiAccountsConfig;
use light_sdk_types::CpiSigner;

declare_id!("CompUser11111111111111111111111111111111111");
pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible_user {

    use light_sdk::account::LightAccount;
    use light_sdk::compressible::{compress_pda, compress_pda_new, decompress_multiple_idempotent};

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

    /// Can be the same because the PDA will be decompressed in a separate instruction.
    /// Updates an existing user record
    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        Ok(())
    }

    /// Decompresses multiple compressed PDAs of any supported account type in a single transaction
    pub fn decompress_multiple_pdas<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressMultiplePdas<'info>>,
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        // Get PDA accounts from remaining accounts
        let pda_accounts_end = system_accounts_offset as usize;
        let pda_accounts = &ctx.remaining_accounts[..pda_accounts_end];

        // Validate we have matching number of PDAs and compressed accounts
        if pda_accounts.len() != compressed_accounts.len() {
            return err!(ErrorCode::InvalidAccountCount);
        }

        // Set up CPI accounts
        let config = CpiAccountsConfig::new(LIGHT_CPI_SIGNER);
        let cpi_accounts = CpiAccounts::new_with_config(
            &ctx.accounts.fee_payer,
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            config,
        );

        // Convert to unified enum accounts
        let mut light_accounts = Vec::new();
        let mut pda_account_refs = Vec::new();

        for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
            // Convert to unified enum type
            let unified_account = match compressed_data.data {
                CompressedAccountVariant::UserRecord(data) => {
                    CompressedAccountVariant::UserRecord(data)
                }
                CompressedAccountVariant::GameSession(data) => {
                    CompressedAccountVariant::GameSession(data)
                }
            };

            let light_account = LightAccount::<'_, CompressedAccountVariant>::new_mut(
                &crate::ID,
                &compressed_data.meta,
                unified_account,
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            light_accounts.push(light_account);
            pda_account_refs.push(&pda_accounts[i]);
        }

        // Single CPI call with unified enum type
        decompress_multiple_idempotent::<CompressedAccountVariant>(
            &pda_account_refs,
            light_accounts,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_payer,
            &ctx.accounts.system_program.to_account_info(),
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

        Ok(())
    }

    pub fn compress_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressRecord<'info>>,
        name: String,
        score: u64,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
        let cpi_accounts =
            CpiAccounts::new_with_config(&ctx.accounts.user, &ctx.remaining_accounts[..], config);

        compress_pda::<UserRecord>(
            &user_record.to_account_info(),
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
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
        space = 8 + 32 + 4 + 32 + 8, // discriminator + owner + string len + name + score
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
pub struct CompressRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = user_record.owner == user.key()
    )]
    pub user_record: Account<'info, UserRecord>,
    pub system_program: Program<'info, System>,
    #[account(address = RENT_RECIPIENT)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DecompressMultiplePdas<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    // Remaining accounts:
    // - First N accounts: PDA accounts to decompress into
    // - After system_accounts_offset: Light Protocol system accounts for CPI
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

impl PdaTimingData for CompressedAccountVariant {
    fn last_written_slot(&self) -> u64 {
        match self {
            Self::UserRecord(data) => data.last_written_slot(),
            Self::GameSession(data) => data.last_written_slot(),
        }
    }

    fn slots_until_compression(&self) -> u64 {
        match self {
            Self::UserRecord(data) => data.slots_until_compression(),
            Self::GameSession(data) => data.slots_until_compression(),
        }
    }

    fn set_last_written_slot(&mut self, slot: u64) {
        match self {
            Self::UserRecord(data) => data.set_last_written_slot(slot),
            Self::GameSession(data) => data.set_last_written_slot(slot),
        }
    }
}

/// Client-side data structures
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMeta,
    pub data: CompressedAccountVariant,
}

#[derive(Default, Debug, LightHasher, LightDiscriminator)]
#[account]
pub struct UserRecord {
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}

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

#[derive(Default, Debug, LightHasher, LightDiscriminator)]
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

impl PdaTimingData for GameSession {
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid account count: PDAs and compressed accounts must match")]
    InvalidAccountCount,
}
