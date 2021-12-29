use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

#[derive(Clone)]
pub struct LiBytes {
    is_initialized: bool,
    pub found_root: u8,
    pub found_nullifier: u8,
    pub executed_withdraw: u8,
    pub signing_address: Vec<u8>, // is relayer address
    pub relayer_refund: Vec<u8>,
    pub to_address: Vec<u8>,
    pub amount: Vec<u8>,
    pub nullifier_hash: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub data_hash: Vec<u8>,         // is commit hash until changed
    pub tx_integrity_hash: Vec<u8>, // is calculated on-chain from to_address, amount, signing_address,
    pub current_instruction_index: usize,
    pub proof_a_b_c_leaves_and_nullifiers: Vec<u8>,
    pub changed_constants: [bool; 12],
}
impl Sealed for LiBytes {}
impl IsInitialized for LiBytes {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for LiBytes {
    const LEN: usize = 3900; // 1020

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LiBytes::LEN];

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
            //220
            unused_remainder,
            proof_a_b_c_leaves_and_nullifiers,
        ) = array_refs![
            input, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 3296, 384
        ];
        msg!("unpacked");

        Ok(LiBytes {
            is_initialized: true,

            found_root: found_root[0],                     //0
            found_nullifier: found_nullifier[0],           //1
            executed_withdraw: executed_withdraw[0],       //2
            signing_address: signing_address.to_vec(),     //3
            relayer_refund: relayer_refund.to_vec(),       //4
            to_address: to_address.to_vec(),               //5
            amount: amount.to_vec(),                       //6
            nullifier_hash: nullifier_hash.to_vec(),       //7
            root_hash: root_hash.to_vec(),                 //8
            data_hash: data_hash.to_vec(),                 //9
            tx_integrity_hash: tx_integrity_hash.to_vec(), //10
            proof_a_b_c_leaves_and_nullifiers: proof_a_b_c_leaves_and_nullifiers.to_vec(),//11

            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            changed_constants: [false; 12],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, LiBytes::LEN];

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
            unused_remainder_dst,
            proof_a_b_c_leaves_and_nullifiers_dst,
        ) = mut_array_refs![
            dst, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 3296, 384];

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
                    *relayer_refund_dst = self.relayer_refund.clone().try_into().unwrap();
                } else if i == 5 {
                    *to_address_dst = self.to_address.clone().try_into().unwrap();
                } else if i == 6 {
                    *amount_dst = self.amount.clone().try_into().unwrap();
                } else if i == 7 {
                    *nullifier_hash_dst = self.nullifier_hash.clone().try_into().unwrap();
                } else if i == 8 {
                    *root_hash_dst = self.root_hash.clone().try_into().unwrap();
                } else if i == 9 {
                    *data_hash_dst = self.data_hash.clone().try_into().unwrap();
                } else if i == 10 {
                    *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
                } else if i == 11 {
                    *proof_a_b_c_leaves_and_nullifiers_dst = self.proof_a_b_c_leaves_and_nullifiers.clone().try_into().unwrap();
                }
            }
        }
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
        *is_initialized_dst = *is_initialized_dst;
    }
}


// Account struct to determine state of the computation
// and perform basic security checks
#[derive(Debug, Clone)]
pub struct InstructionIndex {
    is_initialized: bool,
    pub signer_pubkey: Pubkey,
    pub current_instruction_index: usize,

}

impl Sealed for InstructionIndex {}

impl IsInitialized for InstructionIndex {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for InstructionIndex {
    const LEN: usize = 3900;//3772;
    fn unpack_from_slice(input:  &[u8]) ->  Result<Self, ProgramError>{
        let input = array_ref![input, 0, InstructionIndex::LEN];

        let (
            is_initialized,
            unused_remainder0,
            signer_pubkey,
            unused_remainder1,
            current_instruction_index,
            unused_remainder2
        ) = array_refs![input,1, 3, 32, 176, 8, 3680];
        
        if is_initialized[0] == 0 {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(
                InstructionIndex {
                    is_initialized: true,
                    signer_pubkey: solana_program::pubkey::Pubkey::new(signer_pubkey),
                    current_instruction_index: usize::from_le_bytes(*current_instruction_index),
                }
            )
        }

    }

    fn pack_into_slice(&self, dst: &mut [u8]) {

        let dst = array_mut_ref![dst, 0,  InstructionIndex::LEN];

        let (
            is_initialized,
            unused_remainder0,
            current_instruction_index,
            unused_remainder1
        ) = mut_array_refs![dst, 1, 211, 8, 3680];
        //is not meant to be used
        *is_initialized = *is_initialized;

    }
}
