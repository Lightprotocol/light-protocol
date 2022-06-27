use anchor_lang::prelude::*;

declare_id!("2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av");
use solana_program::program_pack::Pack;
use solana_program::{
    clock::Clock
};
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

use crate::errors::ErrorCode;

use anchor_lang::system_program;


use crate::poseidon_merkle_tree::update_merkle_tree_lib::update_state::MerkleTreeTmpPda;


use crate::utils::config::{
    MERKLE_TREE_ACC_BYTES_ARRAY,
    MERKLE_TREE_TMP_PDA_SIZE,
    MERKLE_TREE_INIT_AUTHORITY,
    ENCRYPTED_UTXOS_LENGTH,
};
use crate::utils::constants::{
    STORAGE_SEED,
    LEAVES_PDA_ACCOUNT_TYPE,
    UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE,
    NF_SEED,
};

use crate::utils::{
    create_pda::create_and_check_pda,
    config
};
use crate::instructions::{
    insert_nullifier::{
        InitializeNullifier
    },
    sol_transfer::{
        WithdrawSOL,
        process_sol_transfer
    }
};

use crate::poseidon_merkle_tree::{
    check_merkle_root_exists::{
        process_check_root_hash_exists
    },
    insert_root::{
        InsertRoot,
        process_insert_root
    },
    insert_two_leaves::{
        InsertTwoLeaves,
        process_insert_two_leaves
    },
    update_merkle_tree::{
        UpdateMerkleTree,
        process_update_merkle_tree
    },
    initialize_new_merkle_tree::{
        initialize_new_merkle_tree_from_bytes
    },
    initialize_update_state::{
        InitializeUpdateState,
        process_initialize_update_state
    },
};


#[program]
pub mod merkle_tree_program {
    use super::*;

