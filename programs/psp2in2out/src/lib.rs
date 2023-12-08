use anchor_lang::prelude::*;
use light_macros::light_verifier_accounts;
use light_verifier_sdk::light_transaction::{
    Amounts, ProofCompressed, Transaction, TransactionInput,
};
use verifying_key::VERIFYINGKEY_TRANSACTION_MASP2_MAIN;

pub mod verifying_key;

declare_id!("J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_psp2in2out",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

#[constant]
pub const PROGRAM_ID: &str = "J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i";

#[program]
pub mod light_psp2in2out {

    use super::*;

    /// This instruction is the first step of a compressed transaction.
    /// It creates and initializes a verifier state account to save state of a verification during
    /// computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data
    /// such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic
    /// in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2
    pub fn compressed_transfer_first<'info>(
        ctx: Context<'_, '_, '_, 'info, LightInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        let inputs: InstructionDataCompressedTransferFirst =
            InstructionDataCompressedTransferFirst::try_deserialize_unchecked(
                &mut [vec![0u8; 8], inputs].concat().as_slice(),
            )?;

        let len_missing_bytes = 256 - inputs.encrypted_utxos.len();
        let mut enc_utxos = inputs.encrypted_utxos;
        enc_utxos.append(&mut vec![0u8; len_missing_bytes]);
        let proof = ProofCompressed {
            a: inputs.proof_a,
            b: inputs.proof_b,
            c: inputs.proof_c,
        };
        let public_amount = Amounts {
            sol: inputs.public_amount_sol,
            spl: inputs.public_amount_spl,
        };

        let input = TransactionInput {
            ctx: &ctx,
            message: None,
            proof: &proof,
            public_amount: &public_amount,
            nullifiers: &inputs.input_nullifier,
            leaves: &inputs.output_commitment,
            encrypted_utxos: &enc_utxos,
            merkle_root_index: inputs.root_index as usize,
            rpc_fee: inputs.rpc_fee,
            checked_public_inputs: &[],
            pool_type: &[0u8; 32],
            verifyingkey: &VERIFYINGKEY_TRANSACTION_MASP2_MAIN,
        };
        let mut transaction = Transaction::<0, 2, 2, 9, LightInstruction<'info>>::new(input);

        transaction.transact()?;

        #[cfg(all(feature = "memory-test", target_os = "solana"))]
        assert!(
            light_verifier_sdk::light_transaction::custom_heap::log_total_heap("memory_check")
                < 6000u64,
            "memory degression detected {} {}",
            light_verifier_sdk::light_transaction::custom_heap::log_total_heap("memory_check"),
            6000u64
        );
        Ok(())
    }
}

#[light_verifier_accounts(sol, spl)]
#[derive(Accounts)]
pub struct LightInstruction<'info> {}

#[derive(Debug)]
#[account]
pub struct InstructionDataCompressedTransferFirst {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    public_amount_spl: [u8; 32],
    input_nullifier: [[u8; 32]; 2],
    output_commitment: [[u8; 32]; 2],
    public_amount_sol: [u8; 32],
    root_index: u64,
    rpc_fee: u64,
    encrypted_utxos: Vec<u8>,
}

#[allow(non_camel_case_types)]
// helper struct to create anchor idl with u256 type
#[account]
pub struct u256 {
    x: [u8; 32],
}

#[account]
pub struct Utxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    verifier_address_index: u64,
    blinding: u256,
    app_data_hash: u256,
    account_compression_public_key: u256,
    account_encryption_public_key: [u8; 32],
}

#[account]
pub struct OutUtxo {
    amounts: [u64; 2],
    spl_asset_index: u64,
    blinding: u256,
    utxo_data_hash: u256,
    account_compression_public_key: u256,
    account_encryption_public_key: [u8; 32],
    is_filling_utxo: bool,
}
