use anchor_lang::prelude::*;

declare_id!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");

use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_merkle_tree",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}

pub mod poseidon_merkle_tree;
pub use poseidon_merkle_tree::*;
pub mod verifier_invoked_instructions;
pub use verifier_invoked_instructions::*;
pub mod errors;
pub use errors::*;
pub mod utils;

pub mod config_accounts;
pub use config_accounts::*;

use crate::config_accounts::{
    init_asset_pda::{RegisterSolPool, RegisterSplPool},
    merkle_tree_authority::{
        InitializeMerkleTreeAuthority, UpdateMerkleTreeAuthority, UpdateMerkleTreeAuthorityConfig,
    },
    register_verifier::RegisterVerifier,
};

use crate::errors::ErrorCode;

use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;

use crate::utils::{
    config::{self, ZERO_BYTES_MERKLE_TREE_18}
};

use crate::verifier_invoked_instructions::{
    insert_nullifier::{process_insert_nullifiers, InitializeNullifiers},
    insert_two_leaves::{process_insert_two_leaves, InsertTwoLeaves},
    sol_transfer::{process_sol_transfer, WithdrawSol},
    spl_transfer::{process_spl_transfer, WithdrawSpl},
};

use crate::poseidon_merkle_tree::{
    initialize_new_merkle_tree_18::{
        process_initialize_new_merkle_tree_18, InitializeNewMerkleTree,
    },
    update_instructions::{
        initialize_update_state::{process_initialize_update_state, InitializeUpdateState},
        insert_root::{process_insert_root, InsertRoot},
        update_merkle_tree::{process_update_merkle_tree, UpdateMerkleTree},
    },
};

#[program]
pub mod merkle_tree_program {
    use super::*;

    /// Initializes a new Merkle tree from config bytes.
    /// Can only be called from the merkle_tree_authority.
    pub fn initialize_new_merkle_tree(
        ctx: Context<InitializeNewMerkleTree>,
        lock_duration: u64,
    ) -> Result<()> {
        let merkle_tree = &mut ctx.accounts.merkle_tree.load_init()?;

        let merkle_tree_index = ctx.accounts.merkle_tree_authority_pda.merkle_tree_index;
        process_initialize_new_merkle_tree_18(
            merkle_tree,
            18,
            ZERO_BYTES_MERKLE_TREE_18.to_vec(),
            merkle_tree_index,
        );

        ctx.accounts.merkle_tree_authority_pda.merkle_tree_index += 1;
        merkle_tree.lock_duration = lock_duration;

        Ok(())
    }

    /// Initializes a new merkle tree authority which can register new verifiers and configure
    /// permissions to create new pools.
    pub fn initialize_merkle_tree_authority(
        ctx: Context<InitializeMerkleTreeAuthority>,
    ) -> Result<()> {
        ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.authority.key();
        Ok(())
    }

    /// Updates the merkle tree authority to a new authority.
    pub fn update_merkle_tree_authority(ctx: Context<UpdateMerkleTreeAuthority>) -> Result<()> {
        // account is checked in ctx struct
        ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.new_authority.key();
        Ok(())
    }

    /// Updates the lock duration for a specific merkle tree.
    pub fn update_lock_duration(
        ctx: Context<UpdateLockDuration>,
        lock_duration: u64,
    ) -> Result<()> {
        ctx.accounts.merkle_tree.load_mut()?.lock_duration = lock_duration;
        Ok(())
    }

    /// Enables permissionless deposits of any spl token with supply of one and zero decimals.
    pub fn enable_nfts(
        ctx: Context<UpdateMerkleTreeAuthorityConfig>,
        enable_permissionless: bool,
    ) -> Result<()> {
        ctx.accounts.merkle_tree_authority_pda.enable_nfts = enable_permissionless;
        Ok(())
    }

    /// Enables anyone to create token pools.
    pub fn enable_permissionless_spl_tokens(
        ctx: Context<UpdateMerkleTreeAuthorityConfig>,
        enable_permissionless: bool,
    ) -> Result<()> {
        ctx.accounts
            .merkle_tree_authority_pda
            .enable_permissionless_spl_tokens = enable_permissionless;
        Ok(())
    }

    // Unactivated feature listed for completeness.
    // pub fn enable_permissionless_merkle_tree_registration(ctx: Context<UpdateMerkleTreeAuthorityConfig>, enable_permissionless: bool) -> Result<()> {
    //     ctx.accounts.merkle_tree_authority_pda.enable_permissionless_merkle_tree_registration = enable_permissionless;
    //     Ok(())
    // }

    /// Registers a new verifier which can withdraw tokens, insert new nullifiers, add new leaves.
    /// These functions can only be invoked from registered verifiers.
    pub fn register_verifier(
        ctx: Context<RegisterVerifier>,
        verifier_pubkey: Pubkey,
    ) -> Result<()> {
        if !ctx
            .accounts
            .merkle_tree_authority_pda
            .enable_permissionless_merkle_tree_registration
            && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
        {
            return err!(ErrorCode::InvalidAuthority);
        }
        ctx.accounts.registered_verifier_pda.pubkey = verifier_pubkey;
        Ok(())
    }

