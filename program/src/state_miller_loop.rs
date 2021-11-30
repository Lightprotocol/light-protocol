use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use solana_program::{
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

#[derive(Clone)]
pub struct MillerLoopBytes {
    pub is_initialized: bool,
    pub signing_address: Vec<u8>, // is relayer address
    pub current_instruction_index: usize,

    // common ranges
    pub f_range: Vec<u8>,
    pub c0_copy_range: Vec<u8>,
    pub cubic_v0_range: Vec<u8>,
    pub cubic_v2_range: Vec<u8>,
    pub cubic_v3_range: Vec<u8>,
    pub coeff_2_range: Vec<u8>,
    pub coeff_1_range: Vec<u8>,
    pub aa_range: Vec<u8>,
    pub coeff_0_range: Vec<u8>,
    pub bb_range: Vec<u8>,
    pub p_1_x_range: Vec<u8>,
    pub p_1_y_range: Vec<u8>,
    pub p_2_x_range: Vec<u8>,
    pub p_2_y_range: Vec<u8>,
    pub p_3_x_range: Vec<u8>,
    pub p_3_y_range: Vec<u8>,
    // ELL p1,2,3 ranges
    pub h: Vec<u8>,
    pub g: Vec<u8>,
    pub e: Vec<u8>,
    pub lambda: Vec<u8>,
    pub theta: Vec<u8>,
    pub r: Vec<u8>,
    pub proof_b: Vec<u8>,
    pub current_coeff_2_range: Vec<u8>,
    pub current_coeff_3_range: Vec<u8>,

    pub changed_variables: [bool; 25],
}
impl Sealed for MillerLoopBytes {}
impl IsInitialized for MillerLoopBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MillerLoopBytes {
    const LEN: usize = 4972; // 1728; // optimize by 1.1k bytes

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, MillerLoopBytes::LEN];

        let (
            is_initialized,
            unused_constants0,
            signing_address,
            unused_constants1,
            current_instruction_index,
            f_range,
            c0_copy_range,
            cubic_v0_range,
            cubic_v2_range,
            cubic_v3_range,
            coeff_2_range,
            coeff_1_range,
            aa_range,
            coeff_0_range,
            bb_range,
            p_1_x_range,
            p_1_y_range,
            p_2_x_range,
            p_2_y_range,
            p_3_x_range,
            p_3_y_range,
            //additions ell_coeffs
            h,
            g,
            e,
            lambda,
            theta,
            r,
            proof_b,
            current_coeff_2_range,
            current_coeff_3_range,
            unused_remainder,
        ) = array_refs![
            input, 1, 3, 32, 176, 8, 576, 288, 288, 288, 288, 96, 96, 288, 96, 288, 48, 48, 48, 48,
            48, 48, 96, 96, 96, 96, 96, 288, 192, 1, 1, 910
        ];
        Ok(
            //216 - 32 - 8
            MillerLoopBytes {
                is_initialized: true,
                signing_address: signing_address.to_vec(),
                current_instruction_index: usize::from_le_bytes(*current_instruction_index),

                f_range: f_range.to_vec(),
                c0_copy_range: c0_copy_range.to_vec(),

                cubic_v0_range: cubic_v0_range.to_vec(),
                cubic_v2_range: cubic_v2_range.to_vec(),
                cubic_v3_range: cubic_v3_range.to_vec(),
                coeff_2_range: coeff_2_range.to_vec(),
                coeff_1_range: coeff_1_range.to_vec(),
                aa_range: aa_range.to_vec(),
                coeff_0_range: coeff_0_range.to_vec(),
                bb_range: bb_range.to_vec(),

                p_1_x_range: p_1_x_range.to_vec(),
                p_1_y_range: p_1_y_range.to_vec(),
                p_2_x_range: p_2_x_range.to_vec(),
                p_2_y_range: p_2_y_range.to_vec(),
                p_3_x_range: p_3_x_range.to_vec(),
                p_3_y_range: p_3_y_range.to_vec(),

                //additions ell_coeffs
                h: h.to_vec(),
                g: g.to_vec(),
                e: e.to_vec(),
                lambda: lambda.to_vec(),
                theta: theta.to_vec(),
                r: r.to_vec(),
                proof_b: proof_b.to_vec(),
                current_coeff_2_range: current_coeff_2_range.to_vec(),
                current_coeff_3_range: current_coeff_3_range.to_vec(),
                changed_variables: [false; 25],
            },
        )
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MillerLoopBytes::LEN];

        let (
            is_initialized_dst,
            unused_constants0_dst,
            signing_address_dst,
            unused_constants1_dst,
            current_instruction_index_dst,
            f_range_dst,
            c0_copy_range_dst,
            cubic_v0_range_dst,
            cubic_v2_range_dst,
            cubic_v3_range_dst,
            coeff_2_range_dst,
            coeff_1_range_dst,
            aa_range_dst,
            coeff_0_range_dst,
            bb_range_dst,
            p_1_x_range_dst,
            p_1_y_range_dst,
            p_2_x_range_dst,
            p_2_y_range_dst,
            p_3_x_range_dst,
            p_3_y_range_dst,
            //additions ell_coeffs
            h_dst,
            g_dst,
            e_dst,
            lambda_dst,
            theta_dst,
            r_dst,
            proof_b_dst,
            current_coeff_2_range_dst,
            current_coeff_3_range_dst,
            unused_remainder,
        ) = mut_array_refs![
            dst, 1, 3, 32, 176, 8, 576, 288, 288, 288, 288, 96, 96, 288, 96, 288, 48, 48, 48, 48,
            48, 48, 96, 96, 96, 96, 96, 288, 192, 1, 1, 910
        ];

        for (i, var_has_changed) in self.changed_variables.iter().enumerate() {
            if *var_has_changed {
                if i == 0 {
                    *f_range_dst = self.f_range.clone().try_into().unwrap();
                } else if i == 1 {
                    *c0_copy_range_dst = self.c0_copy_range.clone().try_into().unwrap();
                } else if i == 2 {
                    *cubic_v0_range_dst = self.cubic_v0_range.clone().try_into().unwrap();
                } else if i == 3 {
                    *cubic_v2_range_dst = self.cubic_v2_range.clone().try_into().unwrap();
                } else if i == 4 {
                    *cubic_v3_range_dst = self.cubic_v3_range.clone().try_into().unwrap();
                } else if i == 5 {
                    *coeff_2_range_dst = self.coeff_2_range.clone().try_into().unwrap();
                } else if i == 6 {
                    *coeff_1_range_dst = self.coeff_1_range.clone().try_into().unwrap();
                } else if i == 7 {
                    *aa_range_dst = self.aa_range.clone().try_into().unwrap();
                } else if i == 8 {
                    *coeff_0_range_dst = self.coeff_0_range.clone().try_into().unwrap();
                } else if i == 9 {
                    *bb_range_dst = self.bb_range.clone().try_into().unwrap();
                } else if i == 10 {
                    *p_1_x_range_dst = self.p_1_x_range.clone().try_into().unwrap();
                } else if i == 11 {
                    *p_1_y_range_dst = self.p_1_y_range.clone().try_into().unwrap();
                } else if i == 12 {
                    *p_2_x_range_dst = self.p_2_x_range.clone().try_into().unwrap();
                } else if i == 13 {
                    *p_2_y_range_dst = self.p_2_y_range.clone().try_into().unwrap();
                } else if i == 14 {
                    *p_3_x_range_dst = self.p_3_x_range.clone().try_into().unwrap();
                } else if i == 15 {
                    *p_3_y_range_dst = self.p_3_y_range.clone().try_into().unwrap();
                //additions ell_coeffs
                } else if i == 16 {
                    *h_dst = self.h.clone().try_into().unwrap();
                } else if i == 17 {
                    *g_dst = self.g.clone().try_into().unwrap();
                } else if i == 18 {
                    *e_dst = self.e.clone().try_into().unwrap();
                } else if i == 19 {
                    *lambda_dst = self.lambda.clone().try_into().unwrap();
                } else if i == 20 {
                    *theta_dst = self.theta.clone().try_into().unwrap();
                } else if i == 21 {
                    *r_dst = self.r.clone().try_into().unwrap();
                } else if i == 22 {
                    *proof_b_dst = self.proof_b.clone().try_into().unwrap();
                } else if i == 23 {
                    *current_coeff_2_range_dst =
                        self.current_coeff_2_range.clone().try_into().unwrap();
                } else if i == 24 {
                    *current_coeff_3_range_dst =
                        self.current_coeff_3_range.clone().try_into().unwrap();
                }
            } else {
                if i == 0 {
                    *f_range_dst = *f_range_dst;
                } else if i == 1 {
                    *c0_copy_range_dst = *c0_copy_range_dst;
                } else if i == 2 {
                    *cubic_v0_range_dst = *cubic_v0_range_dst;
                } else if i == 3 {
                    *cubic_v2_range_dst = *cubic_v2_range_dst;
                } else if i == 4 {
                    *cubic_v3_range_dst = *cubic_v3_range_dst;
                } else if i == 5 {
                    *coeff_2_range_dst = *coeff_2_range_dst;
                } else if i == 6 {
                    *coeff_1_range_dst = *coeff_1_range_dst;
                } else if i == 7 {
                    *aa_range_dst = *aa_range_dst;
                } else if i == 8 {
                    *coeff_0_range_dst = *coeff_0_range_dst;
                } else if i == 9 {
                    *bb_range_dst = *bb_range_dst;
                } else if i == 10 {
                    *p_1_x_range_dst = *p_1_x_range_dst;
                } else if i == 11 {
                    *p_1_y_range_dst = *p_1_y_range_dst;
                } else if i == 12 {
                    *p_2_x_range_dst = *p_2_x_range_dst;
                } else if i == 13 {
                    *p_2_y_range_dst = *p_2_y_range_dst;
                } else if i == 14 {
                    *p_3_x_range_dst = *p_3_x_range_dst;
                } else if i == 15 {
                    *p_3_y_range_dst = *p_3_y_range_dst;
                //additions ell_coeffs
                } else if i == 16 {
                    *h_dst = *h_dst;
                } else if i == 17 {
                    *g_dst = *g_dst;
                } else if i == 18 {
                    *e_dst = *e_dst;
                } else if i == 19 {
                    *lambda_dst = *lambda_dst;
                } else if i == 20 {
                    *theta_dst = *theta_dst;
                } else if i == 21 {
                    *r_dst = *r_dst;
                } else if i == 22 {
                    *proof_b_dst = *proof_b_dst;
                } else if i == 23 {
                    *current_coeff_2_range_dst = *current_coeff_2_range_dst;
                } else if i == 24 {
                    *current_coeff_3_range_dst = *current_coeff_3_range_dst;
                }
            };
        }
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *unused_constants0_dst = *unused_constants0_dst;
        *signing_address_dst = *signing_address_dst;
        *unused_constants1_dst = *unused_constants1_dst;
        *is_initialized_dst = [1u8; 1];
    }
}

