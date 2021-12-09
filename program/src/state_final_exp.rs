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


// Account struct for verify Part 2:
#[derive(Debug, Clone)]
pub struct FinalExpBytes {
    is_initialized: bool,
    pub found_nullifier: u8,
    pub signing_address: Vec<u8>,
    pub relayer_refund: Vec<u8>,
    pub to_address: Vec<u8>,
    pub amount: Vec<u8>,
    pub nullifer: Vec<u8>,
    pub f1_r_range_s: Vec<u8>,
    pub f_f2_range_s: Vec<u8>,
    pub i_range_s: Vec<u8>,

    pub y0_range_s: Vec<u8>,
    pub y1_range_s: Vec<u8>,
    pub y2_range_s: Vec<u8>,
    pub y6_range: Vec<u8>,

    pub cubic_range_0_s: Vec<u8>,
    pub cubic_range_1_s: Vec<u8>,
    pub cubic_range_2_s: Vec<u8>,

    pub quad_range_0_s: Vec<u8>,
    pub quad_range_1_s: Vec<u8>,
    pub quad_range_2_s: Vec<u8>,
    pub quad_range_3_s: Vec<u8>,

    pub fp384_range_s: Vec<u8>,

    pub current_instruction_index: usize,

    pub changed_variables: [bool;16],
}
impl Sealed for FinalExpBytes {}
impl IsInitialized for FinalExpBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl FinalExpBytes {
    pub fn new () -> FinalExpBytes {
        FinalExpBytes {
            is_initialized: true,
            found_nullifier: 0,
            signing_address: vec![0],
            relayer_refund: vec![0],
            to_address: vec![0],

            amount: vec![0],
            nullifer: vec![0],
            f1_r_range_s: vec![0;384],
            f_f2_range_s: vec![0;384],
            i_range_s: vec![0;384],

            y0_range_s: vec![0;384],
            y1_range_s: vec![0;384],
            y2_range_s: vec![0;384],
            y6_range: vec![0;384],

            cubic_range_0_s: vec![0;192],
            cubic_range_1_s: vec![0;192],
            cubic_range_2_s: vec![0;192],


            quad_range_0_s: vec![0;64],
            quad_range_1_s: vec![0;64],
            quad_range_2_s: vec![0;64],
            quad_range_3_s: vec![0;64],

            fp384_range_s: vec![0;32],
            current_instruction_index: 0,
            changed_variables: [false;16],
        }
    }
}

