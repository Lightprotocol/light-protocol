//! Revoke delegation action for Light Token.
//!
//! Simple interface for revoking a delegate on a Light Token account.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::Revoke as RevokeInstruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for revoking delegation for a Light Token account.
///
/// If `owner` is `Some`, the owner keypair will be used as the signer.
/// If `owner` is `None`, the payer will be used as the owner.
///
/// # Example
/// ```ignore
/// // Payer is the owner
/// Revoke {
///     token_account,
///     owner: None,
/// }.execute(&mut rpc, &payer).await?;
///
/// // Separate owner
/// Revoke {
///     token_account,
///     owner: Some(owner_pubkey),
/// }.execute_with_owner(&mut rpc, &payer, &owner_keypair).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct Revoke {
    /// The token account to revoke delegation for.
    pub token_account: Pubkey,
    /// Optional owner public key (for separate owner scenario).
    /// If None, the payer is used as the owner.
    pub owner: Option<Pubkey>,
}

impl Revoke {
    /// Execute the revoke action via RPC where payer is the owner.
    ///
    /// This method only supports cases where `owner == payer`. If you need a
    /// separate owner and payer, use [`execute_with_owner`](Self::execute_with_owner).
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair (must also be the owner)
    ///
    /// # Returns
    /// `Result<Signature, RpcError>` - The transaction signature
    ///
    /// # Errors
    /// Returns an error if `self.owner` is `Some` and does not equal `payer.pubkey()`.
    pub async fn execute<R: Rpc>(
        self,
        rpc: &mut R,
        payer: &Keypair,
    ) -> Result<Signature, RpcError> {
        let owner_pubkey = self.owner.unwrap_or_else(|| payer.pubkey());

        if owner_pubkey != payer.pubkey() {
            return Err(RpcError::CustomError(
                "owner does not match payer; use execute_with_owner for separate owner/payer"
                    .to_string(),
            ));
        }

        let ix = RevokeInstruction {
            token_account: self.token_account,
            owner: owner_pubkey,
            fee_payer: payer.pubkey(),
        }
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

        rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[payer])
            .await
    }

    /// Execute the revoke action via RPC with a separate owner.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair
    /// * `owner` - The owner of the token account (signer)
    ///
    /// # Returns
    /// `Result<Signature, RpcError>` - The transaction signature
    ///
    /// # Errors
    /// Returns an error if `self.owner` is `Some` and does not equal `owner.pubkey()`.
    pub async fn execute_with_owner<R: Rpc>(
        self,
        rpc: &mut R,
        payer: &Keypair,
        owner: &Keypair,
    ) -> Result<Signature, RpcError> {
        // Guard: if self.owner is set, it must match the provided owner keypair
        if let Some(expected_owner) = self.owner {
            if expected_owner != owner.pubkey() {
                return Err(RpcError::CustomError(format!(
                    "owner mismatch: self.owner ({}) does not match owner.pubkey() ({})",
                    expected_owner,
                    owner.pubkey()
                )));
            }
        }

        let ix = RevokeInstruction {
            token_account: self.token_account,
            owner: owner.pubkey(),
            fee_payer: payer.pubkey(),
        }
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

        let mut signers: Vec<&Keypair> = vec![payer];
        if owner.pubkey() != payer.pubkey() {
            signers.push(owner);
        }

        rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &signers)
            .await
    }
}
