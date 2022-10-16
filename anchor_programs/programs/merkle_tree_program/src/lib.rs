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
    init_asset_pda::{
        RegisterSplPool,
        RegisterSolPool,
    },
    merkle_tree_authority::{
        InitializeMerkleTreeAuthority,
        UpdateMerkleTreeAuthority
    },
    register_verifier::RegisterVerifier,

};

use crate::errors::ErrorCode;

use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;

use crate::utils::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY, ZERO_BYTES_MERKLE_TREE_18};

use crate::verifier_invoked_instructions::{
    insert_nullifier::InitializeNullifier,
    sol_transfer::{process_sol_transfer, WithdrawSol},
    spl_transfer::{process_spl_transfer, WithdrawSpl},
    insert_two_leaves::{process_insert_two_leaves, InsertTwoLeaves},
};
use crate::utils::config;

use crate::poseidon_merkle_tree::{
    // check_merkle_root_exists::process_check_merkle_root_exists,
    initialize_new_merkle_tree_18::{
        process_initialize_new_merkle_tree_18,
        InitializeNewMerkleTree
    },
    // initialize_new_merkle_tree_spl::{
    //     process_initialize_new_merkle_tree_spl,
    //     InitializeNewMerkleTreeSpl
    // },
    update_instructions:: {
        initialize_update_state::{process_initialize_update_state, InitializeUpdateState},
        insert_root::{process_insert_root, InsertRoot},
        update_merkle_tree::{process_update_merkle_tree, UpdateMerkleTree},
    }
};

#[program]
pub mod merkle_tree_program {
    use super::*;

    /// Initializes a new Merkle tree from config bytes.
    /// Can only be called from the merkle_tree_authority.
    pub fn initialize_new_merkle_tree(ctx: Context<InitializeNewMerkleTree>) -> Result<()> {
        let merkle_tree = &mut ctx.accounts.merkle_tree.load_init()?;

        let merkle_tree_index = ctx.accounts.merkle_tree_authority_pda.merkle_tree_index;
        process_initialize_new_merkle_tree_18(merkle_tree,18, ZERO_BYTES_MERKLE_TREE_18.to_vec(), merkle_tree_index);

        ctx.accounts.merkle_tree_authority_pda.merkle_tree_index += 1;

        Ok(())

    }

    /// Initializes a new merkle tree authority which can register new verifiers and configure
    /// permissions to create new pools.
    pub fn initialize_merkle_tree_authority(ctx: Context<InitializeMerkleTreeAuthority>) -> Result<()> {
        ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.new_authority.key();
        Ok(())
    }

    pub fn update_merkle_tree_authority(
        ctx: Context<UpdateMerkleTreeAuthority>,
        enable_nfts: bool,
        enable_permissionless_spl_tokens: bool,
        enable_permissionless_merkle_tree_registration: bool
    ) -> Result<()> {
        // account is checked in ctx struct
        ctx.accounts.merkle_tree_authority_pda.pubkey = ctx.accounts.new_authority.key();
        // ctx.accounts.merkle_tree_authority_pda.enable_nfts = enable_nfts;
        // ctx.accounts.merkle_tree_authority_pda.enable_permissionless_spl_tokens = enable_permissionless_spl_tokens;
        // ctx.accounts.merkle_tree_authority_pda.enable_permissionless_merkle_tree_registration = enable_permissionless_merkle_tree_registration;

        Ok(())
    }