pub const complete_instruction_order_verify_one: [u8; 1821] = [
    251, 230, 237, 3, 17, 4, 5, 231, 232, 233, 20, 7, 8, 9, 18, 10, 225, 21, 7, 8, 9, 18, 10, 226,
    22, 7, 8, 9, 18, 10, 234, 235, 236, 23, 7, 8, 9, 18, 10, 225, 24, 7, 8, 9, 18, 10, 226, 25, 7,
    8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 26, 7, 8, 9, 18, 10, 225, 27, 7, 8, 9, 18, 10, 226,
    28, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 29, 7, 8, 9, 18, 10, 225, 30, 7, 8, 9, 18, 10,
    226, 31, 7, 8, 9, 18, 10, 234, 235, 236, 32, 7, 8, 9, 18, 10, 225, 33, 7, 8, 9, 18, 10, 226,
    34, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 35, 7, 8, 9, 18, 10, 225, 36, 7, 8, 9, 18, 10,
    226, 37, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 38, 7, 8, 9, 18, 10, 225, 39, 7, 8, 9,
    18, 10, 226, 40, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 41, 7, 8, 9, 18, 10, 225, 42, 7,
    8, 9, 18, 10, 226, 43, 7, 8, 9, 18, 10, 234, 235, 236, 44, 7, 8, 9, 18, 10, 225, 45, 7, 8, 9,
    18, 10, 226, 46, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 47, 7, 8, 9, 18, 10, 225, 48, 7,
    8, 9, 18, 10, 226, 49, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 50, 7, 8, 9, 18, 10, 225,
    51, 7, 8, 9, 18, 10, 226, 52, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 53, 7, 8, 9, 18, 10,
    225, 54, 7, 8, 9, 18, 10, 226, 55, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 56, 7, 8, 9,
    18, 10, 225, 57, 7, 8, 9, 18, 10, 226, 58, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 59, 7,
    8, 9, 18, 10, 225, 60, 7, 8, 9, 18, 10, 226, 61, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233,
    62, 7, 8, 9, 18, 10, 225, 63, 7, 8, 9, 18, 10, 226, 64, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232,
    233, 65, 7, 8, 9, 18, 10, 225, 66, 7, 8, 9, 18, 10, 226, 67, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231,
    232, 233, 68, 7, 8, 9, 18, 10, 225, 69, 7, 8, 9, 18, 10, 226, 70, 7, 8, 9, 18, 10, 3, 17, 4, 5,
    231, 232, 233, 71, 7, 8, 9, 18, 10, 225, 72, 7, 8, 9, 18, 10, 226, 73, 7, 8, 9, 18, 10, 234,
    235, 236, 74, 7, 8, 9, 18, 10, 225, 75, 7, 8, 9, 18, 10, 226, 76, 7, 8, 9, 18, 10, 3, 17, 4, 5,
    231, 232, 233, 77, 7, 8, 9, 18, 10, 225, 78, 7, 8, 9, 18, 10, 226, 79, 7, 8, 9, 18, 10, 3, 17,
    4, 5, 231, 232, 233, 80, 7, 8, 9, 18, 10, 225, 81, 7, 8, 9, 18, 10, 226, 82, 7, 8, 9, 18, 10,
    3, 17, 4, 5, 231, 232, 233, 83, 7, 8, 9, 18, 10, 225, 84, 7, 8, 9, 18, 10, 226, 85, 7, 8, 9,
    18, 10, 3, 17, 4, 5, 231, 232, 233, 86, 7, 8, 9, 18, 10, 225, 87, 7, 8, 9, 18, 10, 226, 88, 7,
    8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 89, 7, 8, 9, 18, 10, 225, 90, 7, 8, 9, 18, 10, 226,
    91, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 92, 7, 8, 9, 18, 10, 225, 93, 7, 8, 9, 18, 10,
    226, 94, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 95, 7, 8, 9, 18, 10, 225, 96, 7, 8, 9,
    18, 10, 226, 97, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 98, 7, 8, 9, 18, 10, 225, 99, 7,
    8, 9, 18, 10, 226, 100, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 101, 7, 8, 9, 18, 10, 225,
    102, 7, 8, 9, 18, 10, 226, 103, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 104, 7, 8, 9, 18,
    10, 225, 105, 7, 8, 9, 18, 10, 226, 106, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 107, 7,
    8, 9, 18, 10, 225, 108, 7, 8, 9, 18, 10, 226, 109, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233,
    110, 7, 8, 9, 18, 10, 225, 111, 7, 8, 9, 18, 10, 226, 112, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231,
    232, 233, 113, 7, 8, 9, 18, 10, 225, 114, 7, 8, 9, 18, 10, 226, 115, 7, 8, 9, 18, 10, 3, 17, 4,
    5, 231, 232, 233, 116, 7, 8, 9, 18, 10, 225, 117, 7, 8, 9, 18, 10, 226, 118, 7, 8, 9, 18, 10,
    3, 17, 4, 5, 231, 232, 233, 119, 7, 8, 9, 18, 10, 225, 120, 7, 8, 9, 18, 10, 226, 121, 7, 8, 9,
    18, 10, 3, 17, 4, 5, 231, 232, 233, 122, 7, 8, 9, 18, 10, 225, 123, 7, 8, 9, 18, 10, 226, 124,
    7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 125, 7, 8, 9, 18, 10, 225, 126, 7, 8, 9, 18, 10,
    226, 127, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 128, 7, 8, 9, 18, 10, 225, 129, 7, 8, 9,
    18, 10, 226, 130, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 131, 7, 8, 9, 18, 10, 225, 132,
    7, 8, 9, 18, 10, 226, 133, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 134, 7, 8, 9, 18, 10,
    225, 135, 7, 8, 9, 18, 10, 226, 136, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 137, 7, 8, 9,
    18, 10, 225, 138, 7, 8, 9, 18, 10, 226, 139, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 140,
    7, 8, 9, 18, 10, 225, 141, 7, 8, 9, 18, 10, 226, 142, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232,
    233, 143, 7, 8, 9, 18, 10, 225, 144, 7, 8, 9, 18, 10, 226, 145, 7, 8, 9, 18, 10, 3, 17, 4, 5,
    231, 232, 233, 146, 7, 8, 9, 18, 10, 225, 147, 7, 8, 9, 18, 10, 226, 148, 7, 8, 9, 18, 10, 3,
    17, 4, 5, 231, 232, 233, 149, 7, 8, 9, 18, 10, 225, 150, 7, 8, 9, 18, 10, 226, 151, 7, 8, 9,
    18, 10, 3, 17, 4, 5, 231, 232, 233, 152, 7, 8, 9, 18, 10, 225, 153, 7, 8, 9, 18, 10, 226, 154,
    7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 155, 7, 8, 9, 18, 10, 225, 156, 7, 8, 9, 18, 10,
    226, 157, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 158, 7, 8, 9, 18, 10, 225, 159, 7, 8, 9,
    18, 10, 226, 160, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 161, 7, 8, 9, 18, 10, 225, 162,
    7, 8, 9, 18, 10, 226, 163, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 164, 7, 8, 9, 18, 10,
    225, 165, 7, 8, 9, 18, 10, 226, 166, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 167, 7, 8, 9,
    18, 10, 225, 168, 7, 8, 9, 18, 10, 226, 169, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 170,
    7, 8, 9, 18, 10, 225, 171, 7, 8, 9, 18, 10, 226, 172, 7, 8, 9, 18, 10, 234, 235, 236, 173, 7,
    8, 9, 18, 10, 225, 174, 7, 8, 9, 18, 10, 226, 175, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233,
    176, 7, 8, 9, 18, 10, 225, 177, 7, 8, 9, 18, 10, 226, 178, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231,
    232, 233, 179, 7, 8, 9, 18, 10, 225, 180, 7, 8, 9, 18, 10, 226, 181, 7, 8, 9, 18, 10, 3, 17, 4,
    5, 231, 232, 233, 182, 7, 8, 9, 18, 10, 225, 183, 7, 8, 9, 18, 10, 226, 184, 7, 8, 9, 18, 10,
    3, 17, 4, 5, 231, 232, 233, 185, 7, 8, 9, 18, 10, 225, 186, 7, 8, 9, 18, 10, 226, 187, 7, 8, 9,
    18, 10, 3, 17, 4, 5, 231, 232, 233, 188, 7, 8, 9, 18, 10, 225, 189, 7, 8, 9, 18, 10, 226, 190,
    7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 191, 7, 8, 9, 18, 10, 225, 192, 7, 8, 9, 18, 10,
    226, 193, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 194, 7, 8, 9, 18, 10, 225, 195, 7, 8, 9,
    18, 10, 226, 196, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 197, 7, 8, 9, 18, 10, 225, 198,
    7, 8, 9, 18, 10, 226, 199, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 200, 7, 8, 9, 18, 10,
    225, 201, 7, 8, 9, 18, 10, 226, 202, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 203, 7, 8, 9,
    18, 10, 225, 204, 7, 8, 9, 18, 10, 226, 205, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 206,
    7, 8, 9, 18, 10, 225, 207, 7, 8, 9, 18, 10, 226, 208, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232,
    233, 209, 7, 8, 9, 18, 10, 225, 210, 7, 8, 9, 18, 10, 226, 211, 7, 8, 9, 18, 10, 3, 17, 4, 5,
    231, 232, 233, 212, 7, 8, 9, 18, 10, 225, 213, 7, 8, 9, 18, 10, 226, 214, 7, 8, 9, 18, 10, 3,
    17, 4, 5, 231, 232, 233, 215, 7, 8, 9, 18, 10, 225, 216, 7, 8, 9, 18, 10, 226, 217, 7, 8, 9,
    18, 10, 3, 17, 4, 5, 231, 232, 233, 218, 7, 8, 9, 18, 10, 225, 219, 7, 8, 9, 18, 10, 226, 220,
    7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 221, 7, 8, 9, 18, 10, 225, 222, 7, 8, 9, 18, 10,
    226, 223, 7, 8, 9, 18, 10, 16, 255,
];
