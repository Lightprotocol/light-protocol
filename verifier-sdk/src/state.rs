use std::{marker::PhantomData, mem};

use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;

use crate::light_transaction::Config;

/// Verifier state is a boiler plate struct which should be versatile enough to serve many use cases.
/// For specialized use cases with less
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug)]
#[aligned_sized(anchor)]
pub struct VerifierState10Ins<
    const NR_CHECKED_INPUTS: usize,
    const NR_LEAVES: usize,
    const NR_NULLIFIERS: usize,
    T: Config,
> {
    pub signer: Pubkey,
    // TODO(vadorovsky): Use an array.
    #[size = NR_NULLIFIERS * mem::size_of::<[u8; 32]>()]
    pub nullifiers: Vec<[u8; 32]>,
    // TODO(vadorovsky): Use an array.
    #[size = NR_LEAVES * mem::size_of::<[u8; 32]>()]
    pub leaves: Vec<[u8; 32]>,
    pub public_amount_spl: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub merkle_root: [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    pub relayer_fee: u64,
    // TODO(vadorovsky): Use an array.
    // NOTE(vadorovsky): We are probably facing some anchor/borsh/bytemuch bug
    // here. We are always passing no more than 512 bytes, but for some reason,
    // allocating less than 1.5kb results in "cannot deserialize account"
    // errors.
    #[size = 1536 * mem::size_of::<u8>()]
    pub encrypted_utxos: Vec<u8>,
    pub merkle_root_index: u64,
    pub checked_public_inputs: [[u8; 32]; NR_CHECKED_INPUTS],
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
    pub transaction_hash: [u8; 32],
    pub e_phantom: PhantomData<T>,
}

impl<
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        T: Config,
    > anchor_lang::AccountDeserialize
    for VerifierState10Ins<NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, T>
{
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        match VerifierState10Ins::deserialize(buf) {
            Ok(v) => Ok(v),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotDeserialize),
        }
    }
}

impl<
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        T: Config,
    > anchor_lang::AccountSerialize
    for VerifierState10Ins<NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, T>
{
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.serialize(writer).unwrap();
        match self.serialize(writer) {
            Ok(_) => Ok(()),
            Err(_) => err!(anchor_lang::error::ErrorCode::AccountDidNotSerialize),
        }
    }
}

impl<
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        T: Config,
    > anchor_lang::Owner for VerifierState10Ins<NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, T>
{
    fn owner() -> Pubkey {
        T::ID
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Debug)]
pub struct TransactionIndexerEvent<'a> {
    pub leaves: &'a Vec<[u8; 32]>,
    pub public_amount_spl: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub relayer_fee: u64,
    pub encrypted_utxos: Vec<u8>,
    pub nullifiers: Vec<[u8; 32]>,
    pub first_leaf_index: u64,
    pub message: Vec<u8>,
}
