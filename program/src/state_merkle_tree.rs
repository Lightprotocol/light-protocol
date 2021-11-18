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
    pub zeros : Vec<Vec<u8>>,
    pub currentRootIndex : usize,
    pub nextIndex : usize,
    pub ROOT_HISTORY_SIZE : usize,
    pub roots : Vec<u8>,
    pub leaves: Vec<u8>,
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
    const LEN: usize = 135057;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, MerkleTree::LEN];

        let (
            is_initialized,
            levels,
            filledSubtrees,
            zeros,
            //713
            currentRootIndex,
            nextIndex,
            ROOT_HISTORY_SIZE,
            //737
            roots,
            //3937
            leaves,
            current_total_deposits,
            nullifiers,
            pubkey_locked,
            time_locked,
        //) = array_refs![input,1, 8, 64, 64 , 8, 8, 8, 320, 320, 8];let clock = Clock::get()
        //) = array_refs![input,1, 8, 576, 576 , 8, 8, 8, 3200, 8388608, 8];
        //height 8
    ) = array_refs![input, 1, 8, 352, 352, 8, 8, 8, 3200, 65536, 8, 65536, 32, 8];
        assert_eq!(1, is_initialized[0], "Account is not initialized");
        /*
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };*/
        sol_log_compute_units();
        msg!("unpack merkle_tree1");

        //let mut tmp_subtree_vec = vec![vec![0u8;32]; 18];
        let mut tmp_subtree_vec = vec![vec![0u8;32]; 11];

        for (i, bytes) in filledSubtrees.chunks(32).enumerate() {
            tmp_subtree_vec[i] = bytes.to_vec();
        }
        sol_log_compute_units();
        msg!("unpack merkle_tree2");

        //let mut tmp_zeros_vec = vec![vec![0u8;32]; 18];
        let mut tmp_zeros_vec = vec![vec![0u8;32]; 11];

        for (i, bytes) in zeros.chunks(32).enumerate() {
            tmp_zeros_vec[i] = bytes.to_vec();
        }
        sol_log_compute_units();
        msg!("unpack merkle_tree3");
        /*
        let mut tmp_roots_vec = vec![vec![0u8;32]; 10];
        for (i, bytes) in roots.chunks(32).enumerate() {
            tmp_roots_vec[i] = bytes.to_vec();
        }*/
        let current_root_index = usize::from_le_bytes(*currentRootIndex);

        let mut tmp_roots_vec = vec![0u8;32];
        let current_root_start_range = current_root_index * 32;
        let current_root_end_range = (current_root_index + 1) * 32;
        msg!("Start {}, End {}", current_root_start_range, current_root_end_range);
        for (i, byte) in roots[current_root_start_range..current_root_end_range].iter().enumerate() {
            tmp_roots_vec[i] = *byte;
        }
        sol_log_compute_units();
        msg!("unpack merkle_tree4");
        /*
        let mut tmp_leaves_vec = vec![vec![0u8;32]; 10];
        for (i, bytes) in leaves.chunks(32).enumerate() {
            tmp_leaves_vec[i] = bytes.to_vec();
        }*/
        let nextIndex = usize::from_le_bytes(*nextIndex);
        let mut tmp_leaves_vec = vec![0u8;32];
        let current_leave_start_range = nextIndex * 32;
        let current_leave_end_range = (nextIndex + 1) * 32;
        msg!("Start {}, End {}", current_leave_start_range, current_leave_end_range);
        for (i, byte) in leaves[current_leave_start_range..current_leave_end_range].iter().enumerate() {
            tmp_leaves_vec[i] = *byte;
        }
        msg!("leaf: {:?}", tmp_leaves_vec);
        sol_log_compute_units();
        Ok(
            MerkleTree {
                is_initialized: true,
                levels: usize::from_le_bytes(*levels),
                filledSubtrees: tmp_subtree_vec,
                zeros : tmp_zeros_vec,
                currentRootIndex : current_root_index,
                nextIndex : nextIndex,
                ROOT_HISTORY_SIZE : usize::from_le_bytes(*ROOT_HISTORY_SIZE),
                roots : tmp_roots_vec.to_vec(),
                leaves: tmp_leaves_vec.to_vec(),
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
                mut zeros_dst,
                mut currentRootIndex_dst,
                mut nextIndex_dst,
                mut ROOT_HISTORY_SIZE_dst,
                mut roots_dst,
                mut leaves_dst,
                current_total_deposits_dst,
                nullifiers_dst,
                pubkey_locked_dst,
                time_locked_dst,
            //heigh 2
            //) = mut_array_refs![dst,1, 8, 64, 64 , 8, 8, 8, 320, 320, 8];
            //height 18
            //) = mut_array_refs![dst,1, 8, 576, 576 , 8, 8, 8, 3200, 8388608, 8];
            //height 8
        ) = mut_array_refs![dst, 1, 8, 352, 352, 8, 8, 8, 3200, 65536, 8, 65536, 32, 8];
            let mut i = 0;
            for it in &self.filledSubtrees {
                for j in it {
                    filledSubtrees_dst[i] = *j;
                    i += 1;
                }
            }

            let mut i = 0;
            for it in &self.zeros {
                for j in it {
                    zeros_dst[i] = *j;
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
            } else {
                *roots_dst = *roots_dst;
            }

            if self.inserted_leaf {
                let mut i = 0;
                if self.nextIndex != 0 {
                    //i = (self.nextIndex - 1);
                    i = (self.nextIndex);
                }
                let mut tmp_i = i *32;
                for leaf_byte in self.leaves.iter() {
                    leaves_dst[tmp_i] = *leaf_byte;
                    tmp_i += 1;
                }
                msg!("leaf: {:?}", self.leaves);
            } else {
                *leaves_dst = *leaves_dst;
            }
            *nullifiers_dst = *nullifiers_dst;

            //assert_eq!()
            //should change u64 to usize
            LittleEndian::write_u64(currentRootIndex_dst, self.currentRootIndex.try_into().unwrap());
            LittleEndian::write_u64(nextIndex_dst, self.nextIndex.try_into().unwrap());
            LittleEndian::write_u64(current_total_deposits_dst, self.current_total_deposits.try_into().unwrap());
            *levels_dst = *levels_dst;
            *ROOT_HISTORY_SIZE_dst = *ROOT_HISTORY_SIZE_dst;
            *is_initialized_dst = *is_initialized_dst;
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
    const LEN: usize = 135057;

    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, InitMerkleTreeBytes::LEN];

        let (
            bytes,
            left_over,
        //) = array_refs![input, 193, 616];
        //) = array_refs![input, 1217, 8391784];
        ) = array_refs![input, 769, 134288];
        msg!("{:?}", bytes[0]);
        //assert_eq!(bytes[0], 0, "Tree is already initialized");
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
        //) = mut_array_refs![dst, 1217, 8391784];
        ) = mut_array_refs![dst, 769, 134288];

        *left_over_dst = *left_over_dst;
        *bytes_dst =    self.bytes.clone().try_into().unwrap();
        //*is_initialized_dst = [0u8;1];

    }
}

