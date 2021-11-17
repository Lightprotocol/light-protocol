/*use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ff::{Fp384, Fp256};
// use ark_ec;
use ark_bls12_381;
use ark_ed_on_bls12_381;
use ark_ff::fields::models::quadratic_extension::{QuadExtField, QuadExtParameters};
use num_traits::{One};
use ark_ec;*/
//lib

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
};

use std::convert::TryInto;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

use byteorder::LittleEndian;
use byteorder::ByteOrder;

//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[14,   6,  73, 209, 163, 244, 108,  152, 171, 216,  16, 214, 160, 160,  167, 228, 175, 183, 171, 175, 131,  235, 227, 100, 101, 217, 250,  96,  173,  34,  59,  62];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  204,   6,  61,  15,  40,   7, 133,  24,   55, 165, 136,  46, 236, 123,  41,  40,    7, 209,  56, 229,  89, 150, 182, 223,   28, 161, 254, 127, 128,  43, 190,  48];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [   81, 175,  66, 177, 254, 117,   2, 255,   43, 221,  22, 110, 211, 110, 222,  74,   76,   6, 157,  15, 201,  16, 236, 159,  224,  23,  65,  47, 208,  37, 145,  43];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  120, 217, 238,  22, 243,   6, 113,  70,   21,  94, 232,  35,  44,  13,  63, 196,   55, 240,  76,  57, 204,  56,  73,  31,  120, 216, 106, 177, 105, 126, 146, 176];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  245, 112,  69, 183, 178,  14, 144,  18,  139, 104,  93,  69, 192, 247,  84, 207,  153,  87, 160,  75,  64, 135, 239,  43,  247,  64,  69, 177,  13, 241, 100, 117];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  245, 112,  69, 183, 178,  14, 144,  18,  139, 104,  93,  69, 192, 247,  84, 207,  153,  87, 160,  75,  64, 135, 239,  43,  247,  64,  69, 177,  13, 241, 100, 117];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] = [  172,  35, 191, 213, 227,  79, 87, 138,  176, 102, 184, 228,  69, 159, 79, 215,  208,  59, 148, 226, 119,  30, 79, 182,  215, 157, 183,  24, 184,   7, 84, 118];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[126, 172,  99,  74, 140, 170, 149,  84, 1, 182, 133, 240, 194, 184, 188,  75, 106, 171, 128, 167,  19, 237, 167, 181, 207,  88,  29, 194,  64,  97,  42,  14];
//pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[   60, 116, 160, 179, 184, 158,  24, 255,   95, 137, 245, 130,  79, 227,  94,  63,  222, 123, 229,   5, 161,  89, 124, 141,   27,  45, 192,  72, 158, 106, 180, 197];
pub const MERKLE_TREE_ACC_BYTES: [u8;32] =[  248, 195,  48, 203,   9,  32,  62,  30,  228, 182, 113, 174,   6, 199,  42, 142,   28,  50, 151,  71, 124,  39,  36, 163,  243, 193, 128, 139,  33,   3, 225,  20];

#[derive(Clone, Debug)]
pub struct MerkleTreeRoots {
    pub is_initialized: bool,
    pub roots: Vec<u8>,
    pub ROOT_HISTORY_SIZE: u64,

}

impl Sealed for MerkleTreeRoots {}
impl IsInitialized for MerkleTreeRoots {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeRoots {
    const LEN: usize = 135057;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MerkleTreeRoots::LEN];

        let (
            is_initialized,
            unused_prior,
            ROOT_HISTORY_SIZE,
            roots,
            unused_remainder,

        ) = array_refs![input,1 ,728, 8, 3200, 131120];

        Ok(
            MerkleTreeRoots {
                is_initialized: true,
                roots: roots.to_vec(),
                ROOT_HISTORY_SIZE:  u64::from_le_bytes(*ROOT_HISTORY_SIZE),

            }
        )

    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        //is not meant to be called since this structs purpose is to solely unpack roots
        //to check for the existence of one root

    }
}


pub fn check_root_hash_exists(account_main: &AccountInfo, root_bytes: Vec<u8>, found_root: &mut  u8) {

   let mut account_main_data =  MerkleTreeRoots::unpack(&account_main.data.borrow()).unwrap();
   msg!("merkletree acc key: {:?}", *account_main.key);
   msg!("key to check: {:?}",solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..]) );
   assert_eq!(*account_main.key, solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..]));
   msg!("did not crash {}", account_main_data.ROOT_HISTORY_SIZE);
   assert!(account_main_data.ROOT_HISTORY_SIZE < 593, "root history size to large");
   msg!("looking for root {:?}", root_bytes);
   let mut i = 0;
   let mut counter = 0;
   loop {
       //sol_log_compute_units();
       if  account_main_data.roots[i..i+32]  ==  root_bytes {
           msg!("found root hash index {}", counter);
           *found_root = 1u8;
           break;
       }

       if counter % 10 == 0 {
           msg!("{}", counter);

       }
       i += 32;
       counter +=1;
       if counter == account_main_data.ROOT_HISTORY_SIZE{
           break;
       }
   }
}
