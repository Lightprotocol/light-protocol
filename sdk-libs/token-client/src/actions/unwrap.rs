//! Unwrap Light Token to SPL tokens action.
//!
//! Unwraps Light Token back to an SPL token account.

use light_client::rpc::{Rpc, RpcError};
use light_token::{
    constants::{SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID},
    instruction::TransferToSpl,
    spl_interface::{find_spl_interface_pda, has_restricted_extensions},
};
use solana_instruction::Instruction;
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

pub async fn create_unwrap_instructions<R: Rpc>(
    rpc: &R,
    unwrap: &Unwrap,
    payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    // Get the destination account to determine the token program
    let destination_account_info = rpc
        .get_account(unwrap.destination_spl_ata)
        .await?
        .ok_or_else(|| {
            RpcError::CustomError("Destination SPL token account not found".to_string())
        })?;

    let spl_token_program = destination_account_info.owner;

    // Validate that the destination account is owned by a supported SPL token program
    if spl_token_program != SPL_TOKEN_PROGRAM_ID && spl_token_program != SPL_TOKEN_2022_PROGRAM_ID {
        return Err(RpcError::CustomError(format!(
            "Destination SPL token account {} is owned by an unsupported program {}. \
                 Expected SPL Token ({}) or Token-2022 ({}).",
            unwrap.destination_spl_ata,
            destination_account_info.owner,
            SPL_TOKEN_PROGRAM_ID,
            SPL_TOKEN_2022_PROGRAM_ID
        )));
    }

    // Check for restricted extensions if using Token-2022
    let restricted = if spl_token_program == SPL_TOKEN_2022_PROGRAM_ID {
        let mint_account = rpc
            .get_account(unwrap.mint)
            .await?
            .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;
        has_restricted_extensions(&mint_account.data)
    } else {
        false
    };

    let (spl_interface_pda, bump) = find_spl_interface_pda(&unwrap.mint, restricted);

    let ix = TransferToSpl {
        source: unwrap.source,
        destination_spl_token_account: unwrap.destination_spl_ata,
        amount: unwrap.amount,
        authority,
        mint: unwrap.mint,
        payer,
        spl_interface_pda,
        spl_interface_pda_bump: bump,
        decimals: unwrap.decimals,
        spl_token_program,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl Unwrap {
    pub async fn instructions<R: Rpc>(
        &self,
        rpc: &R,
        payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_unwrap_instructions(rpc, self, payer, authority).await
    }

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
        let instructions =
            create_unwrap_instructions(rpc, &self, payer.pubkey(), authority.pubkey()).await?;

        let mut signers: Vec<&Keypair> = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
