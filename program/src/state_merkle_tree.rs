use solana_program::{
    msg,
    pubkey::Pubkey,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
};
use std::convert::TryInto;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::LittleEndian;
use byteorder::ByteOrder;

#[derive(Debug)]
pub struct MerkleTree {
    pub is_initialized: bool,
    pub levels: usize,
    pub filledSubtrees : Vec<Vec<u8>>,
    pub currentRootIndex : usize,
    pub nextIndex : usize,
    pub ROOT_HISTORY_SIZE : usize,
    pub roots : Vec<u8>,
    pub current_total_deposits: u64,
    pub inserted_leaf: bool,
    pub inserted_root: bool,
    pub time_locked: u64,
    pub pubkey_locked:Vec<u8>,

}
impl Sealed for MerkleTree {}
impl IsInitialized for MerkleTree {
    fn is_initialized(&self) -> bool {
        self.is_initialized

    }
}
impl Pack for MerkleTree {
    //height 2
    //const LEN: usize = 809;
    //height 18 8392993
    //const LEN: usize = 8393001;
    //height 11
    const LEN: usize = 16657;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, MerkleTree::LEN];

        let (
            is_initialized,
            levels,
            filledSubtrees,
            currentRootIndex,
            nextIndex,
            ROOT_HISTORY_SIZE,
            //609
            roots,
            //18137
            current_total_deposits,
            pubkey_locked,
            time_locked,
    ) = array_refs![input, 1, 8, 576, 8, 8, 8, 16000, 8, 32, 8];
        assert_eq!(1, is_initialized[0], "Account is not initialized");
        /*
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };*/

        let mut tmp_subtree_vec = vec![vec![0u8;32]; 18];

        for (i, bytes) in filledSubtrees.chunks(32).enumerate() {
            tmp_subtree_vec[i] = bytes.to_vec();
        }

        let current_root_index = usize::from_le_bytes(*currentRootIndex);

        let mut tmp_roots_vec = vec![0u8;32];
        let current_root_start_range = current_root_index * 32;
        let current_root_end_range = (current_root_index + 1) * 32;
        for (i, byte) in roots[current_root_start_range..current_root_end_range].iter().enumerate() {
            tmp_roots_vec[i] = *byte;
        }

        let nextIndex = usize::from_le_bytes(*nextIndex);

        sol_log_compute_units();
        Ok(
            MerkleTree {
                is_initialized: true,
                levels: usize::from_le_bytes(*levels),
                filledSubtrees: tmp_subtree_vec,
                currentRootIndex : current_root_index,
                nextIndex : nextIndex,
                ROOT_HISTORY_SIZE : usize::from_le_bytes(*ROOT_HISTORY_SIZE),
                roots : tmp_roots_vec.to_vec(),
                current_total_deposits: LittleEndian::read_u64(current_total_deposits),
                inserted_leaf: false,
                inserted_root: false,
                pubkey_locked: pubkey_locked.to_vec(),
                time_locked: u64::from_le_bytes(*time_locked),
            }
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {


        //if self.inserted_leaf {
            let dst = array_mut_ref![dst, 0,  MerkleTree::LEN];

            let (
                mut is_initialized_dst,
                mut levels_dst,
                mut filledSubtrees_dst,
                mut currentRootIndex_dst,
                mut nextIndex_dst,
                mut ROOT_HISTORY_SIZE_dst,
                mut roots_dst,
                current_total_deposits_dst,
                pubkey_locked_dst,
                time_locked_dst,
        ) = mut_array_refs![dst, 1, 8, 576, 8, 8, 8, 16000, 8, 32, 8];

            // could change this to insert only the changed subtree if one is changed
            let mut i = 0;
            for it in &self.filledSubtrees {
                for j in it {
                    filledSubtrees_dst[i] = *j;
                    i += 1;
                }
            }
            if self.inserted_root {
                let mut i = 0;
                if self.currentRootIndex != 0 {
                    i = (self.currentRootIndex) ;
                }
                let mut i_tmp = i * 32;
                for it in self.roots.iter() {
                    roots_dst[i_tmp] = *it;
                    i_tmp += 1;

                }
            }

            //should change u64 to usize
            LittleEndian::write_u64(currentRootIndex_dst, self.currentRootIndex.try_into().unwrap());
            LittleEndian::write_u64(nextIndex_dst, self.nextIndex.try_into().unwrap());
            LittleEndian::write_u64(current_total_deposits_dst, self.current_total_deposits.try_into().unwrap());
            *pubkey_locked_dst = self.pubkey_locked.clone().try_into().unwrap();

            LittleEndian::write_u64(time_locked_dst, self.time_locked.try_into().unwrap());
    }
}



