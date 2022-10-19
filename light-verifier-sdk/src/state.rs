use anchor_lang::prelude::*;
use crate::light_transaction::{TxConfig, LightTransaction};
use std::marker::PhantomData;
use groth16_solana::groth16::Groth16Verifyingkey;
// use merkle_tree_program::ID;
use anchor_lang::account;
use borsh::{BorshSerialize, BorshDeserialize};
// #[derive(Eq, PartialEq, Debug)]
// #[derive(Serialize, Deserialize)]
// use std::ops::Deref;
use crate::accounts::Accounts;
/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(BorshSerialize,BorshDeserialize, Clone)]
pub struct VerifierState<T: TxConfig> {
    pub signer: Pubkey,
    pub nullifiers: [[u8;32]; 4],
    pub leaves: [[u8;32]; 4],
    pub public_amount: [u8; 32],
    pub fee_amount: [u8; 32],
    pub mint_pubkey: [u8;32],
    pub additional_checked_public_inputs: [[u8;32];3],
    pub relayer_fee: u64,
    pub encrypted_utxos0: [u8; 128],
    pub encrypted_utxos1: [u8; 128],
    pub encrypted_utxos2: [u8; 128],
    pub encrypted_utxos3: [u8; 128],
    e_phantom: PhantomData<T>
}

impl <T: TxConfig>VerifierState<T> {
    pub const LEN: usize = 1024;
}
use anchor_lang::prelude::Error::AnchorError;
impl <T: TxConfig>anchor_lang::AccountDeserialize for VerifierState<T> {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierState::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(e) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

impl <T: TxConfig>anchor_lang::AccountSerialize for VerifierState<T> {}

impl <T: TxConfig>anchor_lang::Owner for VerifierState<T> {
    fn owner() -> Pubkey {
        Pubkey::new(&T::ID[..])
    }
}

/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(BorshSerialize,BorshDeserialize, Clone)]
pub struct VerifierStateTenNF<T: TxConfig> {
    pub signer: Pubkey,
    pub nullifiers: [[u8;32]; 10],
    pub leaves: [[u8;32]; 2],
    pub public_amount: [u8; 32],
    pub fee_amount: [u8; 32],
    pub mint_pubkey: [u8;32],
    pub merkle_root: [u8; 32],

    pub relayer_fee: u64,
    pub encrypted_utxos0: [u8; 128],
    pub encrypted_utxos1: [u8; 128],
    pub merkle_root_index: u64,

    pub e_phantom: PhantomData<T>
}

impl <T: TxConfig>VerifierStateTenNF<T> {
    pub const LEN: usize = 776;
}

impl <T: TxConfig>anchor_lang::AccountDeserialize for VerifierStateTenNF<T> {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierStateTenNF::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(e) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

impl <T: TxConfig>anchor_lang::AccountSerialize for VerifierStateTenNF<T> {}

impl <T: TxConfig>anchor_lang::Owner for VerifierStateTenNF<T> {
    fn owner() -> Pubkey {
        Pubkey::new(&T::ID[..])
    }
}

impl <T: TxConfig>From<LightTransaction<'_, '_, '_, T>> for VerifierStateTenNF<T> {
    fn from(light_tx: LightTransaction<'_, '_, '_, T>) -> VerifierStateTenNF<T> {
        let mut nullifiers = [[0u8;32]; 10];
        for (i, nf) in light_tx.nullifiers.iter().enumerate() {
                nullifiers[i] = nf.clone().try_into().unwrap();
        }
        let mut leaves = [[[0u8;32]; 2]; 1];
        for (i, leaf) in light_tx.leaves.iter().enumerate() {
                leaves[i] = [leaf[0].clone().try_into().unwrap(), leaf[1].clone().try_into().unwrap()];
        }

        VerifierStateTenNF {
            merkle_root_index: <usize as TryInto::<u64>>::try_into(light_tx.merkle_root_index).unwrap(),
            signer: Pubkey::new(&[0u8;32]),
            nullifiers,
            leaves: leaves[0],
            public_amount: *light_tx.public_amount,
            fee_amount: *light_tx.fee_amount,
            mint_pubkey: *light_tx.mint_pubkey,
            relayer_fee: light_tx.relayer_fee,
            encrypted_utxos0: light_tx.encrypted_utxos[0..128].try_into().unwrap(),
            encrypted_utxos1: light_tx.encrypted_utxos[128..256].try_into().unwrap(),
            merkle_root: *light_tx.merkle_root,
            e_phantom: PhantomData,
        }
    }
}

