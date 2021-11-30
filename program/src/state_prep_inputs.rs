// parsers:
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::{Fp256, Fp384};
// use ark_ec;
use ark_bls12_381;
use ark_ed_on_bls12_381;
use ark_ff::fields::models::quadratic_extension::{QuadExtField, QuadExtParameters};
use num_traits::One;

//lib

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use ark_ec;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryInto;

use byteorder::ByteOrder;
use byteorder::LittleEndian;

pub const X_1_RANGE_INDEX: usize = 1;
pub const G_IC_Z_RANGE_INDEX: usize = 13;

pub const PREPARED_INPUTS_PUBKEY: [u8; 32] = [
    60, 154, 109, 195, 223, 3, 136, 142, 102, 53, 218, 4, 253, 75, 27, 214, 245, 169, 87, 20, 37,
    44, 14, 91, 112, 106, 250, 252, 128, 107, 85, 11,
];

// Account struct for prepare_inputs/p
#[derive(Clone)]
pub struct PrepareInputsBytes {
    is_initialized: bool,
    pub found_root: u8,
    pub found_nullifier: u8,
    pub executed_withdraw: u8,
    //adding 3
    //constants
    pub signing_address: Vec<u8>, // is relayer address
    pub relayer_refund: u64,
    pub to_address: Vec<u8>,
    pub amount: u64,
    pub nullifier_hash: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub data_hash: Vec<u8>,         // is commit hash until changed
    pub tx_integrity_hash: Vec<u8>, // is calculated on-chain from to_address, amount, signing_address,
    //root does not have to be saved for it is looked for immediately when added
    //adding 32 + 8 + 32 + 8 + 32 + 32 + 32 = 176
    //total added 3 + 176 = 179
    //memory variables
    pub i_1_range: Vec<u8>,
    pub x_1_range: Vec<u8>,
    pub i_2_range: Vec<u8>,
    pub x_2_range: Vec<u8>,
    pub i_3_range: Vec<u8>,
    pub x_3_range: Vec<u8>,

    pub i_4_range: Vec<u8>,
    pub x_4_range: Vec<u8>,
    pub res_x_range: Vec<u8>,
    pub res_y_range: Vec<u8>,
    pub res_z_range: Vec<u8>,
    pub g_ic_x_range: Vec<u8>,
    pub g_ic_y_range: Vec<u8>,
    pub g_ic_z_range: Vec<u8>,
    pub current_instruction_index: usize,

    pub changed_variables: [bool; 14],
    pub changed_constants: [bool; 11],
}

