//! Transfer checked action for Light Token.
//!
//! This action provides a clean interface for transferring Light Tokens with decimal validation.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::TransferChecked as TransferCheckedInstruction;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for transferring Light Tokens with decimal validation.
///
/// Unlike the basic transfer, this validates the amount against
/// the token's decimals to ensure the transfer is using the correct precision.
///
/// # Example
/// ```ignore
/// TransferChecked {
///     source,
///     mint,
///     destination,
///     amount: 1000,
///     decimals: 9,
///     ..Default::default()
/// }.execute(&mut rpc, &payer, &authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct TransferChecked {
    /// Source token account.
    pub source: Pubkey,
    /// The mint public key.
    pub mint: Pubkey,
    /// Destination token account.
    pub destination: Pubkey,
    /// Amount of tokens to transfer.
    pub amount: u64,
    /// Expected decimals for the token.
    pub decimals: u8,
}

pub fn create_transfer_checked_instructions(
    transfer: &TransferChecked,
    fee_payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    let ix = TransferCheckedInstruction {
        source: transfer.source,
        mint: transfer.mint,
        destination: transfer.destination,
        amount: transfer.amount,
        decimals: transfer.decimals,
        authority,
        fee_payer,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl TransferChecked {
    pub fn instructions(
        &self,
        fee_payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_transfer_checked_instructions(self, fee_payer, authority)
    }

    /// Execute the transfer_checked action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair (also pays for rent top-ups)
    /// * `authority` - Authority that can spend from the source account
    ///
    /// # Returns
    /// `Result<Signature, RpcError>` - The transaction signature
    pub async fn execute<R: Rpc>(
        self,
        rpc: &mut R,
        payer: &Keypair,
        authority: &Keypair,
    ) -> Result<Signature, RpcError> {
        let instructions =
            create_transfer_checked_instructions(&self, payer.pubkey(), authority.pubkey())?;

        let mut signers = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
