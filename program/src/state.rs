use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

#[derive(Clone)]
pub struct ChecksAndTransferState {
    is_initialized: bool,
    pub found_root: u8,
    pub found_nullifier: u8,
    pub merkle_tree_index: u8,
    pub signing_address: Vec<u8>, // is relayer address
    pub relayer_fees: Vec<u8>,
    pub recipient: Vec<u8>,
    pub ext_amount: Vec<u8>,
    pub amount: Vec<u8>,
    pub root_hash: Vec<u8>,
    pub tx_integrity_hash: Vec<u8>, // is calculated on-chain from recipient, ext_amount, signing_address,
    pub current_instruction_index: usize,
    pub proof_a_b_c_leaves_and_nullifiers: Vec<u8>,
    // set changed_constants to true to pack specified values other values will not be packed
    pub changed_constants: [bool; 11],
}
impl Sealed for ChecksAndTransferState {}
impl IsInitialized for ChecksAndTransferState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for ChecksAndTransferState {
    const LEN: usize = 3900; // 1020

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, ChecksAndTransferState::LEN];

        let (
            _is_initialized,
            found_root,
            found_nullifier,
            merkle_tree_index,
            signing_address, // is relayer address
            relayer_fees,
            recipient,
            ext_amount,
            amount,
            root_hash,
            _unused, // is commit hash until changed
            tx_integrity_hash,
            current_instruction_index,
            //220
            _unused_remainder,
            proof_a_b_c_leaves_and_nullifiers,
        ) = array_refs![input, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 3296, 384]; // 8->32 -- 24+ (old rem: 3296)

        Ok(ChecksAndTransferState {
            is_initialized: true,

            found_root: found_root[0],                     //0 legacy remove
            found_nullifier: found_nullifier[0],           //1 legacy remove
            merkle_tree_index: merkle_tree_index[0],       //2 legacy remove
            signing_address: signing_address.to_vec(),     //3
            relayer_fees: relayer_fees.to_vec(),           //4
            recipient: recipient.to_vec(),                 //5
            ext_amount: ext_amount.to_vec(),               //6
            amount: amount.to_vec(),                       //7
            root_hash: root_hash.to_vec(),                 //8
            tx_integrity_hash: tx_integrity_hash.to_vec(), //10
            proof_a_b_c_leaves_and_nullifiers: proof_a_b_c_leaves_and_nullifiers.to_vec(), //11

            current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            changed_constants: [false; 11],
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, ChecksAndTransferState::LEN];

        let (
            //constants
            _is_initialized_dst,
            found_root_dst,
            found_nullifier_dst,
            merkle_tree_index_dst,
            signing_address_dst, // is relayer address
            relayer_fees_dst,
            recipient_dst,
            ext_amount_dst,
            amount_dst,
            root_hash_dst,
            _unused_dst,
            tx_integrity_hash_dst,
            //variables
            current_instruction_index_dst,
            //220
            _unused_remainder_dst,
            proof_a_b_c_leaves_and_nullifiers_dst,
        ) = mut_array_refs![dst, 1, 1, 1, 1, 32, 8, 32, 8, 32, 32, 32, 32, 8, 3296, 384];

        for (i, const_has_changed) in self.changed_constants.iter().enumerate() {
            if *const_has_changed {
                if i == 0 {
                    *found_root_dst = [self.found_root; 1];
                } else if i == 1 {
                    *found_nullifier_dst = [self.found_nullifier; 1];
                } else if i == 2 {
                    *merkle_tree_index_dst = [self.merkle_tree_index; 1];
                } else if i == 3 {
                    *signing_address_dst = self.signing_address.clone().try_into().unwrap();
                } else if i == 4 {
                    *relayer_fees_dst = self.relayer_fees.clone().try_into().unwrap();
                } else if i == 5 {
                    *recipient_dst = self.recipient.clone().try_into().unwrap();
                } else if i == 6 {
                    *ext_amount_dst = self.ext_amount.clone().try_into().unwrap();
                } else if i == 7 {
                    *amount_dst = self.amount.clone().try_into().unwrap();
                } else if i == 8 {
                    *root_hash_dst = self.root_hash.clone().try_into().unwrap();
                } else if i == 9 {
                    *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
                } else if i == 10 {
                    *proof_a_b_c_leaves_and_nullifiers_dst = self
                        .proof_a_b_c_leaves_and_nullifiers
                        .clone()
                        .try_into()
                        .unwrap();
                }
            }
        }
        *current_instruction_index_dst = usize::to_le_bytes(self.current_instruction_index);
    }
}

// Account struct to determine state of the computation
// and perform basic security checks.
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
    const LEN: usize = 3900;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, InstructionIndex::LEN];

        let (
            is_initialized,
            _unused_remainder0,
            signer_pubkey,
            _unused_remainder1,
            current_instruction_index,
            _unused_remainder2,
        ) = array_refs![input, 1, 3, 32, 176, 8, 3680];
        if is_initialized[0] == 0 {
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(InstructionIndex {
                is_initialized: true,
                signer_pubkey: solana_program::pubkey::Pubkey::new(signer_pubkey),
                current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            })
        }
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {}
}
