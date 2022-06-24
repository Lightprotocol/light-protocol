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

// pub mod authority_config;
pub mod constant;
pub mod instructions;
pub mod poseidon_merkle_tree;
pub mod processor;
pub mod state;
pub mod utils;
pub mod wrapped_state;
// pub mod registry;
// pub use registry::*;

use crate::config::{
    MERKLE_TREE_TMP_PDA_SIZE,
    STORAGE_SEED,
    ENCRYPTED_UTXOS_LENGTH,
    MERKLE_TREE_INIT_AUTHORITY,
    LEAVES_PDA_ACCOUNT_TYPE,
    UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE,
    NF_SEED,
    MERKLE_TREE_ACC_BYTES_ARRAY
};
use crate::poseidon_merkle_tree::processor::pubkey_check;

pub use crate::constant::*;
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::utils::config;

use crate::state::MerkleTreeTmpPda;
use anchor_lang::system_program;


use crate::poseidon_merkle_tree::state::MerkleTree;
use crate::utils::create_pda::create_and_check_pda;
use crate::poseidon_merkle_tree::state::TwoLeavesBytesPda;


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
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(Some(&merkle_tree_storage_acc))?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&config::INIT_BYTES_MERKLE_TREE_18[..])?;
        Ok(())
    }

    pub fn initialize_merkle_tree_update_state(
        ctx: Context<InitializeMerkleTreeUpdateState>,
        merkle_tree_index: u64
    ) -> Result<()>{
        // let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&ctx.accounts.two_leaves_pda.to_account_info().data.borrow())?;
        msg!("InitializeMerkleTreeUpdateState");

        // TODO check merkle tree index if not already done in contraints

        let tmp_storage_pda = &mut ctx.accounts.merkle_tree_tmp_storage.load_init()?;
        //increased by 2 because we're inserting 2 leaves at once
        tmp_storage_pda.merkle_tree_index = merkle_tree_index.try_into().unwrap();
        tmp_storage_pda.relayer = ctx.accounts.authority.key();
        tmp_storage_pda.merkle_tree_pda_pubkey = ctx.accounts.merkle_tree.key();
        // tmp_storage_pda.node_left = node_left.clone().try_into().unwrap();
        // tmp_storage_pda.node_right = node_right.clone().try_into().unwrap();

        // tmp_storage_pda.leaf_left = node_left.clone().try_into().unwrap();
        // tmp_storage_pda.leaf_right = node_right.clone().try_into().unwrap();
        tmp_storage_pda.current_instruction_index = 1;
        msg!("tmp_storage_pda.node_left: {:?}", tmp_storage_pda.node_left);
        msg!("tmp_storage_pda.node_right: {:?}", tmp_storage_pda.node_right);


        if ctx.remaining_accounts.len() == 0 || ctx.remaining_accounts.len() > 16 {
            msg!("Submitted number of leaves: {}", ctx.remaining_accounts.len());
            return err!(ErrorCode::InvalidNumberOfLeaves);
        }

        // tmp_storage_account.tmp_leaves_index = merkle_tree_account.next_index;
        // TODO: add looping over leaves to save their commithashes into the tmp account
        // this will make the upper leaf inserts obsolete still need to check what to do with node_left, right
        for (index, account) in ctx.remaining_accounts.iter().enumerate() {
            msg!("Copying leaves pair {}", index);
            if account.data.borrow()[1] != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
                msg!("Leaf pda state {} with address {:?} is already inserted",account.data.borrow()[1], *account.key);
                return err!(ErrorCode::LeafAlreadyInserted);
            }
            tmp_storage_pda.leaves[index][0] = account.data.borrow()[10..42].try_into().unwrap();
            tmp_storage_pda.leaves[index][1] = account.data.borrow()[42..74].try_into().unwrap();
            msg!("tmp_storage.leaves[index][0] {:?}", tmp_storage_pda.leaves[index][0]);
            msg!("tmp_storage.leaves[index][0] {:?}", tmp_storage_pda.leaves[index][1]);
            tmp_storage_pda.number_of_leaves = (index + 1).try_into().unwrap();
        }

        let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        tmp_storage_pda.tmp_leaves_index = merkle_tree_pda_data.next_index.try_into().unwrap();

        let current_slot = <Clock as  solana_program::sysvar::Sysvar>::get()?.slot;
        msg!("Current slot: {:?}", current_slot);

        msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
        msg!(
            "Lock ends at slot: {}",
            merkle_tree_pda_data.time_locked + constant::LOCK_DURATION
        );

        //lock
        if merkle_tree_pda_data.time_locked == 0
            || merkle_tree_pda_data.time_locked + constant::LOCK_DURATION < current_slot
        {
            merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
            merkle_tree_pda_data.pubkey_locked = ctx.accounts.merkle_tree_tmp_storage.key().to_bytes().to_vec();
            msg!("Locked at slot: {}", merkle_tree_pda_data.time_locked);
            msg!(
                "Locked by: {:?}",
                Pubkey::new(&merkle_tree_pda_data.pubkey_locked)
            );
        } else if merkle_tree_pda_data.time_locked + constant::LOCK_DURATION > current_slot {
            msg!("Contract is still locked.");
            return err!(ErrorCode::ContractStillLocked);
        } else {
            merkle_tree_pda_data.time_locked = <Clock as solana_program::sysvar::Sysvar>::get()?.slot;
            merkle_tree_pda_data.pubkey_locked = ctx.accounts.merkle_tree_tmp_storage.key().to_bytes().to_vec();
        }

        // merkle_tree_pubkey_check(
        //     *merkle_tree_pda.key,
        //     tmp_storage_pda_data.merkle_tree_index,
        //     *merkle_tree_pda.owner,
        //     self.program_id,
        // )?;
        MerkleTree::pack_into_slice(
            &merkle_tree_pda_data,
            &mut ctx.accounts.merkle_tree.data.borrow_mut(),
        );
        // execute

        Ok(())
    }

    pub fn update_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, UpdateMerkleTree<'info>>,
        _bump: u64//data: Vec<u8>,
    ) -> Result<()>{
        msg!("update_merkle_tree");

        // let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        processor::process_instruction(
            &mut ctx
        )?;
        Ok(())
    }

    pub fn last_transaction_update_merkle_tree<'a, 'b, 'c, 'info>(
        mut ctx: Context<'a, 'b, 'c, 'info, LastTransactionUpdateMerkleTree<'info>>,
        _bump: u64
    ) -> Result<()>{
        // doing checks after for mutability
        let mut merkle_tree_processor = MerkleTreeProcessor::new(None)?;
        // let close_acc = &ctx.accounts.merkle_tree_tmp_storage.to_account_info();
        // let close_to_acc = &ctx.accounts.authority.to_account_info();
        merkle_tree_processor.insert_root(&mut ctx)?;


        let tmp_storage_pda = ctx.accounts.merkle_tree_tmp_storage.load_mut()?;

        msg!("inserting merkle tree root");
         if tmp_storage_pda.current_instruction_index != 56 {
             msg!("Wrong state instruction index should be 56 is {}", tmp_storage_pda.current_instruction_index);
        }


        // let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        // processor::process_instruction(
        //     &mut ctx
        // )?;

        // // Close tmp account.
        // close_account(close_acc, close_to_acc).unwrap();
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
        let next_index:u64 = 2;
        let merkle_tree_pda_pubkey = vec![1u8;32];
        msg!("insert_two_leaves");
        // let tmp_storage_pda = ctx.accounts.merkle_tree_tmp_storage.to_account_info();
        // let mut tmp_storage_pda_data = MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        let rent = &Rent::from_account_info(&ctx.accounts.rent.to_account_info())?;
        let two_leaves_pda = ctx.accounts.two_leaves_pda.to_account_info();
        // let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        msg!("Creating two_leaves_pda.");
        create_and_check_pda(
            &ctx.program_id,
            &ctx.accounts.authority.to_account_info(),
            &two_leaves_pda.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            rent,
            &nullifier,
            &b"leaves"[..],
            TWO_LEAVES_PDA_SIZE, //bytes
            0,                   //lamports
            true,                //rent_exempt
        );
        let mut leaf_pda_account_data = TwoLeavesBytesPda::unpack(&two_leaves_pda.data.borrow())?;

        leaf_pda_account_data.account_type = UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
        //save leaves into pda account
        leaf_pda_account_data.node_left = leaf_left.to_vec();
        leaf_pda_account_data.node_right = leaf_right.to_vec();
        //increased by 2 because we're inserting 2 leaves at once
        leaf_pda_account_data.left_leaf_index = next_index.try_into().unwrap();
        leaf_pda_account_data.merkle_tree_pubkey = merkle_tree_pda_pubkey.to_vec();
        // anchor pads encryptedUtxos of length 222 to 254 with 32 zeros in front
        msg!("encrypted_utxos: {:?}", encrypted_utxos.to_vec());
        leaf_pda_account_data.encrypted_utxos = encrypted_utxos[0..222].to_vec();

        TwoLeavesBytesPda::pack_into_slice(
            &leaf_pda_account_data,
            &mut two_leaves_pda.data.borrow_mut(),
        );
        msg!("packed two_leaves_pda");
        Ok(())
    }
    /*pub fn deposit_sol(ctx: Context<DepositSOL>, data: Vec<u8>) -> Result<()>{
        let mut new_data = data.clone();
        new_data.insert(0, 1);
        processor::process_sol_transfer(
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

        processor::process_sol_transfer(
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
}

#[derive(Accounts)]
pub struct InitializeNewMerkleTree<'info> {
    #[account(address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: it should be unpacked internally
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(merkle_tree_index: u64)]
pub struct InitializeMerkleTreeUpdateState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(
        init,
        seeds = [&authority.key().to_bytes().as_ref(), STORAGE_SEED.as_ref()],
        bump,
        payer = authority,
        space = MERKLE_TREE_TMP_PDA_SIZE + 64 * 20,
    )]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK: that the merkle tree is whitelisted
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` that merkle tree is locked for this account
    #[account(mut)]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_tmp_storage
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_tmp_storage.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct LastTransactionUpdateMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut, close = authority)]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_tmp_storage
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_tmp_storage.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut)]
    pub two_leaves_pda: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
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

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary., owner= Pubkey::new(b"2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av")
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,
    // recipients are specified in additional accounts and checked in the verifier
}

#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct InitializeNullifier<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&(nullifier.as_slice()[0..32]), NF_SEED.as_ref()],
        bump,
        space = 8,
    )]
    pub nullifier_pda: Account<'info, Nullifier>,
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Nullfier pdas are derived from the nullifier
// existence of a nullifier is the check to
// prevent double spends.
#[account]
pub struct Nullifier {}

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


#[error_code]
pub enum ErrorCode {
    #[msg("Merkle tree tmp account init failed wrong pda.")]
    MtTmpPdaInitFailed,
    #[msg("Merkle tree tmp account init failed.")]
    MerkleTreeInitFailed,
    #[msg("Contract is still locked.")]
    ContractStillLocked,
    #[msg("InvalidMerkleTree.")]
    InvalidMerkleTree,
    #[msg("InvalidMerkleTreeOwner.")]
    InvalidMerkleTreeOwner,
    #[msg("PubkeyCheckFailed")]
    PubkeyCheckFailed,
    #[msg("CloseAccountFailed")]
    CloseAccountFailed,
    #[msg("WithdrawalFailed")]
    WithdrawalFailed,
    #[msg("MerkleTreeUpdateNotInRootInsert")]
    MerkleTreeUpdateNotInRootInsert,
    #[msg("InvalidNumberOfLeaves")]
    InvalidNumberOfLeaves,
    #[msg("LeafAlreadyInserted")]
    LeafAlreadyInserted,
    #[msg("WrongLeavesLastTx")]
    WrongLeavesLastTx
}
