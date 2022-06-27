use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use crate::processor::pubkey_check;
use crate::state::MerkleTree;
use crate::utils::constants::ROOT_INSERT;
use crate::utils::constants::IX_ORDER;
use crate::MerkleTreeTmpPda;
use crate::poseidon_merkle_tree::instructions::insert_last_double;
use crate::utils::config;
use crate::errors::ErrorCode;

#[derive(Accounts)]
pub struct InsertRoot<'info> {
    #[account(mut, address=merkle_tree_tmp_storage.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    #[account(mut, close = authority)]
    pub merkle_tree_tmp_storage: AccountLoader<'info ,MerkleTreeTmpPda>,
    /// CHECK:` that the merkle tree is whitelisted and consistent with merkle_tree_tmp_storage
    #[account(mut, constraint = merkle_tree.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_tmp_storage.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}



pub fn process_insert_root(
    ctx: &mut Context<InsertRoot>,
) -> Result<()>  {
    let tmp_storage_pda_data = &mut ctx.accounts.merkle_tree_tmp_storage.load_mut()?;

    //inserting root and creating leave pda accounts
    msg!(
        "Root insert Instruction: {}",
        IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize]
    );

    if IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize] != ROOT_INSERT {
        msg!("Merkle Tree update not completed yet, cannot insert root.");
        return err!(ErrorCode::MerkleTreeUpdateNotInRootInsert);
    }

    let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

    msg!("Pubkey::new(&merkle_tree_pda_data.pubkey_locked): {:?}", Pubkey::new(&merkle_tree_pda_data.pubkey_locked));
    msg!("ctx.accounts.merkle_tree_tmp_storage.key(): {:?}", ctx.accounts.merkle_tree_tmp_storage.key());

    //checking if signer locked
    pubkey_check(
        ctx.accounts.merkle_tree_tmp_storage.key(),
        Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
        String::from("Merkle tree locked by another account."),
    )?;

    //insert root into merkle tree
    insert_last_double(&mut merkle_tree_pda_data, tmp_storage_pda_data)?;

    msg!("Lock set at slot: {}", merkle_tree_pda_data.time_locked);
    msg!("Lock released at slot: {}", <Clock as Sysvar>::get()?.slot);
    merkle_tree_pda_data.time_locked = 0;
    merkle_tree_pda_data.pubkey_locked = vec![0; 32];

    MerkleTree::pack_into_slice(
        &merkle_tree_pda_data,
        &mut ctx.accounts.merkle_tree.data.borrow_mut(),
    );

Ok(())
}
