//! Transfer actions for Light Token.
//!
//! These actions provide clean interfaces for transferring Light Tokens.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::Transfer as TransferInstruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for transferring Light Tokens between accounts.
///
/// # Example
/// ```ignore
/// Transfer {
///     source,
///     destination,
///     amount: 1000,
///     ..Default::default()
/// }.execute(&mut rpc, &payer, &authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct Transfer {
    /// Source token account.
    pub source: Pubkey,
    /// Destination token account.
    pub destination: Pubkey,
    /// Amount of tokens to transfer.
    pub amount: u64,
}

impl Transfer {
    /// Execute the transfer action via RPC.
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
        // Only set fee_payer if payer differs from authority
        let fee_payer = if payer.pubkey() != authority.pubkey() {
            Some(payer.pubkey())
        } else {
            None
        };

        let ix = TransferInstruction {
            source: self.source,
            destination: self.destination,
            amount: self.amount,
            authority: authority.pubkey(),
            fee_payer,
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
