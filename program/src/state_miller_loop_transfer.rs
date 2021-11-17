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

#[derive(Clone)]
pub struct MillerLoopTransferBytes {
    pub is_initialized: bool,
    pub found_root: u8,
    pub found_nullifier: u8,
    pub executed_withdraw: u8,
    pub signing_address: Vec<u8>, // is relayer address
    pub relayer_refund: u64,
    pub to_address: Vec<u8>,
    pub amount: u64,
    pub nullifier_hash: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub data_hash: Vec<u8>, // is commit hash until changed
    pub tx_integrity_hash: Vec<u8>,
    pub current_instruction_index: usize,

    // common ranges
    pub f_range: Vec<u8>,

    pub p_2_x_range: Vec<u8>,
    pub p_2_y_range: Vec<u8>,
}
impl Sealed for MillerLoopTransferBytes {}
impl IsInitialized for MillerLoopTransferBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for MillerLoopTransferBytes {
    const LEN: usize = 4972;// 1728; // optimize by 1.1k bytes

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError>{
        let input = array_ref![input,0, MillerLoopTransferBytes::LEN];

        let (
            is_initialized,
            found_root,
            found_nullifier,
            executed_withdraw,
            signing_address,
            relayer_refund,
            to_address,
            amount,
            nullifier_hash,
            root_hash,
            data_hash,
            tx_integrity_hash,
            current_instruction_index,

            f_range,

            unused_remainder0,
            //prepared inputs are parsed in these
            p_2_x_range,
            p_2_y_range,

            unused_remainder1,
        ) = array_refs![input, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 576, 2112, 48, 48, 1968];
        Ok(//216 - 32 - 8
            MillerLoopTransferBytes {
                is_initialized: true,

                found_root: found_root[0],                              //0
                found_nullifier: found_nullifier[0],                    //1
                executed_withdraw: executed_withdraw[0],                //2
                signing_address: signing_address.to_vec(),              //3
                relayer_refund: u64::from_le_bytes(*relayer_refund),    //4
                to_address: to_address.to_vec(),                        //5
                amount: u64::from_le_bytes(*amount),                    //6
                nullifier_hash: nullifier_hash.to_vec(),                //7
                root_hash: root_hash.to_vec(),                          //8
                data_hash: data_hash.to_vec(),                          //9
                tx_integrity_hash: tx_integrity_hash.to_vec(),
                current_instruction_index: usize::from_le_bytes(*current_instruction_index),

                f_range: f_range.to_vec(),

                p_2_x_range: p_2_x_range.to_vec(),
                p_2_y_range: p_2_y_range.to_vec(),

            }
        )

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MillerLoopTransferBytes::LEN];

        let (
            is_initialized_dst,
            found_root_dst,
            found_nullifier_dst,
            executed_withdraw_dst,
            signing_address_dst,
            relayer_refund_dst,
            to_address_dst,
            amount_dst,
            nullifier_hash_dst,
            root_hash_dst,
            data_hash_dst,
            tx_integrity_hash_dst,
            current_instruction_index_dst,

            f_range_dst,

            unused_remainder0_dst,
            //prepared inputs are parsed in these
            p_2_x_range_dst,
            p_2_y_range_dst,

            unused_remainder1_dst
        ) = mut_array_refs![dst,1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 576, 2112, 48, 48, 1968];

        *found_root_dst = [self.found_root.clone(); 1];
        *found_nullifier_dst = [self.found_nullifier.clone(); 1];
        *executed_withdraw_dst = [self.executed_withdraw.clone(); 1];
        *signing_address_dst = self.signing_address.clone().try_into().unwrap();
        *relayer_refund_dst = u64::to_le_bytes(self.relayer_refund);
        *to_address_dst = self.to_address.clone().try_into().unwrap();
        *amount_dst = u64::to_le_bytes(self.amount);
        *nullifier_hash_dst = self.nullifier_hash.clone().try_into().unwrap();
        *root_hash_dst = self.root_hash.clone().try_into().unwrap();
        *data_hash_dst = self.data_hash.clone().try_into().unwrap();
        *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();

        *f_range_dst = self.f_range.clone().try_into().unwrap();

