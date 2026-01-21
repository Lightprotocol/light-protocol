//! Create Associated Token Account actions for Light Token.
//!
//! These actions provide clean interfaces for creating Light Token ATAs.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::{
    derive_associated_token_account, get_associated_token_address, CreateAssociatedTokenAccount,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for creating an associated token account for a Light Token mint.
///
/// # Example
/// ```ignore
/// // Non-idempotent (fails if ATA exists)
/// CreateAta {
///     mint,
///     owner,
///     idempotent: false,
/// }.execute(&mut rpc, &payer).await?;
///
/// // Idempotent (no-op if ATA exists)
/// CreateAta {
///     mint,
///     owner,
///     idempotent: true,
/// }.execute(&mut rpc, &payer).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct CreateAta {
    /// The mint public key.
    pub mint: Pubkey,
    /// The owner of the ATA.
    pub owner: Pubkey,
    /// Whether to use idempotent mode (no-op if ATA exists).
    pub idempotent: bool,
}

impl CreateAta {
    /// Execute the create_ata action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair
    ///
    /// # Returns
    /// `Result<(Signature, Pubkey), RpcError>` - The transaction signature and ATA public key
    pub async fn execute<R: Rpc>(
        self,
        rpc: &mut R,
        payer: &Keypair,
    ) -> Result<(Signature, Pubkey), RpcError> {
        let mut instruction_builder =
            CreateAssociatedTokenAccount::new(payer.pubkey(), self.owner, self.mint);

        if self.idempotent {
            instruction_builder = instruction_builder.idempotent();
        }

        let ix = instruction_builder
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

        let signature = rpc
            .create_and_send_transaction(&[ix], &payer.pubkey(), &[payer])
            .await?;

        Ok((signature, get_ata_address(&self.mint, &self.owner)))
    }
}

/// Get the associated token address for a given owner and mint.
///
/// This is a pure function that computes the ATA address without any RPC calls.
///
/// # Arguments
/// * `mint` - The mint public key
/// * `owner` - The owner public key
///
/// # Returns
/// `Pubkey` - The ATA address
pub fn get_ata_address(mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    get_associated_token_address(owner, mint)
}

/// Derive the associated token address with bump seed.
///
/// # Arguments
/// * `mint` - The mint public key
/// * `owner` - The owner public key
///
/// # Returns
/// `(Pubkey, u8)` - The ATA address and bump seed
pub fn derive_ata_address(mint: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
    derive_associated_token_account(owner, mint)
}
