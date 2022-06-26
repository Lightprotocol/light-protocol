use crate::poseidon_merkle_tree::instructions::*;
use crate::poseidon_merkle_tree::instructions_poseidon::{poseidon_0, poseidon_1, poseidon_2};
use crate::poseidon_merkle_tree::state::{InitMerkleTreeBytes, MerkleTree};
use crate::state::MerkleTreeTmpPda;
use crate::utils::config::{
    MERKLE_TREE_HEIGHT
};
use crate::constant::{
    MERKLE_TREE_UPDATE_START,
    HASH_0,
    HASH_1,
    HASH_2,
    ROOT_INSERT,
    IX_ORDER
};
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use anchor_lang::prelude::*;
use crate::{
    UpdateMerkleTree,
    LastTransactionUpdateMerkleTree
};
use crate::ErrorCode;

#[allow(clippy::manual_memcpy)]
pub fn initialize_new_merkle_tree_from_bytes(
    merkle_tree_pda: AccountInfo,
    init_bytes: &[u8],
) -> Result<()>  {
    let mut unpacked_init_merkle_tree =
        InitMerkleTreeBytes::unpack(&merkle_tree_pda.data.borrow())?;

    for i in 0..unpacked_init_merkle_tree.bytes.len() {
        unpacked_init_merkle_tree.bytes[i] = init_bytes[i];
    }

    InitMerkleTreeBytes::pack_into_slice(
        &unpacked_init_merkle_tree,
        &mut merkle_tree_pda.data.borrow_mut(),
    );
    if unpacked_init_merkle_tree.bytes[0..init_bytes.len()] != init_bytes[..] {
        msg!("merkle tree init failed");
        return err!(ErrorCode::MerkleTreeInitFailed);
    }
    Ok(())
}

pub fn compute_updated_merkle_tree(
    id: u8,
    tmp_storage_pda_data: &mut MerkleTreeTmpPda,
    merkle_tree_pda_data: &mut MerkleTree,
) -> Result<()>  {
    msg!("executing instruction {}", id);
    // Hash computation is split into three parts which can be executed in ~2m compute units
    if id == HASH_0 {
        poseidon_0(tmp_storage_pda_data)?;
    } else if id == HASH_1 {
        poseidon_1(tmp_storage_pda_data)?;
    } else if id == HASH_2 {
        poseidon_2(tmp_storage_pda_data)?;
        // Updating the current level hash after a new hash is completely computed.
        if tmp_storage_pda_data.current_level < MERKLE_TREE_HEIGHT {
            insert_1_inner_loop(merkle_tree_pda_data, tmp_storage_pda_data)?;
        }
    } else if id == MERKLE_TREE_UPDATE_START {
        insert_0_double(merkle_tree_pda_data, tmp_storage_pda_data)?;
    }
    Ok(())
}

pub fn insert_root(
    ctx: &mut Context<LastTransactionUpdateMerkleTree>,
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


pub fn pubkey_check(
    account_pubkey0: Pubkey,
    account_pubkey1: Pubkey,
    msg: String,
) -> Result<()>  {
    if account_pubkey0 != account_pubkey1 {
        msg!(&msg);
        return err!(ErrorCode::PubkeyCheckFailed);
    }

    Ok(())
}
