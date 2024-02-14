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
        // Index of the Merkle tree changelog.
        changelog_index: u16,
        // Index of the address to dequeue.
        queue_index: u16,
        // Index of the next address.
        address_next_index: usize,
        // Value of the next address.
        address_next_value: [u8; 32],
        // Low address.
        low_address: RawIndexingElement<usize, 32>,
        // Value of the next address.
        low_address_next_value: [u8; 32],
        // Merkle proof for updating the low address.
        low_address_proof: [[u8; 32]; 22],
        // ZK proof for integrity of provided `address_next_index` and
        // `address_next_value`.
        next_address_proof: [u8; 128],
    ) -> Result<()> {
        process_update_address_merkle_tree(
            ctx,
            changelog_index,
            queue_index,
            address_next_index,
            address_next_value,
            low_address,
            low_address_next_value,
            low_address_proof,
            next_address_proof,
        )
    }
}
