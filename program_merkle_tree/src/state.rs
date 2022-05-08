use crate::utils::config::{MERKLE_TREE_TMP_PDA_SIZE, MERKLE_TREE_TMP_STORAGE_ACCOUNT_TYPE, ENCRYPTED_UTXOS_LENGTH,NULLIFIERS_LENGTH, TMP_STORAGE_ACCOUNT_TYPE};
use crate::IX_ORDER;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

#[derive(Clone, Debug)]
pub struct MerkleTreeTmpPda {
    pub is_initialized: bool,
    pub found_root: u8,
    pub account_type: u8,
    pub root_hash: Vec<u8>,
    pub amount:  Vec<u8>,
    pub ext_amount:  Vec<u8>,
    pub ext_sol_amount:  Vec<u8>,
    pub relayer_fee:  Vec<u8>,

    pub tx_integrity_hash:  Vec<u8>,
    pub nullifiers:  Vec<u8>,
    pub node_left:  Vec<u8>,
    pub node_right:  Vec<u8>,
    pub leaf_left: Vec<u8>,
    pub leaf_right: Vec<u8>,
    pub recipient:  Vec<u8>,
    pub verifier_index: usize,
    pub encrypted_utxos:  Vec<u8>,
    pub verifier_tmp_pda:  Vec<u8>,
    pub relayer:  Vec<u8>,
    pub merkle_tree_index: usize,

    //
    pub state: Vec<u8>,
    pub current_round: usize,
    pub current_round_index: usize,
    pub current_instruction_index: usize,
    pub current_index: usize,
    pub current_level: usize,
    pub current_level_hash: Vec<u8>,
    // set changed_constants to true to pack specified values other values will not be packed
    pub changed_state: u8,
}
impl MerkleTreeTmpPda {
    pub fn new () -> MerkleTreeTmpPda {
        MerkleTreeTmpPda {
            is_initialized: true,
            found_root: 0,
            account_type: 6,
            root_hash: vec![0u8],
            amount:  vec![0u8],
            ext_amount:  vec![0u8],
            ext_sol_amount:  vec![0u8],
            relayer_fee:  vec![0u8],

            tx_integrity_hash:  vec![0u8],
            nullifiers:  vec![0u8],
            node_left:  vec![0u8],
            node_right:  vec![0u8],
            leaf_left:  vec![0u8],
            leaf_right:  vec![0u8],
            recipient:  vec![0u8],
            verifier_index: 0,
            encrypted_utxos:  vec![0u8],
            verifier_tmp_pda:  vec![0u8],
            relayer:  vec![0u8],
            merkle_tree_index: 0,

            state: vec![0u8],
            current_round: 0,
            current_round_index: 0,
            current_instruction_index: 0,
            current_index: 0,
            current_level: 0,
            current_level_hash: vec![0],
            // set changed_constants to true to pack specified values other values will not be packed
            changed_state: 1
    }
    }
}
impl Sealed for MerkleTreeTmpPda {}
impl IsInitialized for MerkleTreeTmpPda {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerkleTreeTmpPda {
    const LEN: usize =  MERKLE_TREE_TMP_PDA_SIZE;//3900; // 1020
    // for 2 nullifiers 729
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, MerkleTreeTmpPda::LEN];

        let (
            _is_initialized,
            account_type,
            current_instruction_index,
            found_root,
            verifier_index,
            merkle_tree_index,
            relayer, // is relayer address
            relayer_fee,
            recipient,
            ext_amount,
            ext_sol_amount,
            amount,
            root_hash,
            tx_integrity_hash,
            verifier_tmp_pda,
            state,
            current_round,
            current_round_index,
            current_index,
            current_level,
            current_level_hash,

            node_left,
            node_right,
            leaf_left,
            leaf_right,
            encrypted_utxos,
            nullifiers,
        ) = array_refs![
            input,
            1, //inited
            1, // account type
            8, // current instruction index
            1, // found_root
            8, // verifier_index
            8, // merkle_tree_index
            32,// relayer/signer
            8, // relayer_fee
            32,// recipient
            8, // ext_amount
            32, // ext_sol_amount
            32,// amount
            32,// root_hash
            32,// tx_integrity_hash
            32,// verifier_tmp_pda

            96,// poseidon state
            8, // current round
            8, // current round index
            8, // current index
            8, // current level
            32, // current level hash

            32, //node_left
            32, //node_right
            32, //leaf_left
            32, //leaf_right
            ENCRYPTED_UTXOS_LENGTH,
            NULLIFIERS_LENGTH
        ];

