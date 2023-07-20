use crate::light_transaction::{Config, Transaction};
use anchor_lang::prelude::*;
use std::marker::PhantomData;

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

impl<
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        const NR_PUBLIC_INPUTS: usize,
        T: Config,
    >
    From<Transaction<'_, '_, '_, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, NR_PUBLIC_INPUTS, T>>
    for VerifierState10Ins<NR_CHECKED_INPUTS, T>
{
    fn from(
        light_tx: Transaction<
            '_,
            '_,
            '_,
            NR_CHECKED_INPUTS,
            NR_LEAVES,
            NR_NULLIFIERS,
            NR_PUBLIC_INPUTS,
            T,
        >,
    ) -> VerifierState10Ins<NR_CHECKED_INPUTS, T> {
        assert_eq!(T::NR_LEAVES / 2, light_tx.input.leaves.len());
        assert_eq!(T::NR_NULLIFIERS, light_tx.input.nullifiers.len());

        // need to remove one nested layer because serde cannot handle three layered nesting
        let mut leaves = Vec::new();
        for pair in light_tx.input.leaves.iter() {
            leaves.push(pair[0].clone());
            leaves.push(pair[1].clone());
        }

        #[allow(deprecated)]
        VerifierState10Ins {
            merkle_root_index: <usize as TryInto<u64>>::try_into(light_tx.input.merkle_root_index)
                .unwrap(),
            signer: Pubkey::new(&[0u8; 32]),
            nullifiers: light_tx.input.nullifiers.to_vec(),
            leaves,
            public_amount_spl: light_tx.input.public_amount.spl,
            public_amount_sol: light_tx.input.public_amount.sol,
            mint_pubkey: light_tx.mint_pubkey,
            relayer_fee: light_tx.input.relayer_fee,
            encrypted_utxos: light_tx.input.encrypted_utxos.to_vec(),
            proof_a: light_tx.input.proof.a,
            proof_b: light_tx.input.proof.b,
            proof_c: light_tx.input.proof.c,
            merkle_root: light_tx.merkle_root,
            tx_integrity_hash: light_tx.tx_integrity_hash,
            checked_public_inputs: *light_tx.input.checked_public_inputs,
            e_phantom: PhantomData,
        }
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