impl Pack for FinalExpBytes {
    const LEN: usize = 3772;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, FinalExpBytes::LEN];

        let (
            is_initialized,
            found_root,
            found_nullifier,
            unused_constants0,
            signing_address,
            relayer_refund,
            to_address,
            amount,
            nullifer,
            unused_constants2,
            current_instruction_index,

            f_f2_range_s,
            f1_r_range_s,
            i_range_s,

            y0_range_s,
            //1756
            y1_range_s,
            //2140
            y2_range_s,

            cubic_range_0_s,
            cubic_range_1_s,
            cubic_range_2_s,

            quad_range_0_s,
            quad_range_1_s,
            quad_range_2_s,
            quad_range_3_s,

            fp384_range_s,
            y6_range,

        ) = array_refs![input,1, 1, 1, 1, 32, 8, 32, 8, 32, 96, 8, 384, 384, 384, 384, 384, 384, 192, 192, 192, 64, 64, 64, 64, 32, 384];

        Ok(
            FinalExpBytes {
                is_initialized: true,
                found_nullifier: found_nullifier[0],
                signing_address: signing_address.to_vec(),
                relayer_refund: relayer_refund.to_vec(),
                to_address: to_address.to_vec(),

                amount: amount.to_vec(),
                nullifer: nullifer.to_vec(),
                f_f2_range_s: f_f2_range_s.to_vec(),
                f1_r_range_s: f1_r_range_s.to_vec(),
                i_range_s: i_range_s.to_vec(),
                y0_range_s: y0_range_s.to_vec(),
                y1_range_s: y1_range_s.to_vec(),
                y2_range_s: y2_range_s.to_vec(),

                cubic_range_0_s: cubic_range_0_s.to_vec(),
                cubic_range_1_s: cubic_range_1_s.to_vec(),
                cubic_range_2_s: cubic_range_2_s.to_vec(),

                quad_range_0_s: quad_range_0_s.to_vec(),
                quad_range_1_s: quad_range_1_s.to_vec(),
                quad_range_2_s: quad_range_2_s.to_vec(),
                quad_range_3_s: quad_range_3_s.to_vec(),

                fp384_range_s: fp384_range_s.to_vec(),
                y6_range: y6_range.to_vec(),

                current_instruction_index: usize::from_le_bytes(*current_instruction_index),
                changed_variables: [false;16],
            }
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0,  FinalExpBytes::LEN];

        let (
            is_initialized_dst,
            found_root_dst,
            found_nullifier_dst,
            unused_constants_dst,
            current_instruction_index_dst,

            f_f2_range_dst,
            f1_r_range_dst,
            i_range_dst,

            y0_range_dst,
            y1_range_dst,
            y2_range_dst,

            cubic_range_0_dst,
            cubic_range_1_dst,
            cubic_range_2_dst,

            quad_range_0_dst,
            quad_range_1_dst,
            quad_range_2_dst,
            quad_range_3_dst,

            fp384_range_dst,
            y6_range_dst


        ) = mut_array_refs![dst, 1, 1, 1, 209, 8, 384, 384, 384, 384, 384, 384, 192, 192, 192, 64, 64, 64, 64, 32, 384];
        println!("modifying: {:?}", self.changed_variables);
        for (i, variable_has_changed) in self.changed_variables.iter().enumerate() {
            if *variable_has_changed {
                if i == 0  {
                    *f_f2_range_dst = self.f_f2_range_s.clone().try_into().unwrap();
                    msg!("modifying: f_f2_range_dst" );
                } else if i == 1 {
                    *f1_r_range_dst = self.f1_r_range_s.clone().try_into().unwrap();
                    msg!("modifying: f1_r_range_dst" );
                }  else if i == 2 {
                    *i_range_dst = self.i_range_s.clone().try_into().unwrap();
                }  else if i == 3 {
                    *y0_range_dst = self.y0_range_s.clone().try_into().unwrap();
                }  else if i == 4 {
                    *y1_range_dst = self.y1_range_s.clone().try_into().unwrap();
                }  else if i == 5 {
                    *y2_range_dst = self.y2_range_s.clone().try_into().unwrap();
                }  else if i == 6 {
                    *cubic_range_0_dst = self.cubic_range_0_s.clone().try_into().unwrap();
                }  else if i == 7 {
                    *cubic_range_1_dst = self.cubic_range_1_s.clone().try_into().unwrap();
                }  else if i == 8 {
                    *cubic_range_2_dst = self.cubic_range_2_s.clone().try_into().unwrap();
                }  else if i == 9 {
                    *quad_range_0_dst = self.quad_range_0_s.clone().try_into().unwrap();
                }   else if i == 10 {
                    *quad_range_1_dst = self.quad_range_1_s.clone().try_into().unwrap();
                }   else if i == 11 {
                    *quad_range_2_dst = self.quad_range_2_s.clone().try_into().unwrap();
                }   else if i == 12 {
                    *quad_range_3_dst = self.quad_range_3_s.clone().try_into().unwrap();
                }   else if i == 13 {
                    *fp384_range_dst = self.fp384_range_s.clone().try_into().unwrap();
                }   else if i == 14 {
                    *found_nullifier_dst = [self.found_nullifier; 1];
                    msg!("modifying: found_nullifier_dst" );
                }   else if i == 15 {
                    *y6_range_dst = self.y6_range.clone().try_into().unwrap();
                    msg!("modifying: found_nullifier_dst" );
                }
            }
            else {
                if i == 0  {
                    *f_f2_range_dst = *f_f2_range_dst;
                } else if i == 1 {
                    *f1_r_range_dst = *f1_r_range_dst;
                }  else if i == 2 {
                    *i_range_dst = *i_range_dst;
                }  else if i == 3 {
                    *y0_range_dst = *y0_range_dst;
                }  else if i == 4 {
                    *y1_range_dst = *y1_range_dst;
                }  else if i == 5 {
                    *y2_range_dst = *y2_range_dst;
                }  else if i == 6 {
                    *cubic_range_0_dst = *cubic_range_0_dst;
                }  else if i == 7 {
                    *cubic_range_1_dst = *cubic_range_1_dst;
                }  else if i == 8 {
                    *cubic_range_2_dst = *cubic_range_2_dst;
                }  else if i == 9 {
                    *quad_range_0_dst = *quad_range_0_dst;
                }   else if i == 10 {
                    *quad_range_1_dst = *quad_range_1_dst;
                }   else if i == 11 {
                    *quad_range_2_dst = *quad_range_2_dst;
                }   else if i == 12 {
                    *quad_range_3_dst = *quad_range_3_dst;
                }   else if i == 13 {
                    *fp384_range_dst = *fp384_range_dst;
                }   else if i == 14 {
                    *found_nullifier_dst = *found_nullifier_dst;
                }
            }

        }
        *found_root_dst = *found_root_dst;
        *unused_constants_dst = *unused_constants_dst;
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *is_initialized_dst = [1u8; 1];

    }
}


// //current_highest 120
// pub const INSTRUCTION_ORDER_VERIFIER_PART_2 : [u8; 1533] = [
//   0, 1, 2, 3, 4, 5, 120, 6, 7, 101, 102, 8, 9, 10, 11, 104, 12, 13, 14, 15, 8, 9, 10, 11, 104, 12, 16, 17, 18, 19, 20, 105, 21, 22, 20, 105, 21, 22, 28, 29, 30, 31, 107, 32, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 23, 24, 25, 26, 106, 27, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 20, 105, 21, 22, 33, 34, 35, 36, 37, 38, 39, 108, 40, 41, 18, 42, 43, 109, 44, 45, 43, 109, 44, 45, 51, 52, 53, 54, 111, 55, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 56, 57, 36, 37, 38, 39, 108, 40, 41, 18, 42, 43, 109, 44, 45, 43, 109, 44, 45, 51, 52, 53, 54, 111, 55, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 46, 47, 48, 49, 110, 50, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 56, 58, 59, 36, 37, 38, 39, 108, 40, 61, 62, 63, 64, 112, 65, 41, 18, 66, 67, 113, 68, 69, 67, 113, 68, 69, 75, 76, 77, 78, 115, 79, 67, 113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 70, 71, 72, 73, 114, 74, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 67, 113, 68, 69, 80, 81, 18, 82, 43, 109, 44, 45, 43, 109, 44, 45, 51, 52, 53, 54, 111, 55, 43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 83, 84, 85, 86, 116, 87, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 43, 109, 44, 45, 56, 88, 89, 90, 57, 36, 37, 38, 39, 108, 40, 91, 92, 93, 94, 117, 95, 96, 97, 98, 99, 118, 100, 121, 122, 123, 124, 103];
pub const INSTRUCTION_ORDER_VERIFIER_PART_2: [u8; 700] = [0,1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 10, 11, 12, 13, 14, 15, 19, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 55, 56, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 71, 72, 73, 74, 75, 76, 112, 113, 114, 115, 116, 117, 118, 119, 120, 83, 84, 85, 86, 87, 88];
