use crate::poseidon_merkle_tree::instructions::*;
use crate::poseidon_merkle_tree::instructions_poseidon::{poseidon_0, poseidon_1, poseidon_2};
use crate::poseidon_merkle_tree::state::TwoLeavesBytesPda;
use crate::poseidon_merkle_tree::state::{InitMerkleTreeBytes, MerkleTree};
use crate::state::MerkleTreeTmpPda;
use crate::utils::config::MERKLE_TREE_ACC_BYTES_ARRAY;
use crate::{IX_ORDER, TWO_LEAVES_PDA_SIZE};
use crate::constant::{
    LOCK_DURATION,
    MERKLE_TREE_UPDATE_START,
    MERKLE_TREE_UPDATE_LEVEL,
    LOCK_START,
    HASH_0,
    HASH_1,
    HASH_2,
    ROOT_INSERT,
};
use anchor_lang::solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use std::convert::TryFrom;
use std::cell::RefMut;
use anchor_lang::prelude::*;
use crate::{
    UpdateMerkleTree,
    LastTransactionUpdateMerkleTree
};
use crate::ErrorCode;



pub struct MerkleTreeProcessor<'a, 'b> {
    merkle_tree_pda: Option<&'a AccountInfo<'b>>,
    unpacked_merkle_tree: MerkleTree,
}

impl<'a, 'b> MerkleTreeProcessor<'a, 'b> {
    pub fn new(
        merkle_tree_pda: Option<&'a AccountInfo<'b>>,
    ) -> Result<Self> {
        let empty_smt = MerkleTree {
            is_initialized: false,
            levels: 1,
            filled_subtrees: vec![vec![0_u8; 1]; 1],
            //zeros: vec![vec![0 as u8; 1];1],
            current_root_index: 0,
            next_index: 0,
            root_history_size: 10,
            roots: vec![0_u8; 1],
            //leaves: vec![0],
            current_total_deposits: 0,
            inserted_leaf: false,
            inserted_root: false,
            pubkey_locked: vec![0],
            time_locked: 0,
        };

        Ok(MerkleTreeProcessor {
            merkle_tree_pda,
            unpacked_merkle_tree: empty_smt,
        })
    }

    #[allow(clippy::manual_memcpy)]
    pub fn initialize_new_merkle_tree_from_bytes(
        &mut self,
        init_bytes: &[u8],
    ) -> Result<()>  {
        let mut unpacked_init_merkle_tree =
            InitMerkleTreeBytes::unpack(&self.merkle_tree_pda.unwrap().data.borrow())?;

        for i in 0..unpacked_init_merkle_tree.bytes.len() {
            unpacked_init_merkle_tree.bytes[i] = init_bytes[i];
        }

        InitMerkleTreeBytes::pack_into_slice(
            &unpacked_init_merkle_tree,
            &mut self.merkle_tree_pda.unwrap().data.borrow_mut(),
        );
        if unpacked_init_merkle_tree.bytes[0..init_bytes.len()] != init_bytes[..] {
            msg!("merkle tree init failed");
            return err!(ErrorCode::MerkleTreeInitFailed);
        }
        Ok(())
    }
    // can delete completely since it is only one function
    pub fn process_instruction(
        &mut self,
        ctx: &mut Context<UpdateMerkleTree>,
    ) -> Result<()>  {
        let tmp_storage_pda_data = &mut ctx.accounts.merkle_tree_tmp_storage.load_mut()?;
        let mut merkle_tree_pda_data = MerkleTree::unpack(&ctx.accounts.merkle_tree.data.borrow())?;

        pubkey_check(
            ctx.accounts.merkle_tree_tmp_storage.key(),
            Pubkey::new(&merkle_tree_pda_data.pubkey_locked),
            String::from("Merkle tree locked by another account."),
        )?;
        msg!(
            "tmp_storage_pda_data.current_instruction_index {}",
            tmp_storage_pda_data.current_instruction_index
        );
        /*
        if tmp_storage_pda_data.current_instruction_index < IX_ORDER.len().try_into().unwrap()
            && (IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize]
                == MERKLE_TREE_UPDATE_START
                || IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize]
                    == MERKLE_TREE_UPDATE_LEVEL)
        {

            // merkle_tree_pubkey_check(
            //     *merkle_tree_pda.key,
            //     tmp_storage_pda_data.merkle_tree_index,
            //     *merkle_tree_pda.owner,
            //     self.program_id,
            // )?;


            _process_instruction(
                IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
                tmp_storage_pda_data,
                &mut merkle_tree_pda_data,
            )?;
            // tmp_storage_pda_data.changed_state = 4;

            MerkleTree::pack_into_slice(
                &merkle_tree_pda_data,
                &mut ctx.accounts.merkle_tree.data.borrow_mut(),
            );
        } else if IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize] == HASH_0
            || IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize] == HASH_1
            || IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize] == HASH_2
        {
            // merkle_tree_pubkey_check(
            //     *merkle_tree_pda.key,
            //     tmp_storage_pda_data.merkle_tree_index,
            //     *merkle_tree_pda.owner,
            //     self.program_id,
            // )?;
            //hash instructions do not need the merkle tree
            _process_instruction(
                IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
                tmp_storage_pda_data,
                &mut merkle_tree_pda_data,
            )?;
            MerkleTree::pack_into_slice(
                &merkle_tree_pda_data,
                &mut ctx.accounts.merkle_tree.data.borrow_mut(),
            );
            // tmp_storage_pda_data.changed_state = 2;
        }*/
        if tmp_storage_pda_data.current_instruction_index < IX_ORDER.len() as u64 {
            _process_instruction(
                IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize],
                tmp_storage_pda_data,
                &mut merkle_tree_pda_data,
            )?;
            MerkleTree::pack_into_slice(
                &merkle_tree_pda_data,
                &mut ctx.accounts.merkle_tree.data.borrow_mut(),
            );
        }
        msg!(
            "tmp_storage_pda_data.current_instruction_index : {}",
            tmp_storage_pda_data.current_instruction_index
        );
        msg!(
            "tmp_storage_pda_data.current_instruction_index : {:?}",
            IX_ORDER[tmp_storage_pda_data.current_instruction_index as usize]
        );
        // tmp_storage_pda_data.current_instruction_index += 1;

