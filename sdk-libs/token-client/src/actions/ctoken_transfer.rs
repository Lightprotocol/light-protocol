use light_client::rpc::{Rpc, RpcError};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Transfer from one c-token account to another.
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
pub async fn transfer_ctoken<R: Rpc>(
    rpc: &mut R,
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let transfer_instruction =
        create_transfer_ctoken_instruction(source, destination, amount, authority.pubkey())?;

    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    rpc.create_and_send_transaction(&[transfer_instruction], &payer.pubkey(), &signers)
        .await
}

// TODO: consume the variant from compressed-token-sdk instead
/// Create a ctoken transfer instruction.
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
pub fn create_transfer_ctoken_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> Result<Instruction, RpcError> {
    let transfer_instruction = Instruction {
        program_id: light_token_sdk::token::CTOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(source, false),      // Source token account
            AccountMeta::new(destination, false), // Destination token account
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(Pubkey::default(), false),
        ],
        data: {
            // CTokenTransfer discriminator
            let mut data = vec![3u8];
            // Add SPL Token Transfer instruction data exactly like SPL does
            data.extend_from_slice(&amount.to_le_bytes()); // Amount as u64 little-endian
            data
        },
    };

    Ok(transfer_instruction)
}