#[derive(Debug)]
pub struct NullifierBytes {
    pub is_initialized: bool,
    pub bytes: Vec<u8>,
}
impl Sealed for NullifierBytes {}
impl IsInitialized for NullifierBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for NullifierBytes {
    //const LEN: usize = 809;
    //const LEN: usize = 8393001;
    const LEN: usize = 135057;

    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, NullifierBytes::LEN];

        let (
            bytes,
            left_over,
        //) = array_refs![input, 193, 616];
        //) = array_refs![input, 1217, 8391784];
        ) = array_refs![input, 769, 134288];

        assert_eq!(bytes[0], 0, "Tree is already initialized");
        Ok(
            NullifierBytes {
                is_initialized: true,
                bytes: bytes.to_vec(),
            }
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0, NullifierBytes::LEN];

        let (
            bytes_dst,
            left_over_dst,
        //) = mut_array_refs![dst, 1217, 8391784];
        ) = mut_array_refs![dst, 769, 134288];

        *left_over_dst = *left_over_dst;
        *bytes_dst =    self.bytes.clone().try_into().unwrap();
        //*is_initialized_dst = [0u8;1];

    }
}

// Account structs for merkle tree:
#[derive(Debug)]
pub struct HashBytes {
    pub is_initialized: bool,
    pub state_range_1: Vec<u8>,
    pub state_range_2: Vec<u8>,
    pub state_range_3: Vec<u8>,
    //pub result: Vec<u8>,
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
    const LEN: usize = 217;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, HashBytes::LEN];

        let (
            state_range_1,
            state_range_2,
            state_range_3,
            left,
            right,
            currentLevelHash,
            currentIndex,
            currentLevel,
            current_instruction_index,
            is_initialized,
        ) = array_refs![input,32,32,32,32,32,32,8,8,8,1];

        Ok(
            HashBytes {
                is_initialized: true,
                state_range_1: state_range_1.to_vec(),
                state_range_2: state_range_2.to_vec(),
                state_range_3: state_range_3.to_vec(),
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
            state_range_1_dst,
            state_range_2_dst,
            state_range_3_dst,
            left_dst,
            right_dst,
            currentLevelHash_dst,
            currentIndex_dst,
            currentLevel_dst,
            current_instruction_index_dst,
            is_initialized_dst,
        ) = mut_array_refs![dst,32,32,32,32,32,32,8,8,8,1];

        *state_range_1_dst =    self.state_range_1.clone().try_into().unwrap();
        //assert_eq!(*state_range_1_dst.to_vec(), self.state_range_1);
        *state_range_2_dst =    self.state_range_2.clone().try_into().unwrap();
        //assert_eq!(*state_range_2_dst.to_vec(), self.state_range_2);
        *state_range_3_dst =    self.state_range_3.clone().try_into().unwrap();
        //assert_eq!(*state_range_3_dst.to_vec(), self.state_range_3);
        //*left_dst = *left_dst;
        *left_dst =             self.left.clone().try_into().unwrap();
        //assert_eq!(*left_dst.to_vec(), self.left);
        //*right_dst = *right_dst ;
        *right_dst =            self.right.clone().try_into().unwrap();
        //*currentLevelHash_dst = *currentLevelHash_dst;
        *currentLevelHash_dst = self.currentLevelHash.clone().try_into().unwrap();
        //msg!("packed level hash input: {:?}", self.currentLevelHash);

        //msg!("packed level hash in dst: {:?}", currentLevelHash_dst);
        //*currentIndex_dst = *currentIndex_dst;
        *currentIndex_dst = usize::to_le_bytes(self.currentIndex);
        //*currentLevel_dst = *currentLevel_dst;
        *currentLevel_dst = usize::to_le_bytes(self.currentLevel);

        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);

        *is_initialized_dst = [1u8; 1];

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

