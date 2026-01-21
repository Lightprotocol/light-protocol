//! Mint tokens action for Light Token.
//!
//! Simple interface for minting tokens to a Light Token account.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::MintTo as MintToInstruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for minting tokens to a Light Token account.
///
/// # Example
/// ```ignore
/// MintTo {
///     mint,
///     destination,
///     amount: 1000,
///     ..Default::default()
/// }.execute(&mut rpc, &payer, &mint_authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct MintTo {
    /// The mint public key.
    pub mint: Pubkey,
    /// The destination token account.
    pub destination: Pubkey,
    /// Amount of tokens to mint.
    pub amount: u64,
}

impl MintTo {
    /// Execute the mint_to action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair (also pays for rent top-ups)
    /// * `authority` - The mint authority keypair
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

        let ix = MintToInstruction {
            mint: self.mint,
            destination: self.destination,
            amount: self.amount,
            authority: authority.pubkey(),
            max_top_up: None,
            fee_payer,
        }
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

        let mut signers: Vec<&Keypair> = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
            .await
    }
}