    pub fn initialize_new_merkle_tree(ctx: Context<InitializeNewMerkleTree>) -> Result<()>{
        let merkle_tree_storage_acc = ctx.accounts.merkle_tree.to_account_info();
        let rent = Rent::get()?;

        if !rent.is_exempt(
            **merkle_tree_storage_acc.lamports.borrow(),
            merkle_tree_storage_acc.data.borrow().len(),
        ) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::AccountNotRentExempt.try_into().unwrap());
        }
        initialize_new_merkle_tree_from_bytes(merkle_tree_storage_acc, &config::INIT_BYTES_MERKLE_TREE_18[..])
    }

    pub fn initialize_merkle_tree_update_state(
        ctx: Context<InitializeUpdateState>,
        merkle_tree_index: u64
    ) -> Result<()>{
        process_initialize_update_state(
            ctx,
            merkle_tree_index
        )
    }

    pub fn update_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, UpdateMerkleTree<'info>>,
        _bump: u64//data: Vec<u8>,
    ) -> Result<()>{
        process_update_merkle_tree(
            &mut ctx
        )
    }

    pub fn insert_root_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, InsertRoot<'info>>,
        _bump: u64
    ) -> Result<()>{
        // doing checks after for mutability
        process_insert_root(&mut ctx)?;

        let tmp_storage_pda = ctx.accounts.merkle_tree_tmp_storage.load_mut()?;

        msg!("inserting merkle tree root");
         if tmp_storage_pda.current_instruction_index != 56 {
             msg!("Wrong state instruction index should be 56 is {}", tmp_storage_pda.current_instruction_index);
        }

        // mark leaves as inserted
        // check that leaves are the same as in first tx
        for (index, account) in ctx.remaining_accounts.iter().enumerate() {
            msg!("Checking leaves pair {}", index);
            if index >= tmp_storage_pda.number_of_leaves.into() {
                msg!("Submitted to many remaining accounts {}", ctx.remaining_accounts.len());
                return err!(ErrorCode::WrongLeavesLastTx);
            }
            if tmp_storage_pda.leaves[index][0][..] != account.data.borrow()[10..42] {
                msg!("Wrong leaf in position {}", index);
                return err!(ErrorCode::WrongLeavesLastTx);
            }
            if account.data.borrow()[1] != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
                msg!("Leaf pda with address {:?} is already inserted", *account.key);
                return err!(ErrorCode::LeafAlreadyInserted);
            }
            // mark leaves pda as inserted
            account.data.borrow_mut()[1] = LEAVES_PDA_ACCOUNT_TYPE;
        }

        Ok(())
    }

    pub fn insert_two_leaves<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertTwoLeaves<'info>>,
        leaf_left: [u8;32],
        leaf_right: [u8;32],
        encrypted_utxos: Vec<u8>,
        nullifier: [u8;32],
        next_index: u64,
        merkle_tree_pda_pubkey: [u8;32]
    ) -> Result<()>{
        process_insert_two_leaves(
            ctx,
            leaf_left,
            leaf_right,
            encrypted_utxos,
            nullifier,
            next_index,
            merkle_tree_pda_pubkey
        )
    }
    /*pub fn deposit_sol(ctx: Context<DepositSOL>, data: Vec<u8>) -> Result<()>{
        let mut new_data = data.clone();
        new_data.insert(0, 1);
        process_sol_transfer(
            ctx.program_id,
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.tmp_storage.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.merkle_tree_token.to_account_info(),
                ctx.accounts.user_escrow.to_account_info(),
            ],
            &new_data.as_slice()
        )?;
        Ok(())
    }*/
    pub fn withdraw_sol<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, WithdrawSOL<'info>>,
        data: Vec<u8>,
    ) -> Result<()>{
        let mut new_data = data.clone();
        new_data.insert(0, 2);

        let mut accounts = ctx.remaining_accounts.to_vec();
        accounts.insert(0, ctx.accounts.authority.to_account_info());
        accounts.insert(1, ctx.accounts.merkle_tree_token.to_account_info());

        process_sol_transfer(
            ctx.program_id,
            &accounts.as_slice(),
            &new_data.as_slice(),
        )?;
        Ok(())
    }
    /*
    pub fn create_authority_config(ctx: Context<CreateAuthorityConfig>) -> Result<()>{
        ctx.accounts
            .handle(*ctx.bumps.get("authority_config").unwrap())
    }
    pub fn update_authority_config(
        ctx: Context<UpdateAuthorityConfig>,
        new_authority: Pubkey,
    ) -> Result<()>{
        ctx.accounts.handle(new_authority)
    }

    pub fn register_new_id(ctx: Context<RegisterNewId>) -> Result<()>{
        ctx.accounts.handle(*ctx.bumps.get("registry").unwrap())
    }
    */
    pub fn initialize_nullifier(
        _ctx: Context<InitializeNullifier>,
        _nullifier: [u8; 32],
    ) -> anchor_lang::Result<()>{
        Ok(())
    }

    pub fn initialize_merkle_tree_leaves_index<'a, 'b, 'c, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, InitializeMerkleTreeLeavesIndex<'info>>, _bump: u64
    ) -> anchor_lang::Result<()>{
        Ok(())
    }

    pub fn check_root_hash_exists<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, CheckMerkleRootExists<'info>>,
        merkle_tree_index: u64,
        root_hash: [u8;32]
    ) -> anchor_lang::Result<()>{
        msg!("Invoking check_root_hash_exists");
        process_check_root_hash_exists(
            &ctx.accounts.merkle_tree.to_account_info(),
            &root_hash.to_vec(),
            &ctx.program_id,
            &Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index as usize].0)
        );
        Ok(())
    }
}
#[derive(Accounts)]
#[instruction(merkle_tree_index: u64)]
pub struct CheckMerkleRootExists<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[0]))]
    pub authority: Signer<'info>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_tmp_storage
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

// This is a helper instruction to initialize the leaves index for existing
// merkle trees.
#[derive(Accounts)]
pub struct InitializeMerkleTreeLeavesIndex<'info> {
    #[account(mut)]
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
    pub rent: Sysvar<'info, Rent>
}









/*
// deposits are currently implemented in the verifier program
#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY))]
    pub authority: Signer<'info>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub tmp_storage: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,

    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub user_escrow: AccountInfo<'info>,
}*/







/*
// not used right now because already inited merkle tree would not be compatible
#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct InitializeLeavesPda<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECKS should be, address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(address=system_program::ID)]
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// not used right now because already inited merkle tree would not be compatible
#[account(zero_copy)]
pub struct LeavesPda {
    pub leaf_right: [u8; 32],
    pub leaf_left: [u8; 32],
    pub merkle_tree_pubkey: Pubkey,
    pub encrypted_utxos: [u8; 222],
    pub left_leaf_index: u64,
}
*/
