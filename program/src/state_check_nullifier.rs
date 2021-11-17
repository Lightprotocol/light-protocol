use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryInto;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    log::sol_log_compute_units,
};
use solana_program::program_pack::IsInitialized;
use solana_program::program_pack::Pack;
use solana_program::program_pack::Sealed;
use crate::state_merkle_tree;

//instructions for nullifier check for tree of height 11
pub fn check_nullifier_in_range_0(account_main: &AccountInfo, nullifier: &Vec<u8>, found_nullifier: &mut  u8) {
    sol_log_compute_units();

    msg!("inside nullifer check1");
    assert!(*nullifier != vec![0u8;32], "nullifier cannot be [0;32]");
    sol_log_compute_units();

    msg!("inside nullifer check2");
    assert!(*found_nullifier == 0, "found_nullifier should be 0 for not determined yet");
    sol_log_compute_units();
    msg!("inside nullifer check3");
    assert_eq!(*account_main.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
    sol_log_compute_units();

    let mut account_main_data =  MerkleTreeNullifierBytes_0::unpack(&account_main.data.borrow()).unwrap();

    msg!("inside nullifer check4");

    let mut i = 0;
    let mut counter = 0;

    loop {
        //msg!("{:?}  ==  {:?}", account_main_data.nullifier[i..i+32].to_vec(),*nullifier);
       //sol_log_compute_units();
       //the nullifier has already been inserted or found thus break
       if *found_nullifier != 0u8 {
           break;
       }
       //the nullifier has already been inserted thus break
       else if  account_main_data.nullifier[i..i+32].to_vec()  ==  *nullifier {
           msg!("found nullifier hash index {}", counter);
           *found_nullifier = 1u8;
           //break;
       }
       //entered the part of nullifier storage which is zero thus nullifier is not spent yet
       //saving the nullifer and setting found nullifier to 2 which means it is saved
       else if account_main_data.nullifier[i..i+32]  == [0u8;32] {
           *found_nullifier = 2u8;
           for j in 0..32 {

               account_main_data.nullifer_to_be_inserted[j] = nullifier[j].clone();
           }
           account_main_data.nullifier_index_start = i;
           account_main_data.nullifier_index_end = i +32;
           account_main_data.insert_nullifier = true;
       }
       // if counter % 10 == 0 {
       //     msg!("{}", counter);
       //     sol_log_compute_units();
       //
       // }
       i += 32;
       counter +=1;
       //max number of nullifers which can be compared in one compute slot for 20kb mem limit
       if counter == 593 {
           break;
       }
    }
    sol_log_compute_units();
    msg!("inside nullifer check5");

    MerkleTreeNullifierBytes_0::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    sol_log_compute_units();

    msg!("inside nullifer check6");

}


pub fn check_nullifier_in_range_1(account_main: &AccountInfo, nullifier: &Vec<u8>, found_nullifier: &mut  u8) {
    msg!("inside nullifer check1");
    assert!(*nullifier != vec![0u8;32], "nullifier cannot be [0;32]");
    msg!("inside nullifer check2");

    msg!("inside nullifer check3");
    assert_eq!(*account_main.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

    let mut account_main_data =  MerkleTreeNullifierBytes_1::unpack(&account_main.data.borrow()).unwrap();
    msg!("inside nullifer check4");

    let mut i = 0;
    let mut counter = 0;

    loop {
        //msg!("{:?}  ==  {:?}", account_main_data.nullifier[i..i+32].to_vec(),*nullifier);
       //sol_log_compute_units();
       //the nullifier has already been inserted or found thus break
       if *found_nullifier != 0u8 {
           msg!("nullifier already checked in prior tx");
           break;
       }
       //the nullifier has already been inserted thus break
       else if  account_main_data.nullifier[i..i+32].to_vec()  ==  *nullifier {
           msg!("found nullifier hash index {}", counter);
           *found_nullifier = 1u8;
           //assert_eq!(false,true, "nullifier_hash already exists");
           //break;
       }
       //entered the part of nullifier storage which is zero thus nullifier is not spent yet
       //saving the nullifer and setting found nullifier to 2 which means it is saved
       else if account_main_data.nullifier[i..i+32]  == [0u8;32] {
           *found_nullifier = 2u8;
           for j in 0..32 {

               account_main_data.nullifer_to_be_inserted[j] = nullifier[j].clone();
           }
           account_main_data.nullifier_index_start = i;
           account_main_data.nullifier_index_end = i +32;
           account_main_data.insert_nullifier = true;
       }
       if counter % 10 == 0 {
           msg!("{}", counter);

       }
       i += 32;
       counter +=1;
       //max number of nullifers which can be compared in one compute slot for 20kb mem limit
       if counter == 593 {
           break;
       }
    }
    msg!("inside nullifer check5");

    MerkleTreeNullifierBytes_1::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    msg!("inside nullifer check6");

}

pub fn check_nullifier_in_range_2(account_main: &AccountInfo, nullifier: &Vec<u8>, found_nullifier: &mut  u8) {
    msg!("inside nullifer check1");
    assert!(*nullifier != vec![0u8;32], "nullifier cannot be [0;32]");
    msg!("inside nullifer check2");

    msg!("inside nullifer check3");
    assert_eq!(*account_main.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

    let mut account_main_data =  MerkleTreeNullifierBytes_2::unpack(&account_main.data.borrow()).unwrap();
    msg!("inside nullifer check4");

    let mut i = 0;
    let mut counter = 0;

    loop {
        //msg!("{:?}  ==  {:?}", account_main_data.nullifier[i..i+32].to_vec(),*nullifier);
       //sol_log_compute_units();
       //the nullifier has already been inserted or found thus break
       if *found_nullifier != 0u8 {
           msg!("nullifier already checked in prior tx");
           break;
       }
       //the nullifier has already been inserted thus break
       else if  account_main_data.nullifier[i..i+32].to_vec()  ==  *nullifier {
           msg!("found nullifier hash index {}", counter);
           *found_nullifier = 1u8;
           //assert_eq!(false,true, "nullifier_hash already exists");
           //break;
       }
       //entered the part of nullifier storage which is zero thus nullifier is not spent yet
       //saving the nullifer and setting found nullifier to 2 which means it is saved
       else if account_main_data.nullifier[i..i+32]  == [0u8;32] {
           *found_nullifier = 2u8;
           for j in 0..32 {

               account_main_data.nullifer_to_be_inserted[j] = nullifier[j].clone();
           }
           account_main_data.nullifier_index_start = i;
           account_main_data.nullifier_index_end = i +32;
           account_main_data.insert_nullifier = true;
       }
       if counter % 10 == 0 {
           msg!("{}", counter);

       }
       i += 32;
       counter +=1;
       //max number of nullifers which can be compared in one compute slot for 20kb mem limit
       if counter == 593 {
           break;
       }
    }
    msg!("inside nullifer check5");

    MerkleTreeNullifierBytes_2::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    msg!("inside nullifer check6");

}

pub fn check_nullifier_in_range_3(account_main: &AccountInfo, nullifier: &Vec<u8>, found_nullifier: &mut  u8) {
    msg!("inside nullifer check1");
    assert!(*nullifier != vec![0u8;32], "nullifier cannot be [0;32]");
    msg!("inside nullifer check2");
    sol_log_compute_units();
    msg!("inside nullifer check3");
    assert_eq!(*account_main.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));
    sol_log_compute_units();
    let mut account_main_data =  MerkleTreeNullifierBytes_3::unpack(&account_main.data.borrow()).unwrap();
    msg!("inside nullifer check4");
    sol_log_compute_units();
    let mut i = 0;
    let mut counter = 0;

    loop {
        //msg!("{:?}  ==  {:?}", account_main_data.nullifier[i..i+32].to_vec(),*nullifier);
       //sol_log_compute_units();
       //the nullifier has already been inserted or found thus break
       if *found_nullifier != 0u8 {
           msg!("nullifier already checked in prior tx");
           break;
       }
       //the nullifier has already been inserted thus break
       else if  account_main_data.nullifier[i..i+32].to_vec()  ==  *nullifier {
           msg!("found nullifier hash index {}", counter);
           *found_nullifier = 1u8;
           //assert_eq!(false,true, "nullifier_hash already exists");
           //break;
       }
       //entered the part of nullifier storage which is zero thus nullifier is not spent yet
       //saving the nullifer and setting found nullifier to 2 which means it is saved

       else if account_main_data.nullifier[i..i+32]  == [0u8;32] {
           *found_nullifier = 2u8;
           for j in 0..32 {

               account_main_data.nullifer_to_be_inserted[j] = nullifier[j].clone();
           }
           account_main_data.nullifier_index_start = i;
           account_main_data.nullifier_index_end = i +32;
           account_main_data.insert_nullifier = true;
       }
       if counter % 10 == 0 {
           msg!("{}", counter);
           sol_log_compute_units();

       }
       i += 32;
       counter +=1;
       //max number of nullifers which can be compared in one compute slot for 20kb mem limit
       if counter == 269 {
           break;
       }
    }
    msg!("inside nullifer check5");
    sol_log_compute_units();
    MerkleTreeNullifierBytes_3::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    msg!("inside nullifer check6");
    sol_log_compute_units();
}


#[derive(Clone, Debug)]
pub struct MerkleTreeNullifierBytes_0 {
    pub is_initialized: bool,
    pub insert_nullifier: bool,
    pub nullifer_to_be_inserted: Vec<u8>,
    pub nullifier: Vec<u8>,
    pub number_of_nullifiers: u64,
    pub nullifier_index_start: usize,
    pub nullifier_index_end: usize,
}

impl Sealed for MerkleTreeNullifierBytes_0 {}
impl IsInitialized for MerkleTreeNullifierBytes_0 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeNullifierBytes_0 {
    const LEN: usize = 135057;// 65545;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MerkleTreeNullifierBytes_0::LEN];

        let (
            is_initialized,
            unused_remainder0,

            unused_remainder1,

            number_of_nullifiers,

            nullifier,
            unused_remainder2,
        ) = array_refs![input,1,3936, 65536, 8, 18976, 46600]; // 65536
        assert_eq!(is_initialized[0], 1);
        Ok(
            MerkleTreeNullifierBytes_0 {
                is_initialized: true,
                insert_nullifier: false,
                nullifer_to_be_inserted: vec![0u8;32],
                nullifier: nullifier.to_vec(),
                number_of_nullifiers:  u64::from_le_bytes(*number_of_nullifiers),
                nullifier_index_start: 0,
                nullifier_index_end: 0,
            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeNullifierBytes_0::LEN];

        let (
            //unused_prior,
            is_initialized_dst,
            unused_remainder0_dst,

            unused_remainder1_dst,

            number_of_nullifiers_dst,

            nullifier_dst,
            unused_remainder2_dst,
        ) = mut_array_refs![dst,1,3936, 65536, 8, 18976, 46600];

        if self.insert_nullifier == true {
            // msg!("packed nullifier_index_start {}",self.nullifier_index_start );
            // msg!("packed inserted_nullifier {}", self.nullifier_index_end);
            // for i in 0..18944 {
            //     if i >= self.nullifier_index_start && i < self.nullifier_index_end {
            //         nullifier_dst[i] = self.nullifer_to_be_inserted[i%32].clone();
            //         msg!("index {}", i);
            //     } else {
            //         nullifier_dst[i] = nullifier_dst[i];
            //     }
            //     if i % 1000 == 0 {
            //         msg!("index {}", i);
            //         sol_log_compute_units();
            //     }
            // }
            // nullifier_dst[..self.nullifier_index_start].to_vec() = nullifier_dst[..self.nullifier_index_start].to_vec();
            // nullifier_dst[self.nullifier_index_start..self.nullifier_index_end].to_vec()= self.nullifer_to_be_inserted.clone()
            // nullifier_dst[self.nullifier_index_end..].to_vec() = nullifier_dst[self.nullifier_index_end..].to_vec();
            for (i, elem) in    nullifier_dst[self.nullifier_index_start..self.nullifier_index_end].iter_mut().enumerate() {
                *elem = self.nullifer_to_be_inserted[i].clone();

            }

        } else {
            *nullifier_dst = *nullifier_dst;
        }
        sol_log_compute_units();

        *unused_remainder0_dst = *unused_remainder0_dst;
        *unused_remainder1_dst = *unused_remainder1_dst;

        *unused_remainder2_dst = *unused_remainder2_dst;
        *is_initialized_dst = *is_initialized_dst;
        msg!("packed inserted_nullifier");

    }
}

#[derive(Clone, Debug)]
pub struct MerkleTreeNullifierBytes_1 {
    pub is_initialized: bool,
    pub insert_nullifier: bool,
    pub nullifer_to_be_inserted: Vec<u8>,
    pub nullifier: Vec<u8>,
    pub number_of_nullifiers: u64,
    pub nullifier_index_start: usize,
    pub nullifier_index_end: usize,
}

impl Sealed for MerkleTreeNullifierBytes_1 {}
impl IsInitialized for MerkleTreeNullifierBytes_1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeNullifierBytes_1 {
    const LEN: usize = 135057;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MerkleTreeNullifierBytes_1::LEN];

        let (
            is_initialized,
            unused_remainder0,

            unused_remainder1,

            number_of_nullifiers,

            already_checked_nullifiers,
            nullifier,
            unused_remainder3,
        ) = array_refs![input, 1, 3936, 65536, 8, 18976, 18976, 27624];
        assert_eq!(is_initialized[0], 1);
        Ok(
            MerkleTreeNullifierBytes_1 {
                is_initialized: true,
                insert_nullifier: false,
                nullifer_to_be_inserted: vec![0u8;32],
                nullifier: nullifier.to_vec(),
                number_of_nullifiers:  u64::from_le_bytes(*number_of_nullifiers),
                nullifier_index_start: 0,
                nullifier_index_end: 0,
            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeNullifierBytes_1::LEN];

        let (
            is_initialized_dst,
            unused_remainder0_dst,

            unused_remainder1_dst,

            number_of_nullifiers_dst,

            already_checked_nullifiers_dst,
            nullifier_dst,
            unused_remainder3_dst,
        ) = mut_array_refs![dst, 1, 3936, 65536, 8, 18976, 18976, 27624];

        if self.insert_nullifier == true {
            //msg!("packed nullifier_index_start {}",self.nullifier_index_start );
            //msg!("packed inserted_nullifier {}", self.nullifier_index_end);
            for i in 0..18944 {
                if i >= self.nullifier_index_start && i < self.nullifier_index_end {
                    nullifier_dst[i] = self.nullifer_to_be_inserted[i%32].clone();
                    //msg!("index {}", i);
                } else {
                    nullifier_dst[i] = nullifier_dst[i];
                }
            }
        } else {
            *nullifier_dst = *nullifier_dst;
        }


        *unused_remainder0_dst = *unused_remainder0_dst;
        *unused_remainder1_dst = *unused_remainder1_dst;

        *already_checked_nullifiers_dst = *already_checked_nullifiers_dst;
        *unused_remainder3_dst = *unused_remainder3_dst;
        *is_initialized_dst = *is_initialized_dst;
        msg!("packed inserted_nullifier 1");
    }
}

#[derive(Clone, Debug)]
pub struct MerkleTreeNullifierBytes_2 {
    pub is_initialized: bool,
    pub insert_nullifier: bool,
    pub nullifer_to_be_inserted: Vec<u8>,
    pub nullifier: Vec<u8>,
    pub number_of_nullifiers: u64,
    pub nullifier_index_start: usize,
    pub nullifier_index_end: usize,
}

impl Sealed for MerkleTreeNullifierBytes_2 {}
impl IsInitialized for MerkleTreeNullifierBytes_2 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeNullifierBytes_2 {
    const LEN: usize = 135057;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MerkleTreeNullifierBytes_2::LEN];
        let (
            is_initialized,
            unused_remainder0,

            unused_remainder1,

            number_of_nullifiers,

            already_checked_nullifiers,
            nullifier,
            unused_remainder3,
        ) = array_refs![input, 1, 3936, 65536, 8, 37952, 18976, 8648];
        assert_eq!(is_initialized[0], 1);

        Ok(
            MerkleTreeNullifierBytes_2 {
                is_initialized: true,
                insert_nullifier: false,
                nullifer_to_be_inserted: vec![0u8;32],
                nullifier: nullifier.to_vec(),
                number_of_nullifiers:  u64::from_le_bytes(*number_of_nullifiers),
                nullifier_index_start: 0,
                nullifier_index_end: 0,

            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeNullifierBytes_2::LEN];

        let (
            is_initialized_dst,
            unused_remainder0_dst,

            unused_remainder1_dst,

            number_of_nullifiers_dst,

            already_checked_nullifiers_dst,
            nullifier_dst,
            unused_remainder3_dst,
        ) = mut_array_refs![dst, 1, 3936, 65536, 8, 37952, 18976, 8648];
        if self.insert_nullifier == true {
            //msg!("packed nullifier_index_start {}",self.nullifier_index_start );
            //msg!("packed inserted_nullifier {}", self.nullifier_index_end);
            for i in 0..18944 {
                if i >= self.nullifier_index_start && i < self.nullifier_index_end {
                    nullifier_dst[i] = self.nullifer_to_be_inserted[i%32].clone();
                    //msg!("index {}", i);
                } else {
                    nullifier_dst[i] = nullifier_dst[i];
                }
            }
        } else {
            *nullifier_dst = *nullifier_dst;
        }


        *unused_remainder0_dst = *unused_remainder0_dst;
        *unused_remainder1_dst = *unused_remainder1_dst;

        *already_checked_nullifiers_dst = *already_checked_nullifiers_dst;
        *unused_remainder3_dst = *unused_remainder3_dst;
        *is_initialized_dst = *is_initialized_dst;

        msg!("packed inserted_nullifier 2");
    }
}

