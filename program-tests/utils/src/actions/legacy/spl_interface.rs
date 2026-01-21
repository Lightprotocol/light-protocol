//! SPL interface PDA actions for Light Protocol.
//!
//! This module provides actions for working with SPL interface PDAs (token pools).

use light_client::rpc::{Rpc, RpcError};
use light_compressed_token_sdk::spl_interface::{find_spl_interface_pda, CreateSplInterfacePda};
use light_token_interface::has_restricted_extensions;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Check if a mint has restricted extensions that require a restricted pool derivation.
///
/// Restricted extensions include: Pausable, PermanentDelegate, TransferFeeConfig, TransferHook.
/// These extensions require using a different pool PDA derivation path.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The mint public key to check
///
/// # Returns
/// * `Ok(true)` if the mint is a Token-2022 mint with restricted extensions
/// * `Ok(false)` if the mint is not Token-2022 or has no restricted extensions
/// * `Err` if the mint account could not be fetched
pub async fn is_mint_restricted<R: Rpc>(rpc: &mut R, mint: &Pubkey) -> Result<bool, RpcError> {
    let mint_account = rpc
        .get_account(*mint)
        .await?
        .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;

    // Return early if not a Token-2022 mint
    if mint_account.owner != spl_token_2022::ID {
        return Ok(false);
    }

    Ok(has_restricted_extensions(&mint_account.data))
}

/// Create an SPL interface PDA (token pool) for a mint.
///
/// This action automatically determines if the mint has restricted extensions
/// and uses the appropriate pool derivation path.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The mint public key
/// * `payer` - Transaction fee payer keypair
///
/// # Returns
/// * `Ok(Signature)` - The transaction signature
/// * `Err` - If the transaction failed
pub async fn create_spl_interface_pda<R: Rpc>(
    rpc: &mut R,
    mint: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let mint_account = rpc
        .get_account(mint)
        .await?
        .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;

    let token_program = mint_account.owner;

    // Check if restricted - only for Token-2022 mints
    let restricted = if token_program == spl_token_2022::ID {
        has_restricted_extensions(&mint_account.data)
    } else {
        false
    };

    let ix =
        CreateSplInterfacePda::new(payer.pubkey(), mint, token_program, restricted).instruction();

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[payer])
        .await
}

/// Get the SPL interface PDA address for a mint, automatically detecting if restricted.
///
/// # Arguments
/// * `rpc` - RPC client
/// * `mint` - The mint public key
///
/// # Returns
/// * `Ok((Pubkey, u8, bool))` - The PDA address, bump, and whether it's restricted
/// * `Err` - If the mint account could not be fetched
pub async fn get_spl_interface_pda_for_mint<R: Rpc>(
    rpc: &mut R,
    mint: &Pubkey,
) -> Result<(Pubkey, u8, bool), RpcError> {
    let restricted = is_mint_restricted(rpc, mint).await?;
    let (pda, bump) = find_spl_interface_pda(mint, restricted);
    Ok((pda, bump, restricted))
}
