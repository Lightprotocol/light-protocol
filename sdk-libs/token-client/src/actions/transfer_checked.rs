//! Transfer checked action for Light Token.
//!
//! This action provides a clean interface for transferring Light Tokens with decimal validation.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::TransferChecked as TransferCheckedInstruction;
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

impl TransferChecked {
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
        let ix = TransferCheckedInstruction {
            source: self.source,
            mint: self.mint,
            destination: self.destination,
            amount: self.amount,
            decimals: self.decimals,
            authority: authority.pubkey(),
            fee_payer: payer.pubkey(),
        }
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

        let mut signers = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
            .await
    }
}