        // MerkleTreeTmpPda::pack_into_slice(
        //     &tmp_storage_pda_data,
        //     &mut self.tmp_storage_pda.unwrap().data.borrow_mut(),
        // );
        Ok(())
    }

    pub fn insert_root(
        &mut self,
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
        //checking merkle tree pubkey for consistency
        // merkle_tree_pubkey_check(
        //     *merkle_tree_pda.key,
        //     tmp_storage_pda_data.merkle_tree_index,
        //     *merkle_tree_pda.owner,
        //     self.program_id,
        // )?;
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
}

pub fn _process_instruction(
    id: u8,
    tmp_storage_pda_data: &mut MerkleTreeTmpPda,
    merkle_tree_pda_data: &mut MerkleTree,
) -> Result<()>  {
    msg!("executing instruction {}", id);
    if id == HASH_0 {
        poseidon_0(tmp_storage_pda_data)?;
    } else if id == HASH_1 {
        poseidon_1(tmp_storage_pda_data)?;
    } else if id == HASH_2 {
        poseidon_2(tmp_storage_pda_data)?;
        if tmp_storage_pda_data.current_level < 18 {
            insert_1_inner_loop(merkle_tree_pda_data, tmp_storage_pda_data)?;
        }
    } else if id == MERKLE_TREE_UPDATE_LEVEL {
        panic!("should not enter");
    } else if id == MERKLE_TREE_UPDATE_START {
        insert_0_double(merkle_tree_pda_data, tmp_storage_pda_data)?;
    }
    Ok(())
}
// start update
// update level until you get a zero value on the right if that then
// start update again and repeat until no more queued leaves
// no need for ix array
// just need to keep track of the current level and stop at the top
// also need to keep track of whether a hash is completed
#[allow(dead_code)]
fn merkle_tree_pubkey_check(
    account_pubkey: Pubkey,
    merkle_tree_index: usize,
    merkle_tree_pda_owner: Pubkey,
    program_id: Pubkey,
) -> Result<()>  {
    if account_pubkey != Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index].0) {
        msg!(
            "invalid merkle tree {:?}, {:?}",
            account_pubkey,
            Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[merkle_tree_index].0)
        );
        return err!(ErrorCode::InvalidMerkleTree);
    }
    if merkle_tree_pda_owner != program_id {
        msg!("Invalid merkle tree owner.");
        return err!(ErrorCode::InvalidMerkleTreeOwner);
    }
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