pub const INIT_DATA_MERKLE_TREE_HEIGHT_18 : [u8; 1217]= [1, 18, 0, 0, 0, 0, 0, 0, 0, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 98, 54, 90, 210, 36, 244, 233, 73, 165, 175, 0, 188, 155, 74, 134, 124, 136, 171, 169, 215, 147, 139, 165, 144, 98, 254, 218, 107, 202, 111, 128, 182, 6, 236, 174, 238, 84, 10, 176, 105, 110, 148, 105, 11, 165, 208, 125, 45, 48, 66, 35, 190, 135, 7, 87, 232, 16, 205, 58, 156, 105, 91, 182, 205, 63, 254, 19, 101, 63, 189, 205, 121, 25, 25, 51, 214, 99, 21, 103, 50, 17, 20, 157, 199, 140, 174, 16, 68, 225, 144, 189, 87, 111, 32, 205, 101, 22, 25, 32, 43, 138, 68, 60, 76, 241, 255, 254, 60, 206, 126, 58, 35, 25, 212, 210, 196, 231, 4, 79, 114, 197, 126, 185, 2, 172, 7, 52, 1, 68, 10, 85, 52, 14, 122, 173, 206, 241, 130, 56, 74, 22, 247, 5, 197, 57, 53, 217, 148, 76, 0, 172, 241, 27, 181, 248, 181, 153, 109, 196, 123, 88, 0, 194, 200, 151, 183, 155, 185, 213, 254, 109, 156, 0, 72, 122, 249, 104, 94, 178, 21, 23, 15, 74, 69, 3, 10, 15, 64, 228, 6, 245, 203, 102, 208, 60, 19, 97, 112, 252, 248, 249, 208, 219, 197, 79, 96, 59, 249, 51, 78, 96, 45, 35, 170, 199, 156, 216, 166, 197, 121, 195, 182, 177, 131, 82, 3, 229, 113, 147, 188, 23, 236, 85, 20, 201, 235, 167, 220, 60, 22, 17, 166, 249, 213, 162, 64, 140, 102, 101, 219, 30, 18, 65, 36, 210, 159, 70, 211, 74, 172, 171, 42, 167, 203, 244, 200, 191, 20, 107, 197, 220, 107, 18, 36, 52, 98, 134, 124, 8, 30, 164, 208, 137, 113, 162, 26, 190, 110, 112, 42, 169, 149, 71, 205, 5, 105, 192, 83, 33, 236, 114, 78, 86, 75, 94, 156, 21, 7, 4, 81, 217, 88, 231, 71, 223, 64, 216, 118, 157, 215, 88, 59, 219, 140, 26, 140, 25, 114, 178, 79, 3, 96, 33, 175, 121, 189, 26, 248, 83, 186, 123, 53, 160, 91, 140, 55, 158, 234, 184, 141, 78, 197, 54, 228, 51, 141, 60, 183, 106, 159, 66, 189, 222, 56, 191, 236, 102, 37, 213, 10, 80, 52, 215, 39, 11, 202, 108, 152, 8, 20, 211, 229, 21, 119, 51, 146, 6, 197, 33, 196, 229, 178, 206, 227, 92, 105, 40, 252, 198, 119, 133, 138, 176, 31, 5, 241, 121, 44, 5, 84, 30, 13, 81, 235, 43, 180, 28, 143, 115, 147, 106, 68, 216, 45, 235, 203, 180, 183, 139, 70, 4, 134, 4, 159, 214, 6, 14, 162, 39, 61, 74, 169, 165, 181, 52, 86, 132, 62, 67, 14, 139, 169, 161, 29, 110, 168, 64, 253, 37, 226, 119, 134, 207, 69, 42, 174, 109, 66, 67, 9, 11, 183, 137, 246, 157, 85, 182, 175, 191, 188, 47, 79, 190, 190, 35, 211, 73, 180, 1, 93, 92, 1, 23, 192, 85, 212, 25, 208, 220, 56, 65, 13, 179, 240, 1, 64, 229, 104, 91, 170, 81, 250, 2, 198, 173, 123, 25, 67, 123, 197, 195, 30, 221, 128, 180, 123, 1, 79, 150, 75, 235, 106, 246, 158, 18, 199, 132, 102, 110, 168, 70, 94, 54, 120, 84, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 98, 54, 90, 210, 36, 244, 233, 73, 165, 175, 0, 188, 155, 74, 134, 124, 136, 171, 169, 215, 147, 139, 165, 144, 98, 254, 218, 107, 202, 111, 128, 182, 6, 236, 174, 238, 84, 10, 176, 105, 110, 148, 105, 11, 165, 208, 125, 45, 48, 66, 35, 190, 135, 7, 87, 232, 16, 205, 58, 156, 105, 91, 182, 205, 63, 254, 19, 101, 63, 189, 205, 121, 25, 25, 51, 214, 99, 21, 103, 50, 17, 20, 157, 199, 140, 174, 16, 68, 225, 144, 189, 87, 111, 32, 205, 101, 22, 25, 32, 43, 138, 68, 60, 76, 241, 255, 254, 60, 206, 126, 58, 35, 25, 212, 210, 196, 231, 4, 79, 114, 197, 126, 185, 2, 172, 7, 52, 1, 68, 10, 85, 52, 14, 122, 173, 206, 241, 130, 56, 74, 22, 247, 5, 197, 57, 53, 217, 148, 76, 0, 172, 241, 27, 181, 248, 181, 153, 109, 196, 123, 88, 0, 194, 200, 151, 183, 155, 185, 213, 254, 109, 156, 0, 72, 122, 249, 104, 94, 178, 21, 23, 15, 74, 69, 3, 10, 15, 64, 228, 6, 245, 203, 102, 208, 60, 19, 97, 112, 252, 248, 249, 208, 219, 197, 79, 96, 59, 249, 51, 78, 96, 45, 35, 170, 199, 156, 216, 166, 197, 121, 195, 182, 177, 131, 82, 3, 229, 113, 147, 188, 23, 236, 85, 20, 201, 235, 167, 220, 60, 22, 17, 166, 249, 213, 162, 64, 140, 102, 101, 219, 30, 18, 65, 36, 210, 159, 70, 211, 74, 172, 171, 42, 167, 203, 244, 200, 191, 20, 107, 197, 220, 107, 18, 36, 52, 98, 134, 124, 8, 30, 164, 208, 137, 113, 162, 26, 190, 110, 112, 42, 169, 149, 71, 205, 5, 105, 192, 83, 33, 236, 114, 78, 86, 75, 94, 156, 21, 7, 4, 81, 217, 88, 231, 71, 223, 64, 216, 118, 157, 215, 88, 59, 219, 140, 26, 140, 25, 114, 178, 79, 3, 96, 33, 175, 121, 189, 26, 248, 83, 186, 123, 53, 160, 91, 140, 55, 158, 234, 184, 141, 78, 197, 54, 228, 51, 141, 60, 183, 106, 159, 66, 189, 222, 56, 191, 236, 102, 37, 213, 10, 80, 52, 215, 39, 11, 202, 108, 152, 8, 20, 211, 229, 21, 119, 51, 146, 6, 197, 33, 196, 229, 178, 206, 227, 92, 105, 40, 252, 198, 119, 133, 138, 176, 31, 5, 241, 121, 44, 5, 84, 30, 13, 81, 235, 43, 180, 28, 143, 115, 147, 106, 68, 216, 45, 235, 203, 180, 183, 139, 70, 4, 134, 4, 159, 214, 6, 14, 162, 39, 61, 74, 169, 165, 181, 52, 86, 132, 62, 67, 14, 139, 169, 161, 29, 110, 168, 64, 253, 37, 226, 119, 134, 207, 69, 42, 174, 109, 66, 67, 9, 11, 183, 137, 246, 157, 85, 182, 175, 191, 188, 47, 79, 190, 190, 35, 211, 73, 180, 1, 93, 92, 1, 23, 192, 85, 212, 25, 208, 220, 56, 65, 13, 179, 240, 1, 64, 229, 104, 91, 170, 81, 250, 2, 198, 173, 123, 25, 67, 123, 197, 195, 30, 221, 128, 180, 123, 1, 79, 150, 75, 235, 106, 246, 158, 18, 199, 132, 102, 110, 168, 70, 94, 54, 120, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 161, 176, 197, 20, 174, 171, 203, 17, 137, 83, 217, 10, 88, 93, 236, 147, 251, 91, 243, 48, 170, 251, 113, 82, 234, 73, 241, 51, 30, 208, 215, 24];