    /// Registers a new pooltype.
    pub fn register_pool_type(ctx: Context<RegisterPoolType>, pool_type: [u8; 32]) -> Result<()> {
        if !ctx
            .accounts
            .merkle_tree_authority_pda
            .enable_permissionless_spl_tokens
            && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
        {
            return err!(ErrorCode::InvalidAuthority);
        }
        ctx.accounts.registered_pool_type_pda.pool_type = pool_type;
        Ok(())
    }

    /// Creates a new spl token pool which can be used by any registered verifier.
    pub fn register_spl_pool(ctx: Context<RegisterSplPool>) -> Result<()> {

        let is_nft = false;
            // ctx.accounts.mint.decimals == 0
            // && ctx.accounts.mint.supply == 1
            // should add check that authority is metaplex nft
            // && metaplex_token_metadata::state::get_master_edition(&ctx.accounts.metaplex_token.to_account_info()).is_ok();
        msg!("is_nft {}", is_nft);
        // nfts enabled
        if is_nft && !ctx.accounts.merkle_tree_authority_pda.enable_nfts && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
            return err!(ErrorCode::InvalidAuthority);
        }

        // any token enabled
        if !is_nft
            && !ctx
                .accounts
                .merkle_tree_authority_pda
                .enable_permissionless_spl_tokens
            && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);

        }

        ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey =
            ctx.accounts.merkle_tree_pda_token.key();
        ctx.accounts.registered_asset_pool_pda.pool_type =
            ctx.accounts.registered_pool_type_pda.pool_type;
        ctx.accounts.registered_asset_pool_pda.index = ctx
            .accounts
            .merkle_tree_authority_pda
            .registered_asset_index;
        ctx.accounts
            .merkle_tree_authority_pda
            .registered_asset_index += 1;
        Ok(())
    }

    /// Creates a new sol pool which can be used by any registered verifier.
    pub fn register_sol_pool(ctx: Context<RegisterSolPool>) -> Result<()> {
        if !ctx
            .accounts
            .merkle_tree_authority_pda
            .enable_permissionless_spl_tokens
            && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);
        }

        ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey =
            ctx.accounts.registered_asset_pool_pda.key();
        ctx.accounts.registered_asset_pool_pda.pool_type =
            ctx.accounts.registered_pool_type_pda.pool_type;
        ctx.accounts.registered_asset_pool_pda.index = ctx
            .accounts
            .merkle_tree_authority_pda
            .registered_asset_index;
        ctx.accounts
            .merkle_tree_authority_pda
            .registered_asset_index += 1;
        Ok(())
    }

    /// Initializes a merkle tree update state pda. This pda stores the leaves to be inserted
    /// and state of the computation of poseidon hashes to update the Merkle tree.
    /// A maximum of 16 pairs of leaves can be passed in as leaves accounts as remaining accounts.
    /// Every leaf is copied into this account such that no further accounts or data have to be
    /// passed in during the following instructions which compute the poseidon hashes to update the tree.
    /// The hashes are computed with the update merkle tree instruction and the new root is inserted
    /// with the insert root merkle tree instruction.
    pub fn initialize_merkle_tree_update_state<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeUpdateState<'info>>,
    ) -> Result<()> {
        process_initialize_update_state(ctx)
    }

    /// Computes poseidon hashes to update the Merkle tree.
    pub fn update_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, UpdateMerkleTree<'info>>,
        _bump: u64,
    ) -> Result<()> {
        process_update_merkle_tree(&mut ctx)
    }

    /// This is the last step of a Merkle tree update which inserts the prior computed Merkle tree
    /// root.
    pub fn insert_root_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, InsertRoot<'info>>,
        _bump: u64,
    ) -> Result<()> {
        process_insert_root(&mut ctx)
    }

    /// Closes the Merkle tree update state.
    /// A relayer can only close its own update state account.
    pub fn close_merkle_tree_update_state(
        _ctx: Context<CloseUpdateState>,
    ) -> anchor_lang::Result<()> {
        Ok(())
    }

    /// Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.
    /// The inserted leaves are not part of the Merkle tree yet and marked accordingly.
    /// The Merkle tree has to be updated after.
    /// Can only be called from a registered verifier program.
    pub fn insert_two_leaves<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertTwoLeaves<'info>>,
        leaf_left: [u8; 32],
        leaf_right: [u8; 32],
        encrypted_utxo: [u8; 256],
        merkle_tree_pda_pubkey: Pubkey,
    ) -> Result<()> {
        process_insert_two_leaves(
            ctx,
            leaf_left,
            leaf_right,
            encrypted_utxo,
            merkle_tree_pda_pubkey,
        )
    }

    /// Withdraws sol from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn withdraw_sol<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSol<'info>>,
        amount: u64,
    ) -> Result<()> {
        msg!("withdraw_sol");
        process_sol_transfer(
            &ctx.accounts.merkle_tree_token.to_account_info(),
            &ctx.accounts.recipient.to_account_info(),
            amount,
        )
    }

    /// Withdraws spl tokens from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn withdraw_spl<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSpl<'info>>,
        amount: u64,
    ) -> Result<()> {
        process_spl_transfer(ctx, amount)
    }

    pub fn initialize_nullifiers<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeNullifiers<'info>>,
        nullifiers: Vec<Vec<u8>>,
    ) -> Result<()> {
        process_insert_nullifiers(ctx, nullifiers)
    }
}