impl <'info,T: TxConfig + std::clone::Clone>VerifierStateTenNF<T> {


    // fn from_light_transaction(light_tx: LightTransaction<'_, '_, '_, T>, light_acc: &mut anchor_lang::prelude::Account<'info, VerifierStateTenNF<T>, >) {
    //     let mut nullifiers = [[0u8;32]; 10];
    //     for (i, nf) in light_tx.nullifiers.iter().enumerate() {
    //             nullifiers[i] = nf.clone().try_into().unwrap();
    //     }
    //     let mut leaves = [[[0u8;32]; 2]; 1];
    //     for (i, leaf) in light_tx.leaves.iter().enumerate() {
    //             leaves[i] = [leaf[0].clone().try_into().unwrap(), leaf[1].clone().try_into().unwrap()];
    //     }
    //
    //     VerifierStateTenNF {
    //         merkle_root_index: <usize as TryInto::<u64>>::try_into(light_tx.merkle_root_index).unwrap(),
    //         signer: Pubkey::new(&[0u8;32]),
    //         nullifiers,
    //         leaves: leaves[0],
    //         public_amount: *light_tx.public_amount,
    //         fee_amount: *light_tx.fee_amount,
    //         mint_pubkey: *light_tx.mint_pubkey,
    //         relayer_fee: light_tx.relayer_fee,
    //         encrypted_utxos0: light_tx.encrypted_utxos[0..128].try_into().unwrap(),
    //         encrypted_utxos1: light_tx.encrypted_utxos[128..256].try_into().unwrap(),
    //         merkle_root: *light_tx.merkle_root,
    //         e_phantom: PhantomData,
    //     }
    // }

    pub fn into_light_transaction<'a, 'c> (
        &'a self,
        accounts: Option<&'a Accounts<'info, 'a, 'c>>,//Context<'info, LightInstructionTrait<'info>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>
    ) -> LightTransaction<'info, 'a, 'c, T> {
        assert_eq!(T::NR_NULLIFIERS, self.nullifiers.len());

        return LightTransaction {
            ext_data_hash: &[0u8;32],
            checked_public_inputs: Vec::<[u8; 32]>::new(),
            leaves: vec![self.leaves],
            nullifiers: self.nullifiers.to_vec(),
            public_amount: &self.public_amount,
            fee_amount: &self.fee_amount,
            mint_pubkey: &self.mint_pubkey,
            relayer_fee: self.relayer_fee,
            merkle_root: &self.merkle_root,
            encrypted_utxos: [self.encrypted_utxos0.to_vec(), self.encrypted_utxos1.to_vec()].concat(),
            proof_a: [0u8;64].to_vec(),
            proof_b: [0u8;128].to_vec(),
            proof_c: [0u8; 64].to_vec(),
            merkle_root_index: self.merkle_root_index.try_into().unwrap(),
            transferred_funds: false,
            checked_tx_integrity_hash: false,
            verified_proof : false,
            inserted_leaves : false,
            inserted_nullifier : false,
            checked_root : false,
            e_phantom: PhantomData,
            verifyingkey,
            accounts
        }
    }
}

// impl <'info, 'a, 'c, T: TxConfig>From<VerifierStateTenNF<T>> for LightTransaction<'info, 'a, 'c, T> {
//     fn from(light_tx: VerifierStateTenNF<T>) -> LightTransaction<'info, 'a, 'c, T> {
//         LightTransaction {
//             nullifiers: light_tx.nullifiers.to_vec(),
//             leaves: light_tx.leaves,
//             public_amount: &light_tx.public_amount,
//             fee_amount: &light_tx.fee_amount,
//             mint_pubkey: &light_tx.mint_pubkey,
//             relayer_fee: light_tx.relayer_fee,
//             encrypted_utxos: [light_tx.encrypted_utxos0.to_vec(), light_tx.encrypted_utxos1.to_vec()].concat(),
//             e_phantom: PhantomData,
//
//         }
//     }
// }

#[test]
fn test_into() {

    // dbg!(person);


}