        *p_2_x_range_dst = self.p_2_x_range.clone().try_into().unwrap();
        *p_2_y_range_dst = self.p_2_y_range.clone().try_into().unwrap();


        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *unused_remainder0_dst = *unused_remainder0_dst;
        *unused_remainder1_dst = *unused_remainder1_dst;
        *is_initialized_dst = [1u8; 1];
    }
}

pub const complete_instruction_order_verify_one : [u8;1821]= [ 251, 230, 237, 3, 17, 4, 5, 231, 232, 233, 20, 7, 8, 9, 18, 10, 225, 21, 7, 8, 9, 18, 10, 226, 22, 7, 8, 9, 18, 10, 234, 235, 236, 23, 7, 8, 9, 18, 10, 225, 24, 7, 8, 9, 18, 10, 226, 25, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 26, 7, 8, 9, 18, 10, 225, 27, 7, 8, 9, 18, 10, 226, 28, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 29, 7, 8, 9, 18, 10, 225, 30, 7, 8, 9, 18, 10, 226, 31, 7, 8, 9, 18, 10, 234, 235, 236, 32, 7, 8, 9, 18, 10, 225, 33, 7, 8, 9, 18, 10, 226, 34, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 35, 7, 8, 9, 18, 10, 225, 36, 7, 8, 9, 18, 10, 226, 37, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 38, 7, 8, 9, 18, 10, 225, 39, 7, 8, 9, 18, 10, 226, 40, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 41, 7, 8, 9, 18, 10, 225, 42, 7, 8, 9, 18, 10, 226, 43, 7, 8, 9, 18, 10, 234, 235, 236, 44, 7, 8, 9, 18, 10, 225, 45, 7, 8, 9, 18, 10, 226, 46, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 47, 7, 8, 9, 18, 10, 225, 48, 7, 8, 9, 18, 10, 226, 49, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 50, 7, 8, 9, 18, 10, 225, 51, 7, 8, 9, 18, 10, 226, 52, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 53, 7, 8, 9, 18, 10, 225, 54, 7, 8, 9, 18, 10, 226, 55, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 56, 7, 8, 9, 18, 10, 225, 57, 7, 8, 9, 18, 10, 226, 58, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 59, 7, 8, 9, 18, 10, 225, 60, 7, 8, 9, 18, 10, 226, 61, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 62, 7, 8, 9, 18, 10, 225, 63, 7, 8, 9, 18, 10, 226, 64, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 65, 7, 8, 9, 18, 10, 225, 66, 7, 8, 9, 18, 10, 226, 67, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 68, 7, 8, 9, 18, 10, 225, 69, 7, 8, 9, 18, 10, 226, 70, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 71, 7, 8, 9, 18, 10, 225, 72, 7, 8, 9, 18, 10, 226, 73, 7, 8, 9, 18, 10, 234, 235, 236, 74, 7, 8, 9, 18, 10, 225, 75, 7, 8, 9, 18, 10, 226, 76, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 77, 7, 8, 9, 18, 10, 225, 78, 7, 8, 9, 18, 10, 226, 79, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 80, 7, 8, 9, 18, 10, 225, 81, 7, 8, 9, 18, 10, 226, 82, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 83, 7, 8, 9, 18, 10, 225, 84, 7, 8, 9, 18, 10, 226, 85, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 86, 7, 8, 9, 18, 10, 225, 87, 7, 8, 9, 18, 10, 226, 88, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 89, 7, 8, 9, 18, 10, 225, 90, 7, 8, 9, 18, 10, 226, 91, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 92, 7, 8, 9, 18, 10, 225, 93, 7, 8, 9, 18, 10, 226, 94, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 95, 7, 8, 9, 18, 10, 225, 96, 7, 8, 9, 18, 10, 226, 97, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 98, 7, 8, 9, 18, 10, 225, 99, 7, 8, 9, 18, 10, 226, 100, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 101, 7, 8, 9, 18, 10, 225, 102, 7, 8, 9, 18, 10, 226, 103, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 104, 7, 8, 9, 18, 10, 225, 105, 7, 8, 9, 18, 10, 226, 106, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 107, 7, 8, 9, 18, 10, 225, 108, 7, 8, 9, 18, 10, 226, 109, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 110, 7, 8, 9, 18, 10, 225, 111, 7, 8, 9, 18, 10, 226, 112, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 113, 7, 8, 9, 18, 10, 225, 114, 7, 8, 9, 18, 10, 226, 115, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 116, 7, 8, 9, 18, 10, 225, 117, 7, 8, 9, 18, 10, 226, 118, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 119, 7, 8, 9, 18, 10, 225, 120, 7, 8, 9, 18, 10, 226, 121, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 122, 7, 8, 9, 18, 10, 225, 123, 7, 8, 9, 18, 10, 226, 124, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 125, 7, 8, 9, 18, 10, 225, 126, 7, 8, 9, 18, 10, 226, 127, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 128, 7, 8, 9, 18, 10, 225, 129, 7, 8, 9, 18, 10, 226, 130, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 131, 7, 8, 9, 18, 10, 225, 132, 7, 8, 9, 18, 10, 226, 133, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 134, 7, 8, 9, 18, 10, 225, 135, 7, 8, 9, 18, 10, 226, 136, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 137, 7, 8, 9, 18, 10, 225, 138, 7, 8, 9, 18, 10, 226, 139, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 140, 7, 8, 9, 18, 10, 225, 141, 7, 8, 9, 18, 10, 226, 142, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 143, 7, 8, 9, 18, 10, 225, 144, 7, 8, 9, 18, 10, 226, 145, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 146, 7, 8, 9, 18, 10, 225, 147, 7, 8, 9, 18, 10, 226, 148, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 149, 7, 8, 9, 18, 10, 225, 150, 7, 8, 9, 18, 10, 226, 151, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 152, 7, 8, 9, 18, 10, 225, 153, 7, 8, 9, 18, 10, 226, 154, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 155, 7, 8, 9, 18, 10, 225, 156, 7, 8, 9, 18, 10, 226, 157, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 158, 7, 8, 9, 18, 10, 225, 159, 7, 8, 9, 18, 10, 226, 160, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 161, 7, 8, 9, 18, 10, 225, 162, 7, 8, 9, 18, 10, 226, 163, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 164, 7, 8, 9, 18, 10, 225, 165, 7, 8, 9, 18, 10, 226, 166, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 167, 7, 8, 9, 18, 10, 225, 168, 7, 8, 9, 18, 10, 226, 169, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 170, 7, 8, 9, 18, 10, 225, 171, 7, 8, 9, 18, 10, 226, 172, 7, 8, 9, 18, 10, 234, 235, 236, 173, 7, 8, 9, 18, 10, 225, 174, 7, 8, 9, 18, 10, 226, 175, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 176, 7, 8, 9, 18, 10, 225, 177, 7, 8, 9, 18, 10, 226, 178, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 179, 7, 8, 9, 18, 10, 225, 180, 7, 8, 9, 18, 10, 226, 181, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 182, 7, 8, 9, 18, 10, 225, 183, 7, 8, 9, 18, 10, 226, 184, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 185, 7, 8, 9, 18, 10, 225, 186, 7, 8, 9, 18, 10, 226, 187, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 188, 7, 8, 9, 18, 10, 225, 189, 7, 8, 9, 18, 10, 226, 190, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 191, 7, 8, 9, 18, 10, 225, 192, 7, 8, 9, 18, 10, 226, 193, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 194, 7, 8, 9, 18, 10, 225, 195, 7, 8, 9, 18, 10, 226, 196, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 197, 7, 8, 9, 18, 10, 225, 198, 7, 8, 9, 18, 10, 226, 199, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 200, 7, 8, 9, 18, 10, 225, 201, 7, 8, 9, 18, 10, 226, 202, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 203, 7, 8, 9, 18, 10, 225, 204, 7, 8, 9, 18, 10, 226, 205, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 206, 7, 8, 9, 18, 10, 225, 207, 7, 8, 9, 18, 10, 226, 208, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 209, 7, 8, 9, 18, 10, 225, 210, 7, 8, 9, 18, 10, 226, 211, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 212, 7, 8, 9, 18, 10, 225, 213, 7, 8, 9, 18, 10, 226, 214, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 215, 7, 8, 9, 18, 10, 225, 216, 7, 8, 9, 18, 10, 226, 217, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 218, 7, 8, 9, 18, 10, 225, 219, 7, 8, 9, 18, 10, 226, 220, 7, 8, 9, 18, 10, 3, 17, 4, 5, 231, 232, 233, 221, 7, 8, 9, 18, 10, 225, 222, 7, 8, 9, 18, 10, 226, 223, 7, 8, 9, 18, 10, 16, 255];
