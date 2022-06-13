use crate::utils::config::{MERKLE_TREE_TMP_PDA_SIZE, MERKLE_TREE_TMP_STORAGE_ACCOUNT_TYPE};
use anchor_lang::solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryInto;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MerkleTreeTmpPda {
    pub is_initialized: bool,
    pub found_root: u8,
    pub account_type: u8,

    pub node_left: Vec<u8>,
    pub node_right: Vec<u8>,
    pub leaf_left: Vec<u8>,
    pub leaf_right: Vec<u8>,
    pub relayer: Vec<u8>,
    pub merkle_tree_pda_pubkey: Vec<u8>,
    pub root_hash: Vec<u8>,
    //
    pub state: Vec<u8>,
    pub current_round: usize,
    pub current_round_index: usize,
    pub current_instruction_index: usize,
    pub current_index: usize,
    pub current_level: usize,
    pub current_level_hash: Vec<u8>,
    // set changed_constants to true to pack specified values other values will not be packed
    pub changed_state: u8,

    pub leaves: Vec<Vec<u8>>,
    pub number_of_leaves: u8,
    pub insert_leaves_index: u8,
    pub tmp_leaves_index: Vec<u8>
}
impl MerkleTreeTmpPda {
    pub fn new() -> MerkleTreeTmpPda {
        MerkleTreeTmpPda {
            is_initialized: true,
            found_root: 0,
            account_type: 6,

            node_left: vec![0u8],
            node_right: vec![0u8],
            leaf_left: vec![0u8],
            leaf_right: vec![0u8],
            relayer: vec![0u8],
            merkle_tree_pda_pubkey: vec![0u8],
            root_hash: vec![0u8],
            state: vec![0u8],
            current_round: 0,
            current_round_index: 0,
            current_instruction_index: 0,
            current_index: 0,
            current_level: 0,
            current_level_hash: vec![0],
            // set changed_constants to true to pack specified values other values will not be packed
            changed_state: 1,
        }
    }
}
impl Sealed for MerkleTreeTmpPda {}
impl IsInitialized for MerkleTreeTmpPda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeTmpPda {
    const LEN: usize = MERKLE_TREE_TMP_PDA_SIZE; //3900; // 1020
                                                 // for 2 nullifiers 729
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, MerkleTreeTmpPda::LEN];

        let (
            _is_initialized,
            account_type,
            current_instruction_index,
            found_root,
            relayer,
            merkle_tree_pda_pubkey,
            root_hash,
            state,
            current_round,
            current_round_index,
            current_index,
            current_level,
            current_level_hash,
            node_left,
            node_right,
            leaf_left,
            leaf_right,
        ) = array_refs![
            input, 1,  //inited
            1,  // account type
            8,  // current instruction index
            1,  // found_root
            32, // relayer
            32, // merkle_tree_pda_pubkey
            32, // root_hash
            96, // poseidon state
            8,  // current round
            8,  // current round index
            8,  // current index
            8,  // current level
            32, // current level hash
            32, //node_left
            32, //node_right
            32, //leaf_left
            32  //leaf_right
        ];

        if _is_initialized[0] != 0u8 && account_type[0] != MERKLE_TREE_TMP_STORAGE_ACCOUNT_TYPE {
            msg!("Wrong account type.");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(MerkleTreeTmpPda {
            is_initialized: true,
            found_root: found_root[0], //0
            account_type: account_type[0],
            current_instruction_index: usize::from_le_bytes(*current_instruction_index), //1
            merkle_tree_pda_pubkey: merkle_tree_pda_pubkey.to_vec(),                     //2
            relayer: relayer.to_vec(),                                                   //3
            root_hash: root_hash.to_vec(),
            node_left: node_left.to_vec(),
            node_right: node_right.to_vec(),
            leaf_left: leaf_left.to_vec(),
            leaf_right: leaf_right.to_vec(),
            state: state.to_vec(),
            current_round: usize::from_le_bytes(*current_round),
            current_round_index: usize::from_le_bytes(*current_round_index),
            current_index: usize::from_le_bytes(*current_index),
            current_level: usize::from_le_bytes(*current_level),
            current_level_hash: current_level_hash.to_vec(),
            changed_state: 0,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeTmpPda::LEN];

        let (
            _is_initialized,
            account_type_dst,
            current_instruction_index_dst,
            found_root_dst,
            relayer_dst,
            merkle_tree_pda_pubkey_dst,
            root_hash_dst,
            state_dst,
            current_round_dst,
            current_round_index_dst,
            current_index_dst,
            current_level_dst,
            current_level_hash_dst,
            node_left_dst,
            node_right_dst,
            leaf_left_dst,
            leaf_right_dst,
        ) = mut_array_refs![
            dst, 1,  //inited
            1,  // account type
            8,  // current instruction index
            1,  // found_root
            32, // relayer
            32, // merkle_tree_pda_pubkey
            32, // root_hash
            96, // poseidon state
            8,  // current round
            8,  // current round index
            8,  // current index
            8,  // current level
            32, // current level hash
            32, //node_left
            32, //node_right
            32, //leaf_left
            32  //leaf_right
        ];

        if self.changed_state == 1 {
            *account_type_dst = [self.account_type; 1];
            *found_root_dst = [self.found_root; 1];
            // *verifier_index_dst = usize::to_le_bytes(self.verifier_index);
            *merkle_tree_pda_pubkey_dst = self.merkle_tree_pda_pubkey.clone().try_into().unwrap();
            *relayer_dst = self.relayer.clone().try_into().unwrap();
            *node_left_dst = self.node_left.clone().try_into().unwrap();
            *node_right_dst = self.node_right.clone().try_into().unwrap();
            *leaf_left_dst = self.node_left.clone().try_into().unwrap();
            *leaf_right_dst = self.node_right.clone().try_into().unwrap();
            *root_hash_dst = self.root_hash.clone().try_into().unwrap();
        } else if self.changed_state == 2 {
            msg!("packing state: {:?}", self.state[..32].to_vec());

            *state_dst = self.state.clone().try_into().unwrap();
            *current_round_dst = usize::to_le_bytes(self.current_round);
            *current_round_index_dst = usize::to_le_bytes(self.current_round_index);
            *current_index_dst = usize::to_le_bytes(self.current_index);
            *current_level_dst = usize::to_le_bytes(self.current_level);
            *current_level_hash_dst = self.current_level_hash.clone().try_into().unwrap();
        } else if self.changed_state == 3 {
            *found_root_dst = [self.found_root];
        } else if self.changed_state == 4 {
            *root_hash_dst = self.root_hash.clone().try_into().unwrap();
            *node_left_dst = self.node_left.clone().try_into().unwrap();
            *node_right_dst = self.node_right.clone().try_into().unwrap();
            *state_dst = self.state.clone().try_into().unwrap();
            *current_round_dst = usize::to_le_bytes(self.current_round);
            *current_round_index_dst = usize::to_le_bytes(self.current_round_index);
            *current_index_dst = usize::to_le_bytes(self.current_index);
            *current_level_dst = usize::to_le_bytes(self.current_level);
            *current_level_hash_dst = self.current_level_hash.clone().try_into().unwrap();
        }
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
    }
}
