#![allow(deprecated)]

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_sdk::derive_light_cpi_signer;
use light_sdk_types::CpiSigner;

pub mod constants;
pub mod errors;
pub mod instruction_accounts;
pub mod instructions;
pub mod seeds;
pub mod state;

pub use constants::*;
pub use errors::*;
pub use instruction_accounts::*;
// Re-export types needed by Anchor's macro expansion
pub use light_sdk::instruction::{
    account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAddressTreeInfo, ValidityProof,
};
pub use seeds::*;
pub use state::*;

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

#[program]
pub mod sdk_compressible_test {
    use light_sdk::instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAddressTreeInfo,
        ValidityProof,
    };

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        instructions::create_record::create_record(
            ctx,
            name,
            proof,
            compressed_address,
            address_tree_info,
            output_state_tree_index,
        )
    }

    pub fn create_user_record_and_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
        account_data: AccountCreationData,
        compression_params: CompressionParams,
    ) -> Result<()> {
        instructions::create_user_record_and_game_session::create_user_record_and_game_session(
            ctx,
            account_data,
            compression_params,
        )
    }

    pub fn update_game_session(
        ctx: Context<UpdateGameSession>,
        _session_id: u64,
        new_score: u64,
    ) -> Result<()> {
        instructions::update_game_session::update_game_session(ctx, _session_id, new_score)
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
        instructions::create_game_session::create_game_session(
            ctx,
            session_id,
            game_type,
            proof,
            compressed_address,
            address_tree_info,
            output_state_tree_index,
        )
    }

    pub fn initialize_compression_config(
        ctx: Context<InitializeCompressionConfig>,
        compression_delay: u32,
        rent_sponsor: Pubkey,
        address_space: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::initialize_compression_config::initialize_compression_config(
            ctx,
            compression_delay,
            rent_sponsor,
            address_space,
        )
    }

    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        instructions::update_record::update_record(ctx, name, score)
    }

    pub fn create_placeholder_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePlaceholderRecord<'info>>,
        placeholder_id: u64,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        instructions::create_placeholder_record::create_placeholder_record(
            ctx,
            placeholder_id,
            name,
            proof,
            compressed_address,
            address_tree_info,
            output_state_tree_index,
        )
    }

    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        instructions::decompress_accounts_idempotent::decompress_accounts_idempotent(
            ctx,
            proof,
            compressed_accounts,
            system_accounts_offset,
        )
    }

    pub fn compress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, 'info, 'info, CompressAccountsIdempotent<'info>>,
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
        signer_seeds: Vec<Vec<Vec<u8>>>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        instructions::compress_accounts_idempotent::compress_accounts_idempotent(
            ctx,
            proof,
            compressed_accounts,
            signer_seeds,
            system_accounts_offset,
        )
    }

    pub fn update_compression_config(
        ctx: Context<UpdateCompressionConfig>,
        new_compression_delay: Option<u32>,
        new_rent_sponsor: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        instructions::update_compression_config::update_compression_config(
            ctx,
            new_compression_delay,
            new_rent_sponsor,
            new_address_space,
            new_update_authority,
        )
    }
}
