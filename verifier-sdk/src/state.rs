use std::marker::PhantomData;

use anchor_lang::prelude::*;

use crate::light_transaction::Config;

/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct VerifierState10Ins<const NR_CHECKED_INPUTS: usize, T: Config> {
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
    pub checked_public_inputs: [[u8; 32]; NR_CHECKED_INPUTS],
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
    pub transaction_hash: [u8; 32],
    pub e_phantom: PhantomData<T>,
}

impl<const NR_CHECKED_INPUTS: usize, T: Config> VerifierState10Ins<NR_CHECKED_INPUTS, T> {
    pub const LEN: usize = 2048;
}

impl<const NR_CHECKED_INPUTS: usize, T: Config> anchor_lang::AccountDeserialize
    for VerifierState10Ins<NR_CHECKED_INPUTS, T>
{
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierState10Ins::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

impl<const NR_CHECKED_INPUTS: usize, T: Config> anchor_lang::AccountSerialize
    for VerifierState10Ins<NR_CHECKED_INPUTS, T>
{
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.serialize(writer).unwrap();
        match self.serialize(writer) {
            Ok(_) => Ok(()),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotSerialize),
        }
    }
}

impl<const NR_CHECKED_INPUTS: usize, T: Config> anchor_lang::Owner
    for VerifierState10Ins<NR_CHECKED_INPUTS, T>
{
    fn owner() -> Pubkey {
        T::ID
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct TransactionIndexerEvent {
    pub leaves: Vec<[u8; 32]>,
    pub public_amount_spl: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
    pub nullifiers: Vec<[u8; 32]>,
    pub first_leaf_index: u64,
    pub message: Vec<u8>,
}
