use light_client::rpc::{Rpc, RpcError};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Transfer SPL tokens between decompressed compressed token accounts (accounts with compressible extensions).
/// This performs a regular SPL token transfer on accounts that were decompressed from compressed tokens.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `source` - Source token account (decompressed compressed token account)
/// * `destination` - Destination token account
/// * `amount` - Amount of tokens to transfer
/// * `authority` - Authority that can spend from the source token account
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// `Result<Signature, RpcError>` - The transaction signature
pub async fn ctoken_transfer<R: Rpc>(
    rpc: &mut R,
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let transfer_instruction =
        create_ctoken_transfer_instruction(source, destination, amount, authority.pubkey())?;

    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[transfer_instruction], &payer.pubkey(), &signers)
        .await
}

/// Create a decompressed token transfer instruction.
/// This creates an instruction that uses discriminator 3 (CTokenTransfer) to perform
/// SPL token transfers on decompressed compressed token accounts.
///
/// # Arguments
/// * `source` - Source token account
/// * `destination` - Destination token account
/// * `amount` - Amount to transfer
/// * `authority` - Authority pubkey
///
/// # Returns
/// `Result<Instruction, RpcError>`
#[allow(clippy::result_large_err)]
pub fn create_ctoken_transfer_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> Result<Instruction, RpcError> {
    let transfer_instruction = Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            AccountMeta::new(source, false),      // Source token account
            AccountMeta::new(destination, false), // Destination token account
            AccountMeta::new(authority, true), // Owner/Authority (signer, writable for lamport transfers)
            AccountMeta::new_readonly(Pubkey::default(), false), // System program for CPI transfers
        ],
        data: {
            let mut data = vec![3u8]; // CTokenTransfer discriminator
                                      // Add SPL Token Transfer instruction data exactly like SPL does
            data.extend_from_slice(&amount.to_le_bytes()); // Amount as u64 little-endian
            data
        },
    };

    Ok(transfer_instruction)
}
