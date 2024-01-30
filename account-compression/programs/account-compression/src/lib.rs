use anchor_lang::prelude::*;
use light_indexed_merkle_tree::array::RawIndexingElement;

pub mod errors;
pub mod instructions;
use instructions::*;
pub mod state;
use state::*;

declare_id!("5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN");

#[program]
pub mod address {

    use super::*;

    pub fn initialize_address_queue(_ctx: Context<InitializeAddressQueue>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTree<'info>>,
    ) -> Result<()> {
        process_initialize_address_merkle_tree(ctx)
    }

    pub fn insert_addresses<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertAddresses<'info>>,
        addresses: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_addresses(ctx, addresses)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
        changelog_index: u16,
        queue_index: u16,
        address_index: u16,
        address_next_index: u16,
        address_next_value: [u8; 32],
        low_address: RawIndexingElement<32>,
        low_address_next_value: [u8; 32],
        low_address_proof: [[u8; 32]; 22],
    ) -> Result<()> {
        process_update_address_merkle_tree(
            ctx,
            changelog_index,
            queue_index,
            address_index,
            address_next_index,
            address_next_value,
            low_address,
            low_address_next_value,
            low_address_proof,
        )
    }
}