        if _is_initialized[0] != 0u8 && account_type[0] != MERKLE_TREE_TMP_STORAGE_ACCOUNT_TYPE {
            msg!("Wrong account type.");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(MerkleTreeTmpPda {
            is_initialized: true,
            found_root: found_root[0],                     //0
            account_type: account_type[0],
            verifier_index: usize::from_le_bytes(*verifier_index),
            current_instruction_index: usize::from_le_bytes(*current_instruction_index),                //1
            merkle_tree_index: usize::from_le_bytes(*merkle_tree_index),       //2
            relayer: relayer.to_vec(),     //3
            relayer_fee: relayer_fee.to_vec(),             //4

            ext_sol_amount: ext_sol_amount.to_vec(),
            ext_amount: ext_amount.to_vec(),               //6
            amount: amount.to_vec(),                       //7
            root_hash: root_hash.to_vec(),                 //8
            tx_integrity_hash: tx_integrity_hash.to_vec(), //10
            node_left: node_left.to_vec(),
            node_right: node_right.to_vec(),
            leaf_left: leaf_left.to_vec(),
            leaf_right: leaf_right.to_vec(),
            encrypted_utxos: encrypted_utxos.to_vec(),
            recipient: recipient.to_vec(),                 //5
            verifier_tmp_pda: verifier_tmp_pda.to_vec(),
            nullifiers: nullifiers.to_vec(),
            state: state.to_vec(),
            current_round: usize::from_le_bytes(*current_round),
            current_round_index: usize::from_le_bytes(*current_round_index),
            current_index: usize::from_le_bytes(*current_index),
            current_level: usize::from_le_bytes(*current_level),
            current_level_hash: current_level_hash.to_vec(),
            changed_state: 0,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerkleTreeTmpPda::LEN];

        let (
            _is_initialized,
            account_type_dst,
            current_instruction_index_dst,
            found_root_dst,
            verifier_index_dst,
            merkle_tree_index_dst,
            relayer_dst, // is relayer address
            relayer_fee_dst,
            recipient_dst,
            ext_amount_dst,
            ext_sol_amount_dst,
            amount_dst,
            root_hash_dst,
            tx_integrity_hash_dst,

            verifier_tmp_pda_dst,

            state_dst,
            current_round_dst,
            current_round_index_dst,
            current_index_dst, // current index
            current_level_dst, // current level
            current_level_hash_dst, // current level hash

            node_left_dst,
            node_right_dst,
            leaf_left_dst,
            leaf_right_dst,
            encrypted_utxos_dst,
            nullifiers_dst,
        ) = mut_array_refs![
            dst,
            1, //inited
            1, // account type
            8, // current instruction index
            1, // found_root
            8, // verifier_index
            8, // merkle_tree_index
            32,// relayer/signer
            8, // relayer_fee
            32,// recipient
            8, // ext_amount
            32, // ext_sol_amount
            32,// amount
            32,// root_hash
            32,// tx_integrity_hash
            32,// verifier_tmp_pda

            96,// poseidon state
            8, // current round
            8, // current round index
            8, // current index
            8, // current level
            32, // current level hash

            32, //node_left
            32, //node_right
            32, //leaf_left
            32, //leaf_right
            ENCRYPTED_UTXOS_LENGTH,
            NULLIFIERS_LENGTH
        ];
        if self.changed_state == 1 {
            msg!("pack recipient: {:?}", self.ext_sol_amount);
            msg!("ext_sol_amount_dst: {:?}", ext_sol_amount_dst);

            *account_type_dst = [self.account_type; 1];
            *found_root_dst = [self.found_root; 1];
            *verifier_index_dst = usize::to_le_bytes(self.verifier_index);
            *merkle_tree_index_dst = usize::to_le_bytes(self.merkle_tree_index);
            *relayer_dst = self.relayer.clone().try_into().unwrap();
            *relayer_fee_dst = self.relayer_fee.clone().try_into().unwrap();
            *recipient_dst = self.recipient.clone().try_into().unwrap();
            *ext_amount_dst = self.ext_amount.clone().try_into().unwrap();
            *ext_sol_amount_dst = self.ext_sol_amount.clone().try_into().unwrap();
            *amount_dst = self.amount.clone().try_into().unwrap();
            *tx_integrity_hash_dst = self.tx_integrity_hash.clone().try_into().unwrap();
            *node_left_dst = self.node_left.clone().try_into().unwrap();
            *node_right_dst = self.node_right.clone().try_into().unwrap();
            *leaf_left_dst = self.node_left.clone().try_into().unwrap();
            *leaf_right_dst = self.node_right.clone().try_into().unwrap();
            *verifier_tmp_pda_dst = self.verifier_tmp_pda.clone().try_into().unwrap();

            *encrypted_utxos_dst = self.encrypted_utxos.clone().try_into().unwrap();
            *nullifiers_dst = self.nullifiers.clone().try_into().unwrap();
        } else if self.changed_state == 2 {
            msg!("packing state: {:?}", self.state[..32].to_vec());

            *state_dst = self.state.clone().try_into().unwrap();
            *current_round_dst = usize::to_le_bytes(self.current_round);
            *current_round_index_dst = usize::to_le_bytes(self.current_round_index);
            *current_index_dst = usize::to_le_bytes(self.current_index);
            *current_level_dst = usize::to_le_bytes(self.current_level);
            *current_level_hash_dst = self.current_level_hash.clone().try_into().unwrap();


        } else if self.changed_state == 3 {
            *found_root_dst = [self.found_root];

        } else if self.changed_state == 4 {
            *root_hash_dst = self.root_hash.clone().try_into().unwrap();
            *node_left_dst = self.node_left.clone().try_into().unwrap();
            *node_right_dst = self.node_right.clone().try_into().unwrap();
            *state_dst = self.state.clone().try_into().unwrap();
            *current_round_dst = usize::to_le_bytes(self.current_round);
            *current_round_index_dst = usize::to_le_bytes(self.current_round_index);
            *current_index_dst = usize::to_le_bytes(self.current_index);
            *current_level_dst = usize::to_le_bytes(self.current_level);
            *current_level_hash_dst = self.current_level_hash.clone().try_into().unwrap();

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
    const LEN: usize = 3900 + ENCRYPTED_UTXOS_LENGTH;
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, InstructionIndex::LEN];

        let (
            is_initialized,
            account_type,
            _unused_remainder0,
            signer_pubkey,
            _unused_remainder1,
            current_instruction_index,
            _unused_remainder2,
        ) = array_refs![input, 1, 1, 2, 32, 176, 8, 3680 + ENCRYPTED_UTXOS_LENGTH];
        msg!("is_initialized[0], {}", is_initialized[0]);
        if is_initialized[0] == 0 {
            Err(ProgramError::UninitializedAccount)
        } else {
            if account_type[0] != TMP_STORAGE_ACCOUNT_TYPE {
                msg!("Wrong account type tmp storage.");
                return Err(ProgramError::InvalidAccountData);
            }

            if IX_ORDER.len() <= usize::from_le_bytes(*current_instruction_index) {
                msg!(
                    "Computation has finished at instruction index {}.",
                    usize::from_le_bytes(*current_instruction_index)
                );
                return Err(ProgramError::InvalidAccountData);
            }

            Ok(InstructionIndex {
                is_initialized: true,
                signer_pubkey: solana_program::pubkey::Pubkey::new(signer_pubkey),
                current_instruction_index: usize::from_le_bytes(*current_instruction_index),
            })
        }
    }

    fn pack_into_slice(&self, _dst: &mut [u8]) {}
}
