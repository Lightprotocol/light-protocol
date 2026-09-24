//! Transfer actions for Light Token.
//!
//! These actions provide clean interfaces for transferring Light Tokens.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::Transfer as TransferInstruction;
use solana_instruction::Instruction;
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

pub fn create_transfer_instructions(
    transfer: &Transfer,
    fee_payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    let ix = TransferInstruction {
        source: transfer.source,
        destination: transfer.destination,
        amount: transfer.amount,
        authority,
        fee_payer,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl Transfer {
    pub fn instructions(
        &self,
        fee_payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_transfer_instructions(self, fee_payer, authority)
    }

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
        let instructions = create_transfer_instructions(&self, payer.pubkey(), authority.pubkey())?;

        let mut signers = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
