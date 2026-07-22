//! Mint tokens action for Light Token.
//!
//! Simple interface for minting tokens to a Light Token account.

use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::MintTo as MintToInstruction;
use solana_instruction::Instruction;
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

pub fn create_mint_to_instructions(
    mint_to: &MintTo,
    fee_payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    let ix = MintToInstruction {
        mint: mint_to.mint,
        destination: mint_to.destination,
        amount: mint_to.amount,
        authority,
        fee_payer,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl MintTo {
    pub fn instructions(
        &self,
        fee_payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_mint_to_instructions(self, fee_payer, authority)
    }

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
        let instructions = create_mint_to_instructions(&self, payer.pubkey(), authority.pubkey())?;

        let mut signers: Vec<&Keypair> = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