impl Sealed for PrepareInputsBytes {}
impl IsInitialized for PrepareInputsBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for PrepareInputsBytes {
    const LEN: usize = 4972; // 1020

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, PrepareInputsBytes::LEN];

        let (
            is_initialized,
            found_root,
            found_nullifier,
            executed_withdraw,
            signing_address, // is relayer address
            relayer_refund,
            to_address,
            amount,
            nullifier_hash,
            root_hash,
            data_hash, // is commit hash until changed
            tx_integrity_hash,
            current_instruction_index,
            i_1_range, // 32b
            x_1_range, // 96b + constructor
            i_2_range,
            x_2_range,
            i_3_range,
            x_3_range,
            i_4_range,
            x_4_range,
            res_x_range,
            res_y_range,
            res_z_range,
            g_ic_x_range,
            g_ic_y_range,
            g_ic_z_range, // 144b 3*48
            //until here 1020 bytes
            unused_remainder,
        ) = array_refs![
            input, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 32, 96, 32, 96, 32, 96, 32, 96, 48,
            48, 48, 48, 48, 48, 3952
        ];
        Ok(PrepareInputsBytes {
            is_initialized: true,

            found_root: found_root[0],                           //0
            found_nullifier: found_nullifier[0],                 //1
            executed_withdraw: executed_withdraw[0],             //2
            signing_address: signing_address.to_vec(),           //3
            relayer_refund: u64::from_le_bytes(*relayer_refund), //4
            to_address: to_address.to_vec(),                     //5
            amount: u64::from_le_bytes(*amount),                 //6
            nullifier_hash: nullifier_hash.to_vec(),             //7
            root_hash: root_hash.to_vec(),                       //8
            data_hash: data_hash.to_vec(),                       //9
            tx_integrity_hash: tx_integrity_hash.to_vec(),       //10

            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            i_1_range: i_1_range.to_vec(),       //0
            x_1_range: x_1_range.to_vec(),       //1
            i_2_range: i_2_range.to_vec(),       //2
            x_2_range: x_2_range.to_vec(),       //3
            i_3_range: i_3_range.to_vec(),       //4
            x_3_range: x_3_range.to_vec(),       //5
            i_4_range: i_4_range.to_vec(),       //6
            x_4_range: x_4_range.to_vec(),       //7
            res_x_range: res_x_range.to_vec(),   //8
            res_y_range: res_y_range.to_vec(),   //9
            res_z_range: res_z_range.to_vec(),   //10
            g_ic_x_range: g_ic_x_range.to_vec(), //11
            g_ic_y_range: g_ic_y_range.to_vec(), //12
            g_ic_z_range: g_ic_z_range.to_vec(), //13
            changed_variables: [false; 14],
            changed_constants: [false; 11],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, PrepareInputsBytes::LEN];

        let (
            //constants
            is_initialized_dst,
            found_root_dst,
            found_nullifier_dst,
            executed_withdraw_dst,
            signing_address_dst, // is relayer address
            relayer_refund_dst,
            to_address_dst,
            amount_dst,
            nullifier_hash_dst,
            root_hash_dst,
            data_hash_dst,
            tx_integrity_hash_dst,
            //variables
            current_instruction_index_dst,
            //220
            i_1_range_dst,
            x_1_range_dst,
            i_2_range_dst,
            x_2_range_dst,
            i_3_range_dst,
            x_3_range_dst,
            i_4_range_dst,
            x_4_range_dst,
            res_x_range_dst,
            res_y_range_dst,
            res_z_range_dst,
            g_ic_x_range_dst,
            g_ic_y_range_dst,
            g_ic_z_range_dst,
            unused_remainder_dst,
        ) = mut_array_refs![
            dst, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 32, 96, 32, 96, 32, 96, 32, 96, 48,
            48, 48, 48, 48, 48, 3952
        ];

        for (i, var_has_changed) in self.changed_variables.iter().enumerate() {
            if *var_has_changed {
                if i == 0 {
                    *i_1_range_dst = self.i_1_range.clone().try_into().unwrap();
                } else if i == 1 {
                    *x_1_range_dst = self.x_1_range.clone().try_into().unwrap();
                } else if i == 2 {
                    *i_2_range_dst = self.i_2_range.clone().try_into().unwrap();
                } else if i == 3 {
                    *x_2_range_dst = self.x_2_range.clone().try_into().unwrap();
                } else if i == 4 {
                    *i_3_range_dst = self.i_3_range.clone().try_into().unwrap();
                } else if i == 5 {
                    *x_3_range_dst = self.x_3_range.clone().try_into().unwrap();
                } else if i == 6 {
                    *i_4_range_dst = self.i_4_range.clone().try_into().unwrap();
                } else if i == 7 {
                    *x_4_range_dst = self.x_4_range.clone().try_into().unwrap();
                } else if i == 8 {
                    *res_x_range_dst = self.res_x_range.clone().try_into().unwrap();
                } else if i == 9 {
                    *res_y_range_dst = self.res_y_range.clone().try_into().unwrap();
                } else if i == 10 {
                    *res_z_range_dst = self.res_z_range.clone().try_into().unwrap();
                } else if i == 11 {
                    *g_ic_x_range_dst = self.g_ic_x_range.clone().try_into().unwrap();
                } else if i == 12 {
                    *g_ic_y_range_dst = self.g_ic_y_range.clone().try_into().unwrap();
                } else if i == 13 {
                    *g_ic_z_range_dst = self.g_ic_z_range.clone().try_into().unwrap();
                }
            } else {
                if i == 0 {
                    *i_1_range_dst = *i_1_range_dst;
                } else if i == 1 {
                    *x_1_range_dst = *x_1_range_dst;
                } else if i == 2 {
                    *i_2_range_dst = *i_2_range_dst;
                } else if i == 3 {
                    *x_2_range_dst = *x_2_range_dst;
                } else if i == 4 {
                    *i_3_range_dst = *i_3_range_dst;
                } else if i == 5 {
                    *x_3_range_dst = *x_3_range_dst;
                } else if i == 6 {
                    *i_4_range_dst = *i_4_range_dst;
                } else if i == 7 {
                    *x_4_range_dst = *x_4_range_dst;
                } else if i == 8 {
                    *res_x_range_dst = *res_x_range_dst;
                } else if i == 9 {
                    *res_y_range_dst = *res_y_range_dst;
                } else if i == 10 {
                    *res_z_range_dst = *res_z_range_dst;
                } else if i == 11 {
                    *g_ic_x_range_dst = *g_ic_x_range_dst;
                } else if i == 12 {
                    *g_ic_y_range_dst = *g_ic_y_range_dst;
                } else if i == 13 {
                    *g_ic_z_range_dst = *g_ic_z_range_dst;
                }
            };
        }

        for (i, const_has_changed) in self.changed_constants.iter().enumerate() {
            if *const_has_changed {
                if i == 0 {
                    *found_root_dst = [self.found_root.clone(); 1];
                } else if i == 1 {
                    *found_nullifier_dst = [self.found_nullifier.clone(); 1];
                } else if i == 2 {
                    *executed_withdraw_dst = [self.executed_withdraw.clone(); 1];
                } else if i == 3 {
                    *signing_address_dst = self.signing_address.clone().try_into().unwrap();
                } else if i == 4 {
                    *relayer_refund_dst = u64::to_le_bytes(self.relayer_refund);
                } else if i == 5 {
                    *to_address_dst = self.to_address.clone().try_into().unwrap();
                } else if i == 6 {
                    *amount_dst = u64::to_le_bytes(self.amount);
                } else if i == 7 {
                    *nullifier_hash_dst = self.nullifier_hash.clone().try_into().unwrap();
                } else if i == 8 {
                    *root_hash_dst = self.root_hash.clone().try_into().unwrap();
                } else if i == 9 {
                    *data_hash_dst = self.data_hash.clone().try_into().unwrap();
                } else if i == 10 {
                    *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
                }
            } else {
                if i == 0 {
                    *found_root_dst = *found_root_dst;
                } else if i == 1 {
                    *found_nullifier_dst = *found_nullifier_dst;
                } else if i == 2 {
                    *executed_withdraw_dst = *executed_withdraw_dst;
                } else if i == 3 {
                    *signing_address_dst = *signing_address_dst;
                } else if i == 4 {
                    *relayer_refund_dst = *relayer_refund_dst;
                } else if i == 5 {
                    *to_address_dst = *to_address_dst;
                } else if i == 6 {
                    *amount_dst = *amount_dst;
                } else if i == 7 {
                    *nullifier_hash_dst = *nullifier_hash_dst;
                } else if i == 8 {
                    *root_hash_dst = *root_hash_dst;
                } else if i == 9 {
                    *data_hash_dst = *data_hash_dst;
                } else if i == 10 {
                    *tx_integrity_hash_dst = *tx_integrity_hash_dst;
                }
            };
        }

        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *is_initialized_dst = [1u8; 1];
        *unused_remainder_dst = *unused_remainder_dst;
    }
}