    /// Registers a new verifier which can withdraw tokens, insert new nullifiers, add new leaves.
    /// These functions can only be invoked from registered verifiers.
    pub fn register_verifier(
        ctx: Context<RegisterVerifier>,
        verifier_pubkey: Pubkey
    ) -> Result<()> {
        if !ctx.accounts.merkle_tree_authority_pda.enable_permissionless_merkle_tree_registration {
            if ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);
            }
        }
        ctx.accounts.registered_verifier_pda.pubkey = verifier_pubkey;
        Ok(())
    }

    /// Registers a new pooltype.
    pub fn register_pool_type(
        ctx: Context<RegisterPoolType>,
        pool_type: [u8;32]
    ) -> Result<()> {
        if !ctx.accounts.merkle_tree_authority_pda.enable_permissionless_spl_tokens {
            if ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);
            }
        }
        ctx.accounts.registered_pool_type_pda.pool_type = pool_type;
        Ok(())
    }

    /// Creates a new spl token pool which can be used by any registered verifier.
    pub fn register_spl_pool(
        ctx: Context<RegisterSplPool>
    ) -> Result<()> {
        if !ctx.accounts.merkle_tree_authority_pda.enable_permissionless_spl_tokens {
            if ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);
            }
        }
        ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey = ctx.accounts.merkle_tree_pda_token.key();
        ctx.accounts.registered_asset_pool_pda.pool_type = ctx.accounts.registered_pool_type_pda.pool_type;
        Ok(())
    }

    /// Creates a new sol pool which can be used by any registered verifier.
    pub fn register_sol_pool(
        ctx: Context<RegisterSolPool>
    ) -> Result<()> {
        if !ctx.accounts.merkle_tree_authority_pda.enable_permissionless_spl_tokens {
            if ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey {
                return err!(ErrorCode::InvalidAuthority);
            }
        }

        ctx.accounts.registered_asset_pool_pda.asset_pool_pubkey = ctx.accounts.registered_asset_pool_pda.key();
        ctx.accounts.registered_asset_pool_pda.pool_type = ctx.accounts.registered_pool_type_pda.pool_type;
        Ok(())
    }

    /// Initializes a merkle tree update state pda. This pda stores the leaves to be inserted
    /// and state of the computation of poseidon hashes to update the Merkle tree.
    /// A maximum of 16 pairs of leaves can be passed in as leaves accounts as remaining accounts.
    /// Every leaf is copied into this account such that no further accounts or data have to be
    /// passed in during the following instructions.
    /// The hashes are computed with the update merkle tree instruction and the new root is inserted
    /// with the insert root merkle tree instruction.
    pub fn initialize_merkle_tree_update_state<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info,InitializeUpdateState<'info>>,
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
    /// root. Additionally, all inserted leaves are marked as inserted.
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
        encrypted_utxos: [u8;256],
        merkle_tree_pda_pubkey: Pubkey,
    ) -> Result<()> {
        process_insert_two_leaves(
            ctx,
            leaf_left,
            leaf_right,
            encrypted_utxos,
            merkle_tree_pda_pubkey,
        )
    }

    /// Withdraws sol from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn withdraw_sol<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSol<'info>>,
        data: Vec<u8>,
        _verifier_index: u64,
        _merkle_tree_index: u64,
    ) -> Result<()> {
        let mut accounts = ctx.remaining_accounts.to_vec();
        accounts.insert(0, ctx.accounts.merkle_tree_token.to_account_info());

        process_sol_transfer(&accounts.as_slice(), &data.as_slice())
    }

    /// Withdraws spl tokens from a liquidity pool.
    /// An arbitrary number of recipients can be passed in with remaining accounts.
    /// Can only be called from a registered verifier program.
    pub fn withdraw_spl<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSpl<'info>>,
        data: Vec<u8>
    ) -> Result<()> {
        process_spl_transfer(ctx, &data.as_slice())
    }


    /// Creates and initializes a nullifier pda.
    /// Can only be called from a registered verifier program.
    pub fn initialize_nullifier(
        _ctx: Context<InitializeNullifier>,
        _nullifier: [u8; 32],
        _index: u64,
    ) -> anchor_lang::Result<()> {
        Ok(())
    }

    // /// Generates a leaves index account for already existing Merkle trees.
    // /// Can only be called by the init authority.
    // pub fn initialize_merkle_tree_leaves_index<'a, 'b, 'c, 'info>(
    //     _ctx: Context<'a, 'b, 'c, 'info, InitializeMerkleTreeLeavesIndex<'info>>,
    //     _bump: u64,
    // ) -> anchor_lang::Result<()> {
    //     Ok(())
    // }

    // /// Checks whether a passed in merkle root exists.
    // /// Execution fails if root is not found.
    // /// Can only be called from a registered verifier program.
    // pub fn check_merkle_root_exists<'a, 'b, 'c, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, CheckMerkleRootExists<'info>>,
    //     _verifer_index: u64,
    //     _merkle_tree_index: u64,
    //     merkle_root: [u8; 32],
    // ) -> anchor_lang::Result<()> {
    //     msg!("Invoking check_merkle_root_exists");
    //     process_check_merkle_root_exists(
    //         &ctx.accounts.merkle_tree.to_account_info(),
    //         &merkle_root.to_vec(),
    //         &ctx.program_id
    //     )?;
    //     Ok(())
    // }
}

// This is a helper instruction to initialize the leaves index for existing
// merkle trees.
// #[derive(Accounts)]
// pub struct InitializeMerkleTreeLeavesIndex<'info> {
//     #[account(mut, address=)]
//     pub authority: Signer<'info>,
//     // /// CHECK:` doc comment explaining why no checks through types are necessary.
//     #[account(
//         init,
//         payer = authority,
//         seeds = [&merkle_tree.key().to_bytes()],
//         bump,
//         space = 16,
//     )]
//     pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
//     /// CHECK:` that this function can only be used to create this account for the existing sol
//     /// Merkle tree.
//     #[account(mut, address=Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[0].0))]
//     pub merkle_tree: AccountInfo<'info>,
//     pub system_program: Program<'info, System>,
//     pub rent: Sysvar<'info, Rent>,
// }
