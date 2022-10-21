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
use ark_ff::{
    BigInteger256,
    FpParameters,
    PrimeField,
    BigInteger,
    bytes::{
        FromBytes,
        ToBytes
    }
};
use std::ops::Neg;
use crate::utils::to_be_64;
type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;

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
    pub encrypted_utxos0: Vec<u8>,
    pub encrypted_utxos1: Vec<u8>,
    pub encrypted_utxos2: Vec<u8>,
    pub encrypted_utxos3: Vec<u8>,
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

impl <T: TxConfig>anchor_lang::AccountSerialize for VerifierState<T> {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
        // no-op
        Ok(())
    }
}

impl <T: TxConfig>anchor_lang::Owner for VerifierState<T> {
    fn owner() -> Pubkey {
        Pubkey::new(&T::ID[..])
    }
}

/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct VerifierStateTenNF<T: TxConfig> {
    pub signer: Pubkey,
    pub nullifiers: Vec<Vec<u8>>, //[[u8;32]; 10],
    pub leaves: Vec<Vec<u8>>,
    pub public_amount: [u8; 32],
    pub fee_amount: [u8; 32],
    pub mint_pubkey: [u8;32],
    pub merkle_root: [u8; 32],
    pub tx_integrity_hash: [u8; 32],

    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
    pub merkle_root_index: u64,

    pub e_phantom: PhantomData<T>
}

impl <T: TxConfig>VerifierStateTenNF<T> {
    pub const LEN: usize = 2048;
}

impl <T: TxConfig>anchor_lang::AccountDeserialize for VerifierStateTenNF<T> {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierStateTenNF::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(e) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

// impl <T: TxConfig>anchor_lang::AccountSerialize for VerifierStateTenNF<T> {
//     fn try_serialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
//         match VerifierStateTenNF::serialize(buf) {
//             Ok(v) => Ok(v),
//             Err(e) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
//         }
//     }
// }

impl <T: TxConfig>anchor_lang::AccountSerialize for VerifierStateTenNF<T> {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.serialize(writer).unwrap();
        Ok(())
    }
}

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
        let mut leaves = vec![vec![vec![0u8;32]; 2]; 1];
        for (i, leaf) in light_tx.leaves.iter().enumerate() {
                leaves[i] = vec![leaf[0].clone().try_into().unwrap(), leaf[1].clone().try_into().unwrap()];
        }

        VerifierStateTenNF {
            merkle_root_index: <usize as TryInto::<u64>>::try_into(light_tx.merkle_root_index).unwrap(),
            signer: Pubkey::new(&[0u8;32]),
            nullifiers: light_tx.nullifiers,
            leaves: leaves[0].clone(),
            public_amount: *light_tx.public_amount,
            fee_amount: *light_tx.fee_amount,
            mint_pubkey: *light_tx.mint_pubkey,
            relayer_fee: light_tx.relayer_fee,
            encrypted_utxos: [light_tx.encrypted_utxos.clone(), vec![0u8; 256 - light_tx.encrypted_utxos.len()]].concat().try_into().unwrap(),
            merkle_root: *light_tx.merkle_root,
            tx_integrity_hash: *light_tx.tx_integrity_hash,
            e_phantom: PhantomData,
        }
    }
}

impl <'info,T: TxConfig + std::clone::Clone>VerifierStateTenNF<T> {

    pub fn into_light_transaction<'a, 'c> (
        &'a self,
        proof: Option<&'a [u8;256]>,
        accounts: Option<&'a Accounts<'info, 'a, 'c>>,//Context<'info, LightInstructionTrait<'info>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>
    ) -> LightTransaction<'info, 'a, 'c, T> {

        let mut nullifiers = Vec::<Vec<u8>>::new();
        for nf in &self.nullifiers {
            nullifiers.push(nf.to_vec());
        }

        // let mut leaves = Vec::<[[u8;32]; 2]>::new();
        // for leaves_acc in &self.leaves {
        //     leaves.push(leaves_acc);
        // }

        assert_eq!(T::NR_NULLIFIERS, self.nullifiers.len());

        let mut proof_a : Option<Vec<u8>> = None;
        let mut proof_b : Option<Vec<u8>> = None;
        let mut proof_c : Option<Vec<u8>> = None;

        match proof {
            Some(proof) => ({
                let proof_a_tmp: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&proof[0..64])[..], &[0u8][..]].concat()).unwrap();

                let mut proof_a_neg = [0u8;65];
                <G1 as ToBytes>::write(&proof_a_tmp.neg(), &mut proof_a_neg[..]).unwrap();
                proof_a = Some(proof_a_neg[..64].to_vec());
                proof_b = Some(proof[64..192].to_vec());
                proof_c = Some(proof[192..256].to_vec());
            }),
            None => (),
        }




        return LightTransaction {
            checked_public_inputs: Vec::<[u8; 32]>::new(),
            leaves: vec![self.leaves.clone()],
            nullifiers: nullifiers,
            public_amount: &self.public_amount,
            tx_integrity_hash: &self.tx_integrity_hash,
            fee_amount: &self.fee_amount,
            mint_pubkey: &self.mint_pubkey,
            relayer_fee: self.relayer_fee,
            merkle_root: &self.merkle_root,
            encrypted_utxos: self.encrypted_utxos.clone(),
            proof_a: proof_a,
            proof_b: proof_b,
            proof_c: proof_c,
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


#[test]
fn test_into() {

    // dbg!(person);


}