// Account structs for merkle tree:
#[derive(Debug)]
pub struct PoseidonHashBytesPrepInputs {
    pub is_initialized: bool,
    pub state_range_1: Vec<u8>,
    pub state_range_2: Vec<u8>,
    pub state_range_3: Vec<u8>,
    // the following 4 are just read not written
    pub signing_address: [u8; 32], // is relayer address
    pub relayer_refund: [u8; 8],
    pub to_address: [u8; 32],
    pub amount: [u8; 8],

    pub tx_integrity_hash: Vec<u8>,

    pub current_instruction_index: usize,
}
impl Sealed for PoseidonHashBytesPrepInputs {}
impl IsInitialized for PoseidonHashBytesPrepInputs {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for PoseidonHashBytesPrepInputs {
    const LEN: usize = 4972;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, PoseidonHashBytesPrepInputs::LEN];

        let (
            is_initialized,
            unused_first,
            signing_address, // is relayer address
            relayer_refund,
            to_address,
            amount,
            unused_second,
            tx_integrity_hash,
            current_instruction_index,
            //220
            unused_third,
            state_range_1,
            state_range_2,
            state_range_3,
        ) = array_refs![input, 1, 3, 32, 8, 32, 8, 96, 32, 8, 4656, 32, 32, 32];

        assert_eq!(1u8, is_initialized[0]);
        Ok(PoseidonHashBytesPrepInputs {
            is_initialized: true,
            state_range_1: state_range_1.to_vec(),
            state_range_2: state_range_2.to_vec(),
            state_range_3: state_range_3.to_vec(),
            signing_address: *signing_address, //3
            relayer_refund: *relayer_refund,   //4
            to_address: *to_address,           //5
            amount: *amount,                   //6
            tx_integrity_hash: tx_integrity_hash.to_vec(),
            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, PoseidonHashBytesPrepInputs::LEN];
        let (
            is_initialized_dst,
            unused_first_dst,
            tx_integrity_hash_dst,
            current_instruction_index_dst,
            //219
            unused_third_dst,
            state_range_1_dst,
            state_range_2_dst,
            state_range_3_dst,
        ) = mut_array_refs![dst, 1, 179, 32, 8, 4656, 32, 32, 32];

        /*
        let (
            is_initialized_dst,
            unused_first_dst,
            result_dst,
            current_instruction_index_dst,
            unused_second_dst,
            state_range_1_dst,
            state_range_2_dst,
            state_range_3_dst,
        ) = mut_array_refs![dst,1, 146, 32, 8, 4478, 32, 32, 32];
        */
        *state_range_1_dst = self.state_range_1.clone().try_into().unwrap();
        //assert_eq!(*state_range_1_dst.to_vec(), self.state_range_1);
        *state_range_2_dst = self.state_range_2.clone().try_into().unwrap();
        //assert_eq!(*state_range_2_dst.to_vec(), self.state_range_2);
        *state_range_3_dst = self.state_range_3.clone().try_into().unwrap();
        //assert_eq!(*state_range_3_dst.to_vec(), self.state_range_3);
        *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
        msg!("packing {}", self.current_instruction_index);
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *unused_first_dst = *unused_first_dst;
        *unused_third_dst = *unused_third_dst;
        *is_initialized_dst = *is_initialized_dst;
    }
}

