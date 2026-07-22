//! Transfer interface action for Light Token.
//!
//! This action provides a clean interface for transferring tokens that auto-routes
//! based on the account types (Light or SPL).

use light_client::rpc::{Rpc, RpcError};
use light_token::{
    instruction::{SplInterface, TransferInterface as TransferInterfaceInstruction},
    spl_interface::find_spl_interface_pda_with_index,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Parameters for transferring tokens using the interface that auto-routes based on account types.
///
/// This automatically detects whether the source and destination are
/// Light token accounts or SPL token accounts and routes the transfer accordingly:
/// - Light -> Light: Direct transfer between Light accounts
/// - Light -> SPL: Decompress from Light to SPL account
/// - SPL -> Light: Compress from SPL to Light account
/// - SPL -> SPL: Pass-through to SPL token program
///
/// # Example
/// ```ignore
/// TransferInterface {
///     source,
///     mint,
///     destination,
///     amount: 1000,
///     decimals: 9,
///     ..Default::default()
/// }.execute(&mut rpc, &payer, &authority).await?;
/// ```
#[derive(Default, Clone, Debug)]
pub struct TransferInterface {
    /// Source token account.
    pub source: Pubkey,
    /// The mint public key.
    pub mint: Pubkey,
    /// Destination token account.
    pub destination: Pubkey,
    /// Amount of tokens to transfer.
    pub amount: u64,
    /// Token decimals.
    pub decimals: u8,
    /// SPL token program (spl_token::ID or spl_token_2022::ID), required for cross-interface transfers.
    pub spl_token_program: Option<Pubkey>,
    /// Whether the mint has restricted extensions (Token-2022 specific).
    pub restricted: bool,
}

pub async fn create_transfer_interface_instructions<R: Rpc>(
    rpc: &R,
    transfer: &TransferInterface,
    payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    // Fetch account info to determine owners
    let source_account = rpc.get_account(transfer.source).await?.ok_or_else(|| {
        RpcError::CustomError(format!("Source account {} not found", transfer.source))
    })?;

    let destination_account = rpc
        .get_account(transfer.destination)
        .await?
        .ok_or_else(|| {
            RpcError::CustomError(format!(
                "Destination account {} not found",
                transfer.destination
            ))
        })?;

    let source_owner = source_account.owner;
    let destination_owner = destination_account.owner;

    // Build SplInterface if needed for cross-interface transfers
    let spl_interface = if let Some(spl_program) = transfer.spl_token_program {
        let (spl_interface_pda, spl_interface_pda_bump) =
            find_spl_interface_pda_with_index(&transfer.mint, 0, transfer.restricted);
        Some(SplInterface {
            mint: transfer.mint,
            spl_token_program: spl_program,
            spl_interface_pda,
            spl_interface_pda_bump,
        })
    } else {
        None
    };

    let ix = TransferInterfaceInstruction {
        source: transfer.source,
        destination: transfer.destination,
        amount: transfer.amount,
        decimals: transfer.decimals,
        authority,
        payer,
        mint: transfer.mint,
        spl_interface,
        source_owner,
        destination_owner,
    }
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(vec![ix])
}

impl TransferInterface {
    pub async fn instructions<R: Rpc>(
        &self,
        rpc: &R,
        payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        create_transfer_interface_instructions(rpc, self, payer, authority).await
    }

    /// Execute the transfer_interface action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client
    /// * `payer` - Transaction fee payer keypair
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
        let instructions =
            create_transfer_interface_instructions(rpc, &self, payer.pubkey(), authority.pubkey())
                .await?;

        let mut signers = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await
    }
}
