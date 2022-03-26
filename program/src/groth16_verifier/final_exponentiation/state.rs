use crate::utils::config::{ENCRYPTED_UTXOS_LENGTH, TMP_STORAGE_ACCOUNT_TYPE};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct FinalExponentiationState {
    is_initialized: bool,
    pub account_type: u8,
    pub signing_address: Vec<u8>,
    pub relayer_fee: Vec<u8>,
    pub recipient: Vec<u8>,
    pub amount: Vec<u8>,
    pub nullifer: Vec<u8>,
    pub f1_r_range: Vec<u8>,
    pub f_f2_range: Vec<u8>,
    pub i_range: Vec<u8>,

    pub y0_range: Vec<u8>,
    pub y1_range: Vec<u8>,
    pub y2_range: Vec<u8>,
    pub y6_range: Vec<u8>,

    pub cubic_range_0: Vec<u8>,
    pub cubic_range_1: Vec<u8>,
    pub cubic_range_2: Vec<u8>,

    pub quad_range_0: Vec<u8>,
    pub quad_range_1: Vec<u8>,
    pub quad_range_2: Vec<u8>,
    pub quad_range_3: Vec<u8>,

    pub fp256_range: Vec<u8>,

    pub current_instruction_index: usize,

    pub changed_variables: [bool; 16],
}
impl Sealed for FinalExponentiationState {}
impl IsInitialized for FinalExponentiationState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

#[allow(clippy::new_without_default)]
impl FinalExponentiationState {
    pub fn new() -> FinalExponentiationState {
        FinalExponentiationState {
            is_initialized: true,
            account_type: 0,
            signing_address: vec![0],
            relayer_fee: vec![0],
            recipient: vec![0],

            amount: vec![0],
            nullifer: vec![0],
            f1_r_range: vec![0; 384],
            f_f2_range: vec![0; 384],
            i_range: vec![0; 384],

            y0_range: vec![0; 384],
            y1_range: vec![0; 384],
            y2_range: vec![0; 384],
            y6_range: vec![0; 384],

            cubic_range_0: vec![0; 192],
            cubic_range_1: vec![0; 192],
            cubic_range_2: vec![0; 192],

            quad_range_0: vec![0; 64],
            quad_range_1: vec![0; 64],
            quad_range_2: vec![0; 64],
            quad_range_3: vec![0; 64],

            fp256_range: vec![0; 32],
            current_instruction_index: 430,
            changed_variables: [false; 16],
        }
    }
}

impl Pack for FinalExponentiationState {
    const LEN: usize = 3900 + ENCRYPTED_UTXOS_LENGTH;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, FinalExponentiationState::LEN];

        let (
            _is_initialized,
            account_type,
            _found_root,
            _unused_constants0,
            signing_address,
            relayer_fee,
            recipient,
            amount,
            nullifer,
            _unused_constants2,
            current_instruction_index,
            f_f2_range,
            //604
            f1_r_range,
            i_range,
            y0_range,
            //1756
            y1_range,
            //2140
            y2_range,
            cubic_range_0,
            cubic_range_1,
            cubic_range_2,
            quad_range_0,
            quad_range_1,
            quad_range_2,
            quad_range_3,
            fp256_range,
            y6_range,
            _unused_remainder,
        ) = array_refs![
            input,
            1,
            1,
            1,
            1,
            32,
            8,
            32,
            8,
            32,
            96,
            8,
            384,
            384,
            384,
            384,
            384,
            384,
            192,
            192,
            192,
            64,
            64,
            64,
            64,
            32,
            384,
            128 + ENCRYPTED_UTXOS_LENGTH
        ];
        if account_type[0] != TMP_STORAGE_ACCOUNT_TYPE {
            msg!("Wrong account type.");
            return Err(ProgramError::InvalidArgument);
        }
        Ok(FinalExponentiationState {
            is_initialized: true,
            account_type: account_type[0],
            signing_address: signing_address.to_vec(),
            relayer_fee: relayer_fee.to_vec(),
            recipient: recipient.to_vec(),

            amount: amount.to_vec(),
            nullifer: nullifer.to_vec(),
            f_f2_range: f_f2_range.to_vec(),
            f1_r_range: f1_r_range.to_vec(),
            i_range: i_range.to_vec(),
            y0_range: y0_range.to_vec(),
            y1_range: y1_range.to_vec(),
            y2_range: y2_range.to_vec(),

            cubic_range_0: cubic_range_0.to_vec(),
            cubic_range_1: cubic_range_1.to_vec(),
            cubic_range_2: cubic_range_2.to_vec(),

            quad_range_0: quad_range_0.to_vec(),
            quad_range_1: quad_range_1.to_vec(),
            quad_range_2: quad_range_2.to_vec(),
            quad_range_3: quad_range_3.to_vec(),

            fp256_range: fp256_range.to_vec(),
            y6_range: y6_range.to_vec(),

            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            changed_variables: [false; 16],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, FinalExponentiationState::LEN];

        let (
            _is_initialized_dst,
            _account_type_dst,
            _found_root_dst,
            _unused_constants_dst,
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
            y6_range_dst,
            _unused_remainder_dst,
        ) = mut_array_refs![
            dst,
            1,
            1,
            1,
            209,
            8,
            384,
            384,
            384,
            384,
            384,
            384,
            192,
            192,
            192,
            64,
            64,
            64,
            64,
            32,
            384,
            128 + ENCRYPTED_UTXOS_LENGTH
        ];

        for (i, variable_has_changed) in self.changed_variables.iter().enumerate() {
            if *variable_has_changed {
                if i == 0 {
                    *f_f2_range_dst = self.f_f2_range.clone().try_into().unwrap();
                } else if i == 1 {
                    *f1_r_range_dst = self.f1_r_range.clone().try_into().unwrap();
                } else if i == 2 {
                    *i_range_dst = self.i_range.clone().try_into().unwrap();
                } else if i == 3 {
                    *y0_range_dst = self.y0_range.clone().try_into().unwrap();
                } else if i == 4 {
                    *y1_range_dst = self.y1_range.clone().try_into().unwrap();
                } else if i == 5 {
                    *y2_range_dst = self.y2_range.clone().try_into().unwrap();
                } else if i == 6 {
                    *cubic_range_0_dst = self.cubic_range_0.clone().try_into().unwrap();
                } else if i == 7 {
                    *cubic_range_1_dst = self.cubic_range_1.clone().try_into().unwrap();
                } else if i == 8 {
                    *cubic_range_2_dst = self.cubic_range_2.clone().try_into().unwrap();
                } else if i == 9 {
                    *quad_range_0_dst = self.quad_range_0.clone().try_into().unwrap();
                } else if i == 10 {
                    *quad_range_1_dst = self.quad_range_1.clone().try_into().unwrap();
                } else if i == 11 {
                    *quad_range_2_dst = self.quad_range_2.clone().try_into().unwrap();
                } else if i == 12 {
                    *quad_range_3_dst = self.quad_range_3.clone().try_into().unwrap();
                } else if i == 13 {
                    *fp384_range_dst = self.fp256_range.clone().try_into().unwrap();
                } else if i == 14 {
                    *y6_range_dst = self.y6_range.clone().try_into().unwrap();
                }
            }
        }

        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
    }
}
