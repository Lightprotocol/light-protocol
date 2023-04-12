use crate::light_transaction::Config;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use std::marker::PhantomData;

/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct VerifierState10Ins<T: Config, const NR_LEAVES: usize> {
    pub signer: Pubkey,
    pub nullifiers: Vec<[u8; 32]>,
    pub leaves: Vec<[u8; 32]>,
    pub public_amount_spl: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub merkle_root: [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
    pub merkle_root_index: u64,
    pub checked_public_inputs: Vec<Vec<u8>>,
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
    pub e_phantom: PhantomData<T>,
}

impl<T: Config, const NR_LEAVES: usize> VerifierState10Ins<T, NR_LEAVES> {
    pub const LEN: usize = 2048;

    pub fn new(
        nullifiers: Vec<[u8; 32]>,
        leaves: Vec<[u8; 32]>,
        public_amount_spl: [u8; 32],
        public_amount_sol: [u8; 32],
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
        merkle_root_index: usize,
        checked_public_inputs: Vec<Vec<u8>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) -> Self {
        Self {
            signer: Pubkey::default(),
            nullifiers,
            leaves,
            public_amount_spl,
            public_amount_sol,
            mint_pubkey: [0u8; 32],
            merkle_root: [0u8; 32],
            tx_integrity_hash: [0u8; 32],
            relayer_fee,
            encrypted_utxos,
            merkle_root_index: merkle_root_index as u64,
            checked_public_inputs,
            proof_a,
            proof_b,
            proof_c,
            e_phantom: PhantomData,
        }
    }

    pub fn init(
        &mut self,
        signer: Pubkey,
        nullifiers: Vec<[u8; 32]>,
        // leaves: Vec<[u8; 32]>,
        leaves: &[[[u8; 32]; 2]; NR_LEAVES],
        public_amount_spl: [u8; 32],
        public_amount_sol: [u8; 32],
        relayer_fee: u64,
        encrypted_utxos: Vec<u8>,
        merkle_root_index: usize,
        checked_public_inputs: Vec<Vec<u8>>,
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
    ) {
        self.signer = signer;
        self.nullifiers = nullifiers;
        self.leaves = leaves.iter().flat_map(|pair| [pair[0], pair[1]]).collect();
        self.public_amount_spl = public_amount_spl;
        self.public_amount_sol = public_amount_sol;
        self.mint_pubkey = [0u8; 32];
        self.merkle_root = [0u8; 32];
        self.tx_integrity_hash = [0u8; 32];
        self.relayer_fee = relayer_fee;
        self.encrypted_utxos = encrypted_utxos;
        self.merkle_root_index = merkle_root_index as u64;
        self.checked_public_inputs = checked_public_inputs;
        self.proof_a = proof_a;
        self.proof_b = proof_b;
        self.proof_c = proof_c;
    }

    /// Returns the merkle root index (as usize).
    pub fn merkle_root_index(&self) -> usize {
        self.merkle_root_index as usize
    }

    /// Returns an iterator over the pairs of leaves.
    pub fn leaves(&self) -> impl Iterator<Item = (&[u8; 32], &[u8; 32])> {
        self.leaves
            .iter()
            .step_by(2)
            .zip(self.leaves.iter().skip(1).step_by(2))
    }
}

impl<T: Config, const NR_LEAVES: usize> anchor_lang::AccountDeserialize
    for VerifierState10Ins<T, NR_LEAVES>
{
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierState10Ins::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

impl<T: Config, const NR_LEAVES: usize> anchor_lang::AccountSerialize
    for VerifierState10Ins<T, NR_LEAVES>
{
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.serialize(writer).unwrap();
        match self.serialize(writer) {
            Ok(_) => Ok(()),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotSerialize),
        }
    }
}

impl<T: Config, const NR_LEAVES: usize> anchor_lang::Owner for VerifierState10Ins<T, NR_LEAVES> {
    fn owner() -> Pubkey {
        T::ID
    }
}
