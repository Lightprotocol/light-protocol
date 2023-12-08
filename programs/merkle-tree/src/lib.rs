use anchor_lang::prelude::*;

declare_id!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_protocol_merkle_tree",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}

pub mod instructions;
pub use instructions::*;
pub mod state;
pub use state::*;
pub mod verifier_invoked_instructions;
pub use verifier_invoked_instructions::*;
pub mod errors;
pub mod utils;

pub mod config_accounts;
pub use config_accounts::*;

use crate::utils::config;

#[program]
pub mod light_merkle_tree_program {
    use super::*;

    /// Initializes a new Merkle tree from config bytes.
    /// Can only be called from the merkle_tree_authority.
    pub fn initialize_new_merkle_tree_set(ctx: Context<InitializeNewMerkleTreeSet>) -> Result<()> {
        process_initialize_new_merkle_tree_set(ctx)
    }

    /// Initializes a new merkle tree authority which can register new verifiers and configure
    /// permissions to create new pools.
    pub fn initialize_merkle_tree_authority(
        mut ctx: Context<InitializeMerkleTreeAuthority>,
    ) -> Result<()> {
        process_initialize_merkle_tree_authority(&mut ctx)
    }

    /// Updates the merkle tree authority to a new authority.
    pub fn update_merkle_tree_authority(ctx: Context<UpdateMerkleTreeAuthority>) -> Result<()> {
        process_update_merkle_tree_authority(ctx)
    }

    /// Enables anyone to create token pools.
    pub fn enable_permissionless_spl_tokens(
        ctx: Context<UpdateMerkleTreeAuthorityConfig>,
        enable_permissionless: bool,
    ) -> Result<()> {
        process_enable_permissionless_spl_tokens(ctx, enable_permissionless)
    }

    // Unactivated feature listed for completeness.
    // pub fn enable_permissionless_merkle_tree_registration(ctx: Context<UpdateMerkleTreeAuthorityConfig>, enable_permissionless: bool) -> Result<()> {
    //     ctx.accounts.merkle_tree_authority_pda.enable_permissionless_merkle_tree_registration = enable_permissionless;
    //     Ok(())
    // }

    /// Registers a new verifier which can decompress tokens, insert new nullifiers, add new leaves.
    /// These functions can only be invoked from registered verifiers.
    pub fn register_verifier(
        ctx: Context<RegisterVerifier>,
        verifier_pubkey: Pubkey,
    ) -> Result<()> {
        process_register_verifier(ctx, verifier_pubkey)
    }

    /// Registers a new pooltype.
    pub fn register_pool_type(ctx: Context<RegisterPoolType>, pool_type: [u8; 32]) -> Result<()> {
        process_register_pool_type(ctx, pool_type)
    }

    /// Creates a new spl token pool which can be used by any registered verifier.
    pub fn register_spl_pool(ctx: Context<RegisterSplPool>) -> Result<()> {
        process_register_spl_pool(ctx)
    }

    /// Creates a new sol pool which can be used by any registered verifier.
    pub fn register_sol_pool(ctx: Context<RegisterSolPool>) -> Result<()> {
        process_register_sol_pool(ctx)
    }

    /// Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.
    /// The inserted leaves are not part of the Merkle tree yet and marked accordingly.
    /// The Merkle tree has to be updated after.
    /// Can only be called from a registered verifier program.
    pub fn insert_two_leaves<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertTwoLeaves<'info>>,
        leaves: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_two_leaves(ctx, &leaves)
    }

    // pub fn insert_two_leaves_parallel<'info>(
    //     ctx: Context<'_, '_, '_, 'info, InsertTwoLeavesParallel<'info>>,
    //     leaves: Vec<[u8; 32]>,
    // ) -> Result<()> {
    //     process_insert_two_leaves_parallel(ctx, &leaves)
    // }

    pub fn insert_two_leaves_event<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertTwoLeavesEvent<'info>>,
        leaf_left: [u8; 32],
        leaf_right: [u8; 32],
    ) -> Result<()> {
        process_insert_two_leaves_event(ctx, leaf_left, leaf_right)
    }

    /// Decompresses sol from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn decompress_sol<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressSol<'info>>,
        amount: u64,
    ) -> Result<()> {
        process_sol_transfer(
            &ctx.accounts.merkle_tree_token.to_account_info(),
            &ctx.accounts.recipient.to_account_info(),
            amount,
        )
    }

    /// Decompresses spl tokens from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn decompress_spl<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressSpl<'info>>,
        amount: u64,
    ) -> Result<()> {
        process_spl_transfer(ctx, amount)
    }

    pub fn initialize_nullifiers<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeNullifiers<'info>>,
        nullifiers: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_nullifiers(ctx, nullifiers)
    }
}