pub const INIT_DATA_MERKLE_TREE_HEIGHT_11 : [u8; 769] = [1, 11, 0, 0, 0, 0, 0, 0, 0, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 98, 54, 90, 210, 36, 244, 233, 73, 165, 175, 0, 188, 155, 74, 134, 124, 136, 171, 169, 215, 147, 139, 165, 144, 98, 254, 218, 107, 202, 111, 128, 182, 6, 236, 174, 238, 84, 10, 176, 105, 110, 148, 105, 11, 165, 208, 125, 45, 48, 66, 35, 190, 135, 7, 87, 232, 16, 205, 58, 156, 105, 91, 182, 205, 63, 254, 19, 101, 63, 189, 205, 121, 25, 25, 51, 214, 99, 21, 103, 50, 17, 20, 157, 199, 140, 174, 16, 68, 225, 144, 189, 87, 111, 32, 205, 101, 22, 25, 32, 43, 138, 68, 60, 76, 241, 255, 254, 60, 206, 126, 58, 35, 25, 212, 210, 196, 231, 4, 79, 114, 197, 126, 185, 2, 172, 7, 52, 1, 68, 10, 85, 52, 14, 122, 173, 206, 241, 130, 56, 74, 22, 247, 5, 197, 57, 53, 217, 148, 76, 0, 172, 241, 27, 181, 248, 181, 153, 109, 196, 123, 88, 0, 194, 200, 151, 183, 155, 185, 213, 254, 109, 156, 0, 72, 122, 249, 104, 94, 178, 21, 23, 15, 74, 69, 3, 10, 15, 64, 228, 6, 245, 203, 102, 208, 60, 19, 97, 112, 252, 248, 249, 208, 219, 197, 79, 96, 59, 249, 51, 78, 96, 45, 35, 170, 199, 156, 216, 166, 197, 121, 195, 182, 177, 131, 82, 3, 229, 113, 147, 188, 23, 236, 85, 20, 201, 235, 167, 220, 60, 22, 17, 166, 249, 213, 162, 64, 140, 102, 101, 219, 30, 18, 65, 36, 210, 159, 70, 211, 74, 172, 171, 42, 167, 203, 244, 200, 191, 20, 107, 197, 220, 107, 18, 36, 52, 98, 134, 124, 8, 30, 164, 208, 137, 113, 162, 26, 190, 110, 112, 42, 169, 149, 71, 205, 5, 105, 192, 83, 33, 236, 114, 78, 86, 75, 94, 156, 21, 7, 4, 81, 217, 88, 231, 71, 223, 64, 216, 118, 157, 215, 88, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 98, 54, 90, 210, 36, 244, 233, 73, 165, 175, 0, 188, 155, 74, 134, 124, 136, 171, 169, 215, 147, 139, 165, 144, 98, 254, 218, 107, 202, 111, 128, 182, 6, 236, 174, 238, 84, 10, 176, 105, 110, 148, 105, 11, 165, 208, 125, 45, 48, 66, 35, 190, 135, 7, 87, 232, 16, 205, 58, 156, 105, 91, 182, 205, 63, 254, 19, 101, 63, 189, 205, 121, 25, 25, 51, 214, 99, 21, 103, 50, 17, 20, 157, 199, 140, 174, 16, 68, 225, 144, 189, 87, 111, 32, 205, 101, 22, 25, 32, 43, 138, 68, 60, 76, 241, 255, 254, 60, 206, 126, 58, 35, 25, 212, 210, 196, 231, 4, 79, 114, 197, 126, 185, 2, 172, 7, 52, 1, 68, 10, 85, 52, 14, 122, 173, 206, 241, 130, 56, 74, 22, 247, 5, 197, 57, 53, 217, 148, 76, 0, 172, 241, 27, 181, 248, 181, 153, 109, 196, 123, 88, 0, 194, 200, 151, 183, 155, 185, 213, 254, 109, 156, 0, 72, 122, 249, 104, 94, 178, 21, 23, 15, 74, 69, 3, 10, 15, 64, 228, 6, 245, 203, 102, 208, 60, 19, 97, 112, 252, 248, 249, 208, 219, 197, 79, 96, 59, 249, 51, 78, 96, 45, 35, 170, 199, 156, 216, 166, 197, 121, 195, 182, 177, 131, 82, 3, 229, 113, 147, 188, 23, 236, 85, 20, 201, 235, 167, 220, 60, 22, 17, 166, 249, 213, 162, 64, 140, 102, 101, 219, 30, 18, 65, 36, 210, 159, 70, 211, 74, 172, 171, 42, 167, 203, 244, 200, 191, 20, 107, 197, 220, 107, 18, 36, 52, 98, 134, 124, 8, 30, 164, 208, 137, 113, 162, 26, 190, 110, 112, 42, 169, 149, 71, 205, 5, 105, 192, 83, 33, 236, 114, 78, 86, 75, 94, 156, 21, 7, 4, 81, 217, 88, 231, 71, 223, 64, 216, 118, 157, 215, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 59, 219, 140, 26, 140, 25, 114, 178, 79, 3, 96, 33, 175, 121, 189, 26, 248, 83, 186, 123, 53, 160, 91, 140, 55, 158, 234, 184, 141, 78, 197, 54];