// x
pub fn parse_x_group_affine_from_bytes(
    account: &Vec<u8>,
) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters> {
    let x = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bls12_381::g1::Parameters>::new(
        <Fp384<ark_bls12_381::FqParameters> as FromBytes>::read(&account[0..48]).unwrap(),
        <Fp384<ark_bls12_381::FqParameters> as FromBytes>::read(&account[48..96]).unwrap(),
        false,
    );
    x
}

pub fn parse_x_group_affine_to_bytes(
    x: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>,
    account: &mut Vec<u8>,
) {
    //println!("Parsing {:?}", c.c0);
    // parse_fp384_to_bytes(x.x, acc1, range1);
    // parse_fp384_to_bytes(x.y, acc2, range2);
    <Fp384<ark_bls12_381::FqParameters> as ToBytes>::write(&x.x, &mut account[0..48]);
    <Fp384<ark_bls12_381::FqParameters> as ToBytes>::write(&x.y, &mut account[48..96]);
}

// res,g_ic

pub fn parse_group_projective_from_bytes(
    acc1: &Vec<u8>,
    acc2: &Vec<u8>,
    acc3: &Vec<u8>,
) -> ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters> {
    let res =
        ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bls12_381::g1::Parameters>::new(
            <Fp384<ark_bls12_381::FqParameters> as FromBytes>::read(&acc1[0..48]).unwrap(),
            <Fp384<ark_bls12_381::FqParameters> as FromBytes>::read(&acc2[0..48]).unwrap(),
            <Fp384<ark_bls12_381::FqParameters> as FromBytes>::read(&acc3[0..48]).unwrap(),
        );
    res
}

pub fn parse_group_projective_to_bytes(
    res: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters>,
    acc1: &mut Vec<u8>,
    acc2: &mut Vec<u8>,
    acc3: &mut Vec<u8>,
) {
    //println!("Parsing {:?}", c.c0);
    // parse_fp384_to_bytes(res.x, acc1, range1);
    // parse_fp384_to_bytes(res.y, acc2, range2);
    // parse_fp384_to_bytes(res.z, acc3, range3);

    <Fp384<ark_bls12_381::FqParameters> as ToBytes>::write(&res.x, &mut acc1[0..48]);
    <Fp384<ark_bls12_381::FqParameters> as ToBytes>::write(&res.y, &mut acc2[0..48]);
    <Fp384<ark_bls12_381::FqParameters> as ToBytes>::write(&res.z, &mut acc3[0..48]);
}