#[derive(Debug)]
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
    //const LEN: usize = 809;
    //const LEN: usize = 8393001;
    const LEN: usize = 16657;

    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, InitMerkleTreeBytes::LEN];

        let (
            bytes,
            left_over,
        ) = array_refs![input, 641, 16016];
        msg!("unpacking init bytes");
        assert_eq!(bytes[0], 0, "Tree is already initialized");
        Ok(
            InitMerkleTreeBytes {
                is_initialized: true,
                bytes: bytes.to_vec(),
            }
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0, InitMerkleTreeBytes::LEN];

        let (
            bytes_dst,
            left_over_dst,
        ) = mut_array_refs![dst, 641, 16016];

        *bytes_dst =    self.bytes.clone().try_into().unwrap();
    }
}

// Account structs for merkle tree:
#[derive(Debug)]
pub struct HashBytes {
    pub is_initialized: bool,
    pub state: Vec<Vec<u8>>,
    pub current_round: usize,
    pub current_round_index: usize,
    //pub result: Vec<u8>,
    pub leaf_left: Vec<u8>,
    pub leaf_right: Vec<u8>,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
    pub currentLevelHash: Vec<u8>,
    pub currentIndex: usize,
    pub currentLevel: usize,
    pub current_instruction_index: usize,
    // levels,
    // filledSubtrees,
    // zeros,
    //new_root,
}
impl Sealed for HashBytes {}
impl IsInitialized for HashBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for HashBytes {
    const LEN: usize = 3900;//297;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, HashBytes::LEN];

        let (
            is_initialized,
            unused_remainder0,
            current_instruction_index,
            //220
            unused_remainder1,

            state,
            current_round,
            current_round_index,
            left,
            right,
            currentLevelHash,
            currentIndex,
            currentLevel,
            leaf_left,
            leaf_right,
            nullifier_0,
            nullifier_1,
        ) = array_refs![input, 1, 211, 8, 3328, 96, 8 , 8, 32, 32, 32, 8, 8, 32, 32, 32, 32];

        let mut parsed_state = Vec::new();
        for i in state.chunks(32) {
            parsed_state.push(i.to_vec());
        }

        Ok(
            HashBytes {
                is_initialized: true,
                state: parsed_state.to_vec(),
                current_round: usize::from_le_bytes(*current_round),
                current_round_index: usize::from_le_bytes(*current_round_index),
                leaf_left: leaf_left.to_vec(),
                leaf_right: leaf_right.to_vec(),
                left: left.to_vec(),
                right: right.to_vec(),
                currentLevelHash: currentLevelHash.to_vec(),
                currentIndex: usize::from_le_bytes(*currentIndex),
                currentLevel: usize::from_le_bytes(*currentLevel),
                current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            }
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0,  HashBytes::LEN];

        let (
            is_initialized_dst,
            unused_remainder0_dst,
            current_instruction_index_dst,
            //220
            unused_remainder1_dst,

            state_dst,
            current_round_dst,
            current_round_index_dst,
            left_dst,
            right_dst,
            currentLevelHash_dst,
            currentIndex_dst,
            currentLevel_dst,
            leaf_left_dst,
            leaf_right_dst,
            //+288
            nullifier_0_dst,
            nullifier_1_dst,
        ) = mut_array_refs![dst, 1, 211, 8, 3328, 96, 8 , 8, 32, 32, 32, 8, 8, 32, 32, 32, 32];

        let mut state_tmp = [0u8;96];
        let mut z = 0;
        for i in self.state.iter() {
            for j in i {
                state_tmp[z] = *j;
                z +=1;
            }
        }

        *state_dst = state_tmp;
        *current_round_dst = usize::to_le_bytes(self.current_round);
        *current_round_index_dst= usize::to_le_bytes(self.current_round_index);

        *leaf_left_dst =             self.leaf_left.clone().try_into().unwrap();

        *leaf_right_dst =            self.leaf_right.clone().try_into().unwrap();
        msg!("self.left {:?}", self.left);
        *left_dst =             self.left.clone().try_into().unwrap();

        *right_dst =            self.right.clone().try_into().unwrap();
        *currentLevelHash_dst = self.currentLevelHash.clone().try_into().unwrap();

        *currentIndex_dst = usize::to_le_bytes(self.currentIndex);
        *currentLevel_dst = usize::to_le_bytes(self.currentLevel);
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
}

