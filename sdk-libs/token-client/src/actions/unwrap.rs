//! Unwrap Light Token to SPL tokens action.
//!
//! Unwraps Light Token back to an SPL token account.

use light_client::rpc::{Rpc, RpcError};
use light_token::{
    constants::SPL_TOKEN_PROGRAM_ID,
    instruction::{get_spl_interface_pda_and_bump, TransferToSpl},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for unwrapping Light Token back to SPL tokens.
///
/// This transfers tokens from a Light Token account to an SPL token account.
///
/// # Example
/// ```ignore
/// Unwrap {
///     source,
///     destination_spl_ata,
///     mint,
///     amount: 1000,
///     decimals: 9,
/// }.execute(&mut rpc, &payer, &authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct Unwrap {
    /// Source Light Token account.
    pub source: Pubkey,
    /// Destination SPL token account.
    pub destination_spl_ata: Pubkey,
    /// The mint public key.
    pub mint: Pubkey,
    /// Amount of tokens to unwrap.
    pub amount: u64,
    /// Token decimals.
    pub decimals: u8,
}

impl Unwrap {
    /// Execute the unwrap action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair
    /// * `authority` - Authority for the source Light Token account
    ///
    /// # Returns
    /// `Result<Signature, RpcError>` - The transaction signature
    pub async fn execute<R: Rpc>(
        self,
        rpc: &mut R,
        payer: &Keypair,
        authority: &Keypair,
    ) -> Result<Signature, RpcError> {
        let (spl_interface_pda, bump) = get_spl_interface_pda_and_bump(&self.mint);

        let ix = TransferToSpl {
            source: self.source,
            destination_spl_token_account: self.destination_spl_ata,
            amount: self.amount,
            authority: authority.pubkey(),
            mint: self.mint,
            payer: payer.pubkey(),
            spl_interface_pda,
            spl_interface_pda_bump: bump,
            decimals: self.decimals,
            spl_token_program: SPL_TOKEN_PROGRAM_ID,
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
