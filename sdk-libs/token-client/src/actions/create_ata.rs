//! Create Associated Token Account actions for Light Token.
//!
//! These actions provide clean interfaces for creating Light Token ATAs.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::{get_associated_token_address, CreateAssociatedTokenAccount};
use solana_instruction::Instruction;
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

pub fn create_ata_instructions(
    create_ata: &CreateAta,
    fee_payer: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    let mut instruction_builder =
        CreateAssociatedTokenAccount::new(fee_payer, create_ata.owner, create_ata.mint);

    if create_ata.idempotent {
        instruction_builder = instruction_builder.idempotent();
    }

    let ix = instruction_builder
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl CreateAta {
    pub fn instructions(&self, fee_payer: Pubkey) -> Result<Vec<Instruction>, RpcError> {
        create_ata_instructions(self, fee_payer)
    }

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
        let instructions = create_ata_instructions(&self, payer.pubkey())?;

        let signature = rpc
            .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
            .await?;

        Ok((
            signature,
            get_associated_token_address(&self.owner, &self.mint),
        ))
    }
}
