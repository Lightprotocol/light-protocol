//! Wrap SPL tokens to Light Token action.
//!
//! Wraps SPL tokens into a Light Token account (rent-free storage).

use light_client::rpc::{Rpc, RpcError};
use light_token::{
    constants::SPL_TOKEN_PROGRAM_ID,
    instruction::{get_spl_interface_pda_and_bump, TransferFromSpl},
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for wrapping SPL tokens into a Light Token account.
///
/// This transfers tokens from an SPL token account to a Light Token account,
/// enabling rent-free storage.
///
/// # Example
/// ```ignore
/// Wrap {
///     source_spl_ata,
///     destination,
///     mint,
///     amount: 1000,
///     decimals: 9,
/// }.execute(&mut rpc, &payer, &authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct Wrap {
    /// Source SPL token account.
    pub source_spl_ata: Pubkey,
    /// Destination Light Token account.
    pub destination: Pubkey,
    /// The mint public key.
    pub mint: Pubkey,
    /// Amount of tokens to wrap.
    pub amount: u64,
    /// Token decimals.
    pub decimals: u8,
}

impl Wrap {
    /// Execute the wrap action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair
    /// * `authority` - Authority for the source SPL token account
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

        let ix = TransferFromSpl {
            amount: self.amount,
            spl_interface_pda_bump: bump,
            decimals: self.decimals,
            source_spl_token_account: self.source_spl_ata,
            destination: self.destination,
            authority: authority.pubkey(),
            mint: self.mint,
            payer: payer.pubkey(),
            spl_interface_pda,
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
