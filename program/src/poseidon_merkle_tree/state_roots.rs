use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use crate::utils::init_bytes18::MERKLE_TREE_ACC_BYTES_ARRAY;
use arrayref::{array_ref, array_refs};
use std::convert::TryFrom;

// max roots that can be checked within one ix memory budget.
const ROOT_HISTORY_SIZE: u64 = 593;

#[derive(Clone, Debug)]
pub struct MerkleTreeRoots {
    pub is_initialized: bool,
    pub roots: Vec<u8>,
    pub root_history_size: u64,
}

impl Sealed for MerkleTreeRoots {}
impl IsInitialized for MerkleTreeRoots {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeRoots {
    const LEN: usize = 16657;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, MerkleTreeRoots::LEN];

        let (
            is_initialized,
            _levels,
            _filled_subtrees,
            _current_root_index,
            _next_index,
            root_history_size,
            //609
            roots,
            //18137
            _unused_remainder,
        ) = array_refs![input, 1, 8, 576, 8, 8, 8, 16000, 48];

        if is_initialized[0] != 1u8 {
            msg!("Merkle Tree is not initialized");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(MerkleTreeRoots {
            is_initialized: true,
            roots: roots.to_vec(),
            root_history_size: u64::from_le_bytes(*root_history_size),
        })
    }
    fn pack_into_slice(&self, _dst: &mut [u8]) {
        //is not meant to be called since this structs purpose is to solely unpack roots
        //to check for the existence of one root
    }
}

pub fn check_root_hash_exists(
    merkle_tree_pda: &AccountInfo,
    root_bytes: &Vec<u8>,
    program_id: &Pubkey,
    merkle_tree_index: u8,
) -> Result<u8, ProgramError> {
    let merkle_tree_pda_data = MerkleTreeRoots::unpack(&merkle_tree_pda.data.borrow()).unwrap();
    msg!("Passed-in merkle_tree_pda pubkey: {:?}", *merkle_tree_pda);
    msg!(
        "Checks against hardcoded merkle_tree_pda pubkey: {:?}",
        solana_program::pubkey::Pubkey::new(
            &MERKLE_TREE_ACC_BYTES_ARRAY
                [<usize as TryFrom<u8>>::try_from(merkle_tree_index).unwrap()]
            .0
        )
    );

    if *merkle_tree_pda.key
        != solana_program::pubkey::Pubkey::new(
            &MERKLE_TREE_ACC_BYTES_ARRAY
                [<usize as TryFrom<u8>>::try_from(merkle_tree_index).unwrap()]
            .0,
        )
    {
        msg!("Merkle tree account pubkey is incorrect.");
        return Err(ProgramError::InvalidArgument);
    }

    if *merkle_tree_pda.owner != *program_id {
        msg!("Merkle tree account owner is incorrect.");
        return Err(ProgramError::IllegalOwner);
    }

    if merkle_tree_pda_data.root_history_size > ROOT_HISTORY_SIZE {
        msg!("Root history size too large.");
        return Err(ProgramError::InvalidAccountData);
    }
    msg!("Looking for root: {:?}", *root_bytes);
    let mut found_root = 0u8;
    for (i, chunk) in merkle_tree_pda_data.roots.chunks(32).enumerate() {
        if *chunk == *root_bytes {
            msg!("Found root hash index: {}", i);
            found_root = 1u8;
            break;
        }
    }
    if found_root != 1 {
        msg!("Did not find root.");
        return Err(ProgramError::InvalidAccountData);
    }

    // let mut i = 0;
    // let mut counter = 0;
    //
    // loop {
    //     if merkle_tree_pda_data.roots[i..i + ROOT_HASH_SIZE] == *root_bytes {
    //         msg!("Found root hash index: {}", counter);
    //         found_root = 1u8;
    //         break;
    //     }
    //
    //     i += ROOT_HASH_SIZE;
    //     counter += 1;
    //     if counter == merkle_tree_pda_data.root_history_size {
    //         msg!("Did not find root.");
    //         return Err(ProgramError::InvalidAccountData);
    //     }
    // }
    Ok(found_root)
}