const init_merkle_tree_height_2 : [u8;193] =[1, 2, 0, 0, 0, 0, 0, 0, 0, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 44, 207, 149, 93, 99, 198, 128, 46, 55, 231, 28, 179, 93, 178, 0, 130, 236, 14, 160, 231, 117, 60, 23, 25, 204, 92, 26, 66, 89, 198, 110, 205, 66, 19, 97, 209, 41, 177, 54, 187, 0, 170, 207, 82, 55, 238, 205, 83, 242, 219, 137, 212, 108, 15, 248, 123, 138, 104, 142, 194, 176, 163, 221, 34, 44, 207, 149, 93, 99, 198, 128, 46, 55, 231, 28, 179, 93, 178, 0, 130, 236, 14, 160, 231, 117, 60, 23, 25, 204, 92, 26, 66, 89, 198, 110, 205, 66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 236, 92, 190, 167, 184, 114, 237, 0, 240, 226, 55, 7, 189, 145, 108, 47, 150, 69, 247, 94, 168, 45, 151, 166, 245, 83, 87, 239, 95, 232, 109, 170];



pub const INSTRUCTION_ARRAY_INSERT_MERKLE_TREE_HEIGHT_11 : [u8;311] =  [34, 24, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 25, 27, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 33, 19, 20, 21, 23, 26];
