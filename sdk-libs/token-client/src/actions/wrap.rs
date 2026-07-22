//! Wrap SPL tokens to Light Token action.
//!
//! Wraps SPL tokens into a Light Token account (rent-free storage).

use light_client::rpc::{Rpc, RpcError};
use light_token::{
    constants::{SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID},
    instruction::TransferFromSpl,
    spl_interface::{find_spl_interface_pda, has_restricted_extensions},
};
use solana_instruction::Instruction;
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

pub async fn create_wrap_instructions<R: Rpc>(
    rpc: &R,
    wrap: &Wrap,
    payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    // Get the source account to determine the token program
    let source_account_info = rpc
        .get_account(wrap.source_spl_ata)
        .await?
        .ok_or_else(|| RpcError::CustomError("Source SPL token account not found".to_string()))?;

    let spl_token_program = source_account_info.owner;

    // Validate that the source account is owned by a supported SPL token program
    if spl_token_program != SPL_TOKEN_PROGRAM_ID && spl_token_program != SPL_TOKEN_2022_PROGRAM_ID {
        return Err(RpcError::CustomError(format!(
            "Source SPL token account {} is owned by an unsupported program {}. \
                 Expected SPL Token ({}) or Token-2022 ({}).",
            wrap.source_spl_ata,
            source_account_info.owner,
            SPL_TOKEN_PROGRAM_ID,
            SPL_TOKEN_2022_PROGRAM_ID
        )));
    }

    // Check for restricted extensions if using Token-2022
    let restricted = if spl_token_program == SPL_TOKEN_2022_PROGRAM_ID {
        let mint_account = rpc
            .get_account(wrap.mint)
            .await?
            .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;
        has_restricted_extensions(&mint_account.data)
    } else {
        false
    };

    let (spl_interface_pda, bump) = find_spl_interface_pda(&wrap.mint, restricted);

    let ix = TransferFromSpl {
        amount: wrap.amount,
        spl_interface_pda_bump: bump,
        decimals: wrap.decimals,
        source_spl_token_account: wrap.source_spl_ata,
        destination: wrap.destination,
        authority,
        mint: wrap.mint,
        payer,
        spl_interface_pda,
        spl_token_program,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl Wrap {
    pub async fn instructions<R: Rpc>(
        &self,
        rpc: &R,
        payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_wrap_instructions(rpc, self, payer, authority).await
    }

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
        let instructions =
            create_wrap_instructions(rpc, &self, payer.pubkey(), authority.pubkey()).await?;

        let mut signers: Vec<&Keypair> = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
