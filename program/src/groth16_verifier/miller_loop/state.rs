use crate::utils::config::TMP_STORAGE_ACCOUNT_TYPE;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryInto;

// Implements partial pack to save compute budget.
#[derive(Clone)]
pub struct MillerLoopState {
    pub is_initialized: bool,
    pub signing_address: Vec<u8>, // is relayer address
    pub current_instruction_index: usize,
    // common ranges
    pub f_range: Vec<u8>,
    pub coeff_2_range: Vec<u8>,
    pub coeff_1_range: Vec<u8>,
    pub coeff_0_range: Vec<u8>,
    pub p_1_x_range: Vec<u8>,
    pub p_1_y_range: Vec<u8>,
    pub p_2_x_range: Vec<u8>,
    pub p_2_y_range: Vec<u8>,
    pub p_3_x_range: Vec<u8>,
    pub p_3_y_range: Vec<u8>,
    pub r: Vec<u8>,
    pub proof_b: Vec<u8>,
    pub current_coeff_2_range: Vec<u8>,
    pub current_coeff_3_range: Vec<u8>,
    pub changed_variables: [bool; 14],
}
impl Sealed for MillerLoopState {}
impl IsInitialized for MillerLoopState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MillerLoopState {
    const LEN: usize = 3900 + 384; // 1728;

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, MillerLoopState::LEN];

        let (
            _is_initialized,
            account_type,
            _unused_constants0,
            signing_address,
            _unused_constants1,
            current_instruction_index,
            f_range,
            coeff_2_range,
            coeff_1_range,
            coeff_0_range,
            p_1_x_range, //32
            p_1_y_range, //32
            p_2_x_range,
            p_2_y_range,
            p_3_x_range,
            p_3_y_range,
            r,
            proof_b, //128
            current_coeff_2_range,
            current_coeff_3_range,
            _unused_remainder,
        ) = array_refs![
            input, 1, 1, 2, 32, 176, 8, 384, 64, 64, 64, 32, 32, 32, 32, 32, 32, 192, 128, 1, 1,
            2590 + 384
        ];
        if account_type[0] != TMP_STORAGE_ACCOUNT_TYPE {
            msg!("Wrong account type.");
            return Err(ProgramError::InvalidArgument);
        }
        Ok(MillerLoopState {
            is_initialized: true,
            signing_address: signing_address.to_vec(),
            current_instruction_index: usize::from_le_bytes(*current_instruction_index),

            f_range: f_range.to_vec(),
            coeff_2_range: coeff_2_range.to_vec(),
            coeff_1_range: coeff_1_range.to_vec(),
            coeff_0_range: coeff_0_range.to_vec(),

            p_1_x_range: p_1_x_range.to_vec(),
            p_1_y_range: p_1_y_range.to_vec(),
            p_2_x_range: p_2_x_range.to_vec(),
            p_2_y_range: p_2_y_range.to_vec(),
            p_3_x_range: p_3_x_range.to_vec(),
            p_3_y_range: p_3_y_range.to_vec(),

            r: r.to_vec(),
            proof_b: proof_b.to_vec(),
            current_coeff_2_range: current_coeff_2_range.to_vec(),
            current_coeff_3_range: current_coeff_3_range.to_vec(),
            changed_variables: [false; 14],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MillerLoopState::LEN];

        let (
            _is_initialized_dst,
            _unused_constants0_dst,
            _signing_address_dst,
            _unused_constants1_dst,
            current_instruction_index_dst,
            f_range_dst,
            coeff_2_range_dst,
            coeff_1_range_dst,
            coeff_0_range_dst,
            p_1_x_range_dst,
            p_1_y_range_dst,
            p_2_x_range_dst,
            p_2_y_range_dst,
            p_3_x_range_dst,
            p_3_y_range_dst,
            r_dst,
            proof_b_dst,
            current_coeff_2_range_dst,
            current_coeff_3_range_dst,
            _unused_remainder,
        ) = mut_array_refs![
            dst, 1, 3, 32, 176, 8, 384, 64, 64, 64, 32, 32, 32, 32, 32, 32, 192, 128, 1, 1, 2590 + 384
        ];

        for (i, var_has_changed) in self.changed_variables.iter().enumerate() {
            if *var_has_changed {
                if i == 0 {
                    *f_range_dst = self.f_range.clone().try_into().unwrap();
                } else if i == 1 {
                    *coeff_2_range_dst = self.coeff_2_range.clone().try_into().unwrap();
                } else if i == 2 {
                    *coeff_1_range_dst = self.coeff_1_range.clone().try_into().unwrap();
                } else if i == 3 {
                    *coeff_0_range_dst = self.coeff_0_range.clone().try_into().unwrap();
                } else if i == 4 {
                    *p_1_x_range_dst = self.p_1_x_range.clone().try_into().unwrap();
                } else if i == 5 {
                    *p_1_y_range_dst = self.p_1_y_range.clone().try_into().unwrap();
                } else if i == 6 {
                    *p_2_x_range_dst = self.p_2_x_range.clone().try_into().unwrap();
                } else if i == 7 {
                    *p_2_y_range_dst = self.p_2_y_range.clone().try_into().unwrap();
                } else if i == 8 {
                    *p_3_x_range_dst = self.p_3_x_range.clone().try_into().unwrap();
                } else if i == 9 {
                    *p_3_y_range_dst = self.p_3_y_range.clone().try_into().unwrap();
                } else if i == 10 {
                    *r_dst = self.r.clone().try_into().unwrap();
                } else if i == 11 {
                    *proof_b_dst = self.proof_b.clone().try_into().unwrap();
                } else if i == 12 {
                    *current_coeff_2_range_dst =
                        self.current_coeff_2_range.clone().try_into().unwrap();
                } else if i == 13 {
                    *current_coeff_3_range_dst =
                        self.current_coeff_3_range.clone().try_into().unwrap();
                }
            }
        }
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
    }
}
