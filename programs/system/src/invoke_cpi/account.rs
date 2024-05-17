use crate::InstructionDataInvokeCpi;
use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
// Security:
// - checking the slot is not enough there can be multiple transactions in the
//   same slot
// - the CpiContextAccount must be derived from the first Merkle tree account as
//   the current transaction
// - to check that all data in the CpiSignature account is from the same
//   transaction we compare the proof bytes
// - I need to guaratee that all the data in the cpi signature account is from
//   the same transaction
//   - if we just overwrite the data in the account if the proof is different we
//     cannot be sure because the program could be malicious
//   - wouldn't the same proofs be enough, if you overwrite something then I
//     discard everything that is in the account -> these utxos will not be
//     spent
//   - check ownership before or after? before we need to check who invoked the
//     program
//   - we need a transaction hash that hashes the complete instruction data,
//     this will be a pain to produce offchain Sha256(proof,
//     input_account_hashes, output_account_hashes, relay_fee,
//     compression_lamports)
//   - the last tx passes the hash and tries to recalculate the hash
/// collects invocations without proofs
/// invocations are collected and processed when an invocation with a proof is received
#[aligned_sized(anchor)]
#[derive(Debug, PartialEq, Default)]
#[account]
pub struct CpiContextAccount {
    pub associated_merkle_tree: Pubkey,
    pub context: Vec<InstructionDataInvokeCpi>,
    pub network_fee: u64,
}

impl CpiContextAccount {
    pub fn init(&mut self, associated_merkle_tree: Pubkey, network_fee: u64) {
        self.associated_merkle_tree = associated_merkle_tree;
        self.context = Vec::new();
        self.network_fee = network_fee;
    }
}
