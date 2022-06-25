use crate::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_ACCOUNT_TYPE};
use anchor_lang::solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::{ByteOrder, LittleEndian};
use std::convert::TryInto;
use crate::UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE;
#[allow(unused_variables)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MerkleTree {
    pub is_initialized: bool,
    pub levels: usize,
    pub filled_subtrees: Vec<Vec<u8>>,
    pub current_root_index: usize,
    pub next_index: usize,
    pub root_history_size: usize,
    pub roots: Vec<u8>,
    pub current_total_deposits: u64,
    pub inserted_leaf: bool,
    pub inserted_root: bool,
    pub time_locked: u64,
    pub pubkey_locked: Vec<u8>,
}
impl Sealed for MerkleTree {}
impl IsInitialized for MerkleTree {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for MerkleTree {
    //height 18
    const LEN: usize = 16658;
    fn unpack_from_slice(input: &[u8]) -> std::result::Result<Self, ProgramError> {
        let input = array_ref![input, 0, MerkleTree::LEN];

        let (
            is_initialized,
            account_type,
            levels,
            filled_subtrees,
            current_root_index,
            next_index,
            root_history_size,
            //609
            roots,
            //18137
            current_total_deposits,
            pubkey_locked,
            time_locked,
        ) = array_refs![input, 1, 1, 8, 576, 8, 8, 8, 16000, 8, 32, 8];

        if 1u8 != is_initialized[0] {
            msg!("merkle tree account is not initialized");
            return Err(ProgramError::UninitializedAccount);
        }
        if account_type[0] != MERKLE_TREE_ACCOUNT_TYPE {
            msg!("Account is not of type Merkle tree.");
            return Err(ProgramError::InvalidAccountData);
        }
        let mut tmp_subtree_vec = vec![vec![0u8; 32]; 18];

        for (i, bytes) in filled_subtrees.chunks(32).enumerate() {
            tmp_subtree_vec[i] = bytes.to_vec();
        }

        let current_root_index = usize::from_le_bytes(*current_root_index);

        let mut tmp_roots_vec = vec![0u8; 32];
        let current_root_start_range = current_root_index * 32;
        let current_root_end_range = (current_root_index + 1) * 32;

        for (i, byte) in roots[current_root_start_range..current_root_end_range]
            .iter()
            .enumerate()
        {
            tmp_roots_vec[i] = *byte;
        }

        let next_index = usize::from_le_bytes(*next_index);

        Ok(MerkleTree {
            is_initialized: true,
            levels: usize::from_le_bytes(*levels),
            filled_subtrees: tmp_subtree_vec,
            current_root_index,
            next_index,
            root_history_size: usize::from_le_bytes(*root_history_size),
            roots: tmp_roots_vec.to_vec(),
            current_total_deposits: LittleEndian::read_u64(current_total_deposits),
            inserted_leaf: false,
            inserted_root: false,
            pubkey_locked: pubkey_locked.to_vec(),
            time_locked: u64::from_le_bytes(*time_locked),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        //if self.inserted_leaf {
        let dst = array_mut_ref![dst, 0, MerkleTree::LEN];

        let (
            _is_initialized_dst,
            _account_type_dst,
            _levels_dst,
            filled_subtrees_dst,
            current_root_index_dst,
            next_index_dst,
            _root_history_size_dst,
            roots_dst,
            current_total_deposits_dst,
            pubkey_locked_dst,
            time_locked_dst,
        ) = mut_array_refs![dst, 1, 1, 8, 576, 8, 8, 8, 16000, 8, 32, 8];

        // could change this to insert only the changed subtree if one is changed
        let mut i = 0;
        for it in &self.filled_subtrees {
            for j in it {
                filled_subtrees_dst[i] = *j;
                i += 1;
            }
        }
        if self.inserted_root {
            let mut i = 0;
            if self.current_root_index != 0 {
                i = self.current_root_index;
            }
            let mut i_tmp = i * 32;
            for it in self.roots.iter() {
                roots_dst[i_tmp] = *it;
                i_tmp += 1;
            }
            msg!("packed root");
        }

        LittleEndian::write_u64(
            current_root_index_dst,
            self.current_root_index.try_into().unwrap(),
        );
        LittleEndian::write_u64(next_index_dst, self.next_index.try_into().unwrap());
        LittleEndian::write_u64(
            current_total_deposits_dst,
            self.current_total_deposits, //.try_into().unwrap(),
        );
        *pubkey_locked_dst = self.pubkey_locked.clone().try_into().unwrap();

        LittleEndian::write_u64(time_locked_dst, self.time_locked); // TODO: check if removing try_into().unwrap() has sideeffects
        msg!("packed merkle tree");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitMerkleTreeBytes {
    pub is_initialized: bool,
    pub bytes: Vec<u8>,
}
impl Sealed for InitMerkleTreeBytes {}
impl IsInitialized for InitMerkleTreeBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for InitMerkleTreeBytes {
    const LEN: usize = 16658;

    fn unpack_from_slice(input: &[u8]) -> std::result::Result<Self, ProgramError> {
        let input = array_ref![input, 0, InitMerkleTreeBytes::LEN];

        let (bytes, _left_over) = array_refs![input, 642, 16016];

        if bytes[0] != 0 {
            msg!("Tree is already initialized");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(InitMerkleTreeBytes {
            is_initialized: true,
            bytes: bytes.to_vec(),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, InitMerkleTreeBytes::LEN];

        let (bytes_dst, _left_over_dst) = mut_array_refs![dst, 642, 16016];

        *bytes_dst = self.bytes.clone().try_into().unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct TwoLeavesBytesPda {
    pub is_initialized: bool,
    pub account_type: u8,
    pub node_right: Vec<u8>,
    pub node_left: Vec<u8>,
    pub merkle_tree_pubkey: Vec<u8>,
    pub encrypted_utxos: Vec<u8>,
    pub left_leaf_index: usize,
}

impl Sealed for TwoLeavesBytesPda {}
impl IsInitialized for TwoLeavesBytesPda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for TwoLeavesBytesPda {
    const LEN: usize = 106 + ENCRYPTED_UTXOS_LENGTH;

    fn unpack_from_slice(input: &[u8]) -> std::result::Result<Self, ProgramError> {
        let input = array_ref![input, 0, TwoLeavesBytesPda::LEN];

        let (
            _is_initialized,
            account_type,
            left_leaf_index,
            node_left,
            node_right,
            merkle_tree_pubkey,
            _encrypted_utxos,
        ) = array_refs![input, 1, 1, 8, 32, 32, 32, ENCRYPTED_UTXOS_LENGTH];
        //check that account was not initialized before
        //assert_eq!(is_initialized[0], 0);
        // if is_initialized[0] != 0 || account_type[0] != UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE {
        //     msg!("Leaf pda is already initialized");
        //     return Err(ProgramError::InvalidAccountData);
        // }
        Ok(TwoLeavesBytesPda {
            is_initialized: true,
            account_type: account_type[0],
            node_left: node_left.to_vec(),
            node_right: node_right.to_vec(),
            merkle_tree_pubkey: merkle_tree_pubkey.to_vec(),
            encrypted_utxos: vec![0u8; ENCRYPTED_UTXOS_LENGTH],
            left_leaf_index: usize::from_le_bytes(*left_leaf_index),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, TwoLeavesBytesPda::LEN];
        let (
            is_initialized_dst,
            account_type_dst,
            left_leaf_index_dst,
            node_left_dst,
            node_right_dst,
            merkle_tree_pubkey_dst,
            encrypted_utxos_dst,
        ) = mut_array_refs![dst, 1, 1, 8, 32, 32, 32, ENCRYPTED_UTXOS_LENGTH];

        *is_initialized_dst = [1];
        *account_type_dst = [self.account_type];
        *node_right_dst = self.node_right.clone().try_into().unwrap();
        *node_left_dst = self.node_left.clone().try_into().unwrap();
        *merkle_tree_pubkey_dst = self.merkle_tree_pubkey.clone().try_into().unwrap();
        *left_leaf_index_dst = usize::to_le_bytes(self.left_leaf_index);
        *encrypted_utxos_dst = self.encrypted_utxos.clone().try_into().unwrap();
        msg!("packed inserted_leaves");
    }
}
