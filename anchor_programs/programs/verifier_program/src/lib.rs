/*
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_verifier_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}
*/

pub mod errors;
pub mod light_verifier_sdk;
pub mod config;
pub mod verification_key;
pub mod processor;

pub use processor::*;
pub use config::*;
pub use light_verifier_sdk::*;
pub use verification_key::*;

pub use errors::*;

use anchor_lang::prelude::*;
use crate::processor::{
    ShieldedTransfer2Inputs,
    process_shielded_transfer_2_inputs
};

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[program]
pub mod verifier_program {
    use super::*;

    /// This instruction is the first step of a shieled transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification.
    pub fn shielded_transfer_inputs<'a, 'b, 'c, 'info> (
        ctx: Context<'a, 'b, 'c, 'info, ShieldedTransfer2Inputs<'info>>,
        proof: [u8; 256],
        merkle_root: [u8; 32],
        amount: [u8; 8],
        ext_data_hash: [u8; 32],
        nullifiers: [[u8; 32]; 2],
        leaves: [[u8; 32]; 2],
        fee_amount: [u8; 8],
        mint_pubkey: [u8;32],
        merkle_tree_index: u64,
        root_index: u64,
        relayer_fee: u64,
        encrypted_utxos0: [u8; 128],
        encrypted_utxos1: [u8; 64],
        encrypted_utxos2: [u8; 32],
        encrypted_utxos3: [u8; 14],
    ) -> Result<()> {
        process_shielded_transfer_2_inputs(
            ctx,
            proof,
            merkle_root,
            [vec![0u8;24], amount.to_vec()].concat().try_into().unwrap(),
            ext_data_hash,
            nullifiers[0],
            nullifiers[1],
            leaves[0],
            leaves[1],
            0,//ext_amount,
            [vec![0u8;24], fee_amount.to_vec()].concat().try_into().unwrap(),
            mint_pubkey,
            [encrypted_utxos0.to_vec(), encrypted_utxos1.to_vec(), encrypted_utxos2.to_vec(), encrypted_utxos3.to_vec()].concat(),
            merkle_tree_index,
            relayer_fee
        )
    }

}
