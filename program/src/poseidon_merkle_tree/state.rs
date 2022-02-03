use crate::config::MERKLE_TREE_ACCOUNT_TYPE;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::{ByteOrder, LittleEndian};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryInto;

#[allow(unused_variables)]
#[derive(Debug)]
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
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
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
    }
}

#[derive(Debug, Clone)]
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

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
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

// Account structs for merkle tree:
#[derive(Debug)]
pub struct TmpStoragePda {
    pub is_initialized: bool,
    pub merkle_tree_index: u8,
    pub state: Vec<Vec<u8>>,
    pub current_round: usize,
    pub current_round_index: usize,
    pub leaf_left: Vec<u8>,
    pub leaf_right: Vec<u8>,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
    pub current_level_hash: Vec<u8>,
    pub current_index: usize,
    pub current_level: usize,
    pub current_instruction_index: usize,
}

impl Sealed for TmpStoragePda {}
impl IsInitialized for TmpStoragePda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for TmpStoragePda {
    const LEN: usize = 3900; //297;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TmpStoragePda::LEN];

        let (
            _is_initialized,
            _unused_remainder0,
            merkle_tree_index,
            _unused_remainder0_1,
            current_instruction_index,
            //220
            _unused_remainder1,
            state,
            current_round,
            current_round_index,
            left,
            right,
            current_level_hash,
            current_index,
            current_level,
            leaf_left,
            leaf_right,
            _nullifier_0,
            _nullifier_1,
        ) = array_refs![input, 1, 2, 1, 208, 8, 3328, 96, 8, 8, 32, 32, 32, 8, 8, 32, 32, 32, 32];

        let mut parsed_state = Vec::new();
        for i in state.chunks(32) {
            parsed_state.push(i.to_vec());
        }

        Ok(TmpStoragePda {
            is_initialized: true,
            merkle_tree_index: merkle_tree_index[0],
            state: parsed_state.to_vec(),
            current_round: usize::from_le_bytes(*current_round),
            current_round_index: usize::from_le_bytes(*current_round_index),
            leaf_left: leaf_left.to_vec(),
            leaf_right: leaf_right.to_vec(),
            left: left.to_vec(),
            right: right.to_vec(),
            current_level_hash: current_level_hash.to_vec(),
            current_index: usize::from_le_bytes(*current_index),
            current_level: usize::from_le_bytes(*current_level),
            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, TmpStoragePda::LEN];

        let (
            _is_initialized_dst,
            _unused_remainder0_dst,
            current_instruction_index_dst,
            //220
            _unused_remainder1_dst,
            state_dst,
            current_round_dst,
            current_round_index_dst,
            left_dst,
            right_dst,
            current_level_hash_dst,
            current_index_dst,
            current_level_dst,
            leaf_left_dst,
            leaf_right_dst,
            //+288
            _nullifier_0_dst,
            _nullifier_1_dst,
        ) = mut_array_refs![dst, 1, 211, 8, 3328, 96, 8, 8, 32, 32, 32, 8, 8, 32, 32, 32, 32];

        let mut state_tmp = [0u8; 96];
        let mut z = 0;
        for i in self.state.iter() {
            for j in i {
                state_tmp[z] = *j;
                z += 1;
            }
        }

        *state_dst = state_tmp;
        *current_round_dst = usize::to_le_bytes(self.current_round);
        *current_round_index_dst = usize::to_le_bytes(self.current_round_index);

        *leaf_left_dst = self.leaf_left.clone().try_into().unwrap();

        *leaf_right_dst = self.leaf_right.clone().try_into().unwrap();
        *left_dst = self.left.clone().try_into().unwrap();

        *right_dst = self.right.clone().try_into().unwrap();
        *current_level_hash_dst = self.current_level_hash.clone().try_into().unwrap();

        *current_index_dst = usize::to_le_bytes(self.current_index);
        *current_level_dst = usize::to_le_bytes(self.current_level);
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
    }
}

#[derive(Clone, Debug)]
pub struct TwoLeavesBytesPda {
    pub is_initialized: bool,
    pub account_type: u8,
    pub leaf_right: Vec<u8>,
    pub leaf_left: Vec<u8>,
    pub merkle_tree_pubkey: Vec<u8>,
    pub left_leaf_index: usize,
}

impl Sealed for TwoLeavesBytesPda {}
impl IsInitialized for TwoLeavesBytesPda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for TwoLeavesBytesPda {
    const LEN: usize = 106;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, TwoLeavesBytesPda::LEN];

        let (
            is_initialized,
            _account_type,
            _left_leaf_index,
            _leaf_left,
            _leaf_right,
            _merkle_tree_pubkey,
        ) = array_refs![input, 1, 1, 8, 32, 32, 32];
        //check that account was not initialized before
        //assert_eq!(is_initialized[0], 0);
        if is_initialized[0] != 0 {
            msg!("Leaf pda is already initialized");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(TwoLeavesBytesPda {
            is_initialized: true,
            account_type: 4,
            leaf_right: vec![0u8; 32],
            leaf_left: vec![0u8; 32],
            merkle_tree_pubkey: vec![0u8; 32],
            left_leaf_index: 0usize,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, TwoLeavesBytesPda::LEN];
        let (
            is_initialized_dst,
            account_type_dst,
            left_leaf_index_dst,
            leaf_left_dst,
            leaf_right_dst,
            merkle_tree_pubkey_dst,
        ) = mut_array_refs![dst, 1, 1, 8, 32, 32, 32];

        *is_initialized_dst = [1];
        *account_type_dst = [4];
        *leaf_right_dst = self.leaf_right.clone().try_into().unwrap();
        *leaf_left_dst = self.leaf_left.clone().try_into().unwrap();
        *merkle_tree_pubkey_dst = self.merkle_tree_pubkey.clone().try_into().unwrap();
        *left_leaf_index_dst = usize::to_le_bytes(self.left_leaf_index);

        msg!("packed inserted_leaves");
    }
}