#[derive(Clone, Debug)]
pub struct MerkleTreeNullifierBytes_3 {
    pub is_initialized: bool,
    pub insert_nullifier: bool,
    pub nullifer_to_be_inserted: Vec<u8>,
    pub nullifier: Vec<u8>,
    pub number_of_nullifiers: u64,
    pub nullifier_index_start: usize,
    pub nullifier_index_end: usize,
}

impl Sealed for MerkleTreeNullifierBytes_3 {}
impl IsInitialized for MerkleTreeNullifierBytes_3 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeNullifierBytes_3 {
    const LEN: usize = 135057;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MerkleTreeNullifierBytes_3::LEN];

        let (
            is_initialized,
            unused_remainder0,

            unused_remainder1,

            number_of_nullifiers,

            already_checked_nullifiers,
            nullifier,
            unused_remainder,
        ) = array_refs![input,1, 3936, 65536, 8, 56928, 8608, 40];
        assert_eq!(is_initialized[0], 1);

        Ok(
            MerkleTreeNullifierBytes_3 {
                is_initialized: true,
                insert_nullifier: false,
                nullifer_to_be_inserted: vec![0u8;32],
                nullifier: nullifier.to_vec(),
                number_of_nullifiers:  u64::from_le_bytes(*number_of_nullifiers),
                nullifier_index_start: 0,
                nullifier_index_end: 0,

            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeNullifierBytes_3::LEN];

        let (
            is_initialized_dst,
            unused_remainder0_dst,

            unused_remainder1_dst,

            number_of_nullifiers_dst,

            already_checked_nullifiers_dst,
            nullifier_dst,
            unused_remainder_dst,
        ) = mut_array_refs![dst, 1, 3936, 65536, 8, 56928, 8608, 40];


        if self.insert_nullifier == true {
            //msg!("packed nullifier_index_start {}",self.nullifier_index_start );
            //msg!("packed inserted_nullifier {}", self.nullifier_index_end);
            for i in 0..8608 {
                if i >= self.nullifier_index_start && i < self.nullifier_index_end {
                    nullifier_dst[i] = self.nullifer_to_be_inserted[i%32].clone();
                    //msg!("index {}", i);
                } else {
                    nullifier_dst[i] = nullifier_dst[i];
                }
            }
        } else {
            *nullifier_dst = *nullifier_dst;
        }


        *unused_remainder0_dst = *unused_remainder0_dst;
        *unused_remainder1_dst = *unused_remainder1_dst;
        *unused_remainder_dst = *unused_remainder_dst;
        *already_checked_nullifiers_dst = *already_checked_nullifiers_dst;
        *is_initialized_dst = *is_initialized_dst;
        msg!("packed inserted_nullifier 3");
    }
}

const hash_wrong: [u8;32] = [186, 11, 250, 107, 131, 86, 119, 78, 239, 31, 50, 120, 132, 189, 175, 67, 30, 6, 80, 159, 190, 145, 23, 2, 253, 30, 141, 111, 155, 114, 43, 46];
const hash_right: [u8;32] = [31, 144, 21, 151, 128, 237, 76, 1, 73, 117, 131, 239, 81, 189, 153, 59, 25, 174, 65, 141, 117, 247, 123, 35, 102, 61, 246, 100, 161, 94, 165, 96];
