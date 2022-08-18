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
pub mod instructions;
pub use instructions::*;
pub mod errors;
pub use errors::*;
pub mod utils;

use crate::poseidon_merkle_tree::update_merkle_tree_lib::merkle_tree_update_state::MerkleTreeUpdateState;

use crate::utils::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY};

use crate::instructions::{
    insert_nullifier::InitializeNullifier,
    sol_transfer::{process_sol_transfer, WithdrawSol},
    spl_transfer::{process_spl_transfer, WithdrawSpl},
};
use crate::utils::config;

use crate::poseidon_merkle_tree::{
    check_merkle_root_exists::process_check_merkle_root_exists,
    initialize_new_merkle_tree_sol::{
        process_initialize_new_merkle_tree_sol,
        InitializeNewMerkleTreeSol
    },
    initialize_new_merkle_tree_spl::{
        process_initialize_new_merkle_tree_spl,
        InitializeNewMerkleTreeSpl
    },
    initialize_update_state::{process_initialize_update_state, InitializeUpdateState},
    insert_root::{process_insert_root, InsertRoot},
    insert_two_leaves::{process_insert_two_leaves, InsertTwoLeaves},
    update_merkle_tree::{process_update_merkle_tree, UpdateMerkleTree},
};

#[program]
pub mod merkle_tree_program {
    use super::*;

    /// Initializes a new Merkle tree from config bytes.
    /// Can only be called from the init authority.
    pub fn initialize_new_merkle_tree_sol(ctx: Context<InitializeNewMerkleTreeSol>) -> Result<()> {
        let merkle_tree_storage_acc = ctx.accounts.merkle_tree.to_account_info();
        let rent = Rent::get()?;

        if !rent.is_exempt(
            **merkle_tree_storage_acc.lamports.borrow(),
            merkle_tree_storage_acc.data.borrow().len(),
        ) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::AccountNotRentExempt.try_into().unwrap());
        }
        process_initialize_new_merkle_tree_sol(
            merkle_tree_storage_acc,
            &config::INIT_BYTES_MERKLE_TREE_18[..],
        )

    }

    /// Initializes a new Merkle tree from config bytes.
    /// Can only be called from the init authority.
    pub fn initialize_new_merkle_tree_spl(ctx: Context<InitializeNewMerkleTreeSpl>) -> Result<()> {
        let merkle_tree_storage_acc = ctx.accounts.merkle_tree.to_account_info();

        process_initialize_new_merkle_tree_spl(
            merkle_tree_storage_acc,
            &config::INIT_BYTES_MERKLE_TREE_18[..],
        )

    }

    /// Initializes a merkle tree update state pda. This pda stores the leaves to be inserted
    /// and state of the computation of poseidon hashes to update the Merkle tree.
    /// A maximum of 16 pairs of leaves can be passed in as leaves accounts as remaining accounts.
    /// Every leaf is copied into this account such that no further accounts or data have to be
    /// passed in during the following instructions.
    /// The hashes are computed with the update merkle tree instruction and the new root is inserted
    /// with the insert root merkle tree instruction.
    pub fn initialize_merkle_tree_update_state(
        ctx: Context<InitializeUpdateState>,
        merkle_tree_index: u64,
    ) -> Result<()> {
        process_initialize_update_state(ctx, merkle_tree_index)
    }

    /// Computes poseidon hashes to update the Merkle tree.
    pub fn update_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, UpdateMerkleTree<'info>>,
        _bump: u64, //data: Vec<u8>,
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
        _index: u64,
        leaf_left: [u8; 32],
        leaf_right: [u8; 32],
        encrypted_utxos: Vec<u8>,
        nullifier: [u8; 32],
        merkle_tree_pda_pubkey: [u8; 32],
    ) -> Result<()> {
        process_insert_two_leaves(
            ctx,
            leaf_left,
            leaf_right,
            encrypted_utxos,
            nullifier,
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
        data: Vec<u8>,
        _verifier_index: u64,
        _merkle_tree_index: u64,
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

    /// Generates a leaves index account for already existing Merkle trees.
    /// Can only be called by the init authority.
    pub fn initialize_merkle_tree_leaves_index<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, InitializeMerkleTreeLeavesIndex<'info>>,
        _bump: u64,
    ) -> anchor_lang::Result<()> {
        Ok(())
    }

    /// Checks whether a passed in merkle root exists.
    /// Execution fails if root is not found.
    /// Can only be called from a registered verifier program.
    pub fn check_merkle_root_exists<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, CheckMerkleRootExists<'info>>,
        _verifer_index: u64,
        _merkle_tree_index: u64,
        merkle_root: [u8; 32],
    ) -> anchor_lang::Result<()> {
        msg!("Invoking check_merkle_root_exists");
        process_check_merkle_root_exists(
            &ctx.accounts.merkle_tree.to_account_info(),
            &merkle_root.to_vec(),
            &ctx.program_id
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(verifer_index: u64, merkle_tree_index: u64)]
pub struct CheckMerkleRootExists<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut, address=anchor_lang::prelude::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[usize::try_from(verifer_index).unwrap()]))]
    pub authority: Signer<'info>,
    /// CHECK:` that the merkle tree is whitelisted.
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(merkle_tree_index).unwrap()].0))]
    pub merkle_tree: AccountInfo<'info>,
}

// This is a helper instruction to initialize the leaves index for existing
// merkle trees.
#[derive(Accounts)]
pub struct InitializeMerkleTreeLeavesIndex<'info> {
    #[account(mut, address= Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    // /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(
        init,
        payer = authority,
        seeds = [&merkle_tree.key().to_bytes()],
        bump,
        space = 16,
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK:` that this function can only be used to create this account for the existing sol
    /// Merkle tree.
    #[account(mut, address=Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[0].0))]
    pub merkle_tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