impl Sealed for TwoLeavesBytesPda {}
impl IsInitialized for TwoLeavesBytesPda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for TwoLeavesBytesPda {
    const LEN: usize = 98;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, TwoLeavesBytesPda::LEN];

        let (
            is_initialized,
            account_type,
            leaf_left,
            leaf_right,
            merkle_tree_pubkey,
        ) = array_refs![input, 1, 1, 32, 32, 32];
        //check that account was not initialized before
        assert_eq!(is_initialized[0], 0);
        Ok(
            TwoLeavesBytesPda {
                is_initialized: true,
                account_type: 4,
                leaf_right: vec![0u8;32],
                leaf_left: vec![0u8;32],
                merkle_tree_pubkey: vec![0u8;32],
            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, TwoLeavesBytesPda::LEN];
        let (
            is_initialized_dst,
            account_type_dst,
            leaf_left_dst,
            leaf_right_dst,
            merkle_tree_pubkey_dst,
        ) = mut_array_refs![dst, 1, 1, 32, 32, 32];

        *is_initialized_dst = [1];
        *account_type_dst = [4];
        *leaf_right_dst = self.leaf_right.clone().try_into().unwrap();
        *leaf_left_dst = self.leaf_left.clone().try_into().unwrap();
        *merkle_tree_pubkey_dst = self.merkle_tree_pubkey.clone().try_into().unwrap();
        msg!("packed inserted_leaves");

    }
}



//1217 byte init data for height 18
// total space required init data - one root which is included plus 100 roots in history and 2^18 leaves + total nr of deposits
//1217 - 32 + 100 * 32 + (2**18) * 32 + 8 = 8393001 bytes

//bytes0 of crashed merkletree
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[14,   6,  73, 209, 163, 244, 108,  152, 171, 216,  16, 214, 160, 160,  167, 228, 175, 183, 171, 175, 131,  235, 227, 100, 101, 217, 250,  96,  173,  34,  59,  62];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  204,   6,  61,  15,  40,   7, 133,  24,   55, 165, 136,  46, 236, 123,  41,  40,    7, 209,  56, 229,  89, 150, 182, 223,   28, 161, 254, 127, 128,  43, 190,  48];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [   81, 175,  66, 177, 254, 117,   2, 255,   43, 221,  22, 110, 211, 110, 222,  74,   76,   6, 157,  15, 201,  16, 236, 159,  224,  23,  65,  47, 208,  37, 145,  43];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  120, 217, 238,  22, 243,   6, 113,  70,   21,  94, 232,  35,  44,  13,  63, 196,   55, 240,  76,  57, 204,  56,  73,  31,  120, 216, 106, 177, 105, 126, 146, 176];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  245, 112,  69, 183, 178,  14, 144,  18,  139, 104,  93,  69, 192, 247,  84, 207,  153,  87, 160,  75,  64, 135, 239,  43,  247,  64,  69, 177,  13, 241, 100, 117];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  245, 112,  69, 183, 178,  14, 144,  18,  139, 104,  93,  69, 192, 247,  84, 207,  153,  87, 160,  75,  64, 135, 239,  43,  247,  64,  69, 177,  13, 241, 100, 117];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  172,  35, 191, 213, 227,  79, 87, 138,  176, 102, 184, 228,  69, 159, 79, 215,  208,  59, 148, 226, 119,  30, 79, 182,  215, 157, 183,  24, 184,   7, 84, 118];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[126, 172,  99,  74, 140, 170, 149,  84, 1, 182, 133, 240, 194, 184, 188,  75, 106, 171, 128, 167,  19, 237, 167, 181, 207,  88,  29, 194,  64,  97,  42,  14];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[   60, 116, 160, 179, 184, 158,  24, 255,   95, 137, 245, 130,  79, 227,  94,  63,  222, 123, 229,   5, 161,  89, 124, 141,   27,  45, 192,  72, 158, 106, 180, 197];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  248, 195,  48, 203,   9,  32,  62,  30,  228, 182, 113, 174,   6, 199,  42, 142,   28,  50, 151,  71, 124,  39,  36, 163,  243, 193, 128, 139,  33,   3, 225,  20];
pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[222,  66,  10, 195,  58, 162, 229,  40,
  247,  92,  17,  93,  85, 233,  85, 138,
  197, 136,   2,  65, 208, 158,  38,  39,
  155, 208, 117, 251, 244,  33,  72, 213
];
