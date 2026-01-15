use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_token_interface::instructions::mint_action::Recipient;
use light_token_sdk::compressed_token::create_compressed_mint::{
    derive_mint_compressed_address, find_mint_address,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::mint_action::{
    create_mint_action_instruction, DecompressMintParams, MintActionParams, MintActionType,
    MintToRecipient,
};

/// Executes a mint action that can perform multiple operations in a single instruction
///
/// # Arguments
/// * `rpc` - RPC client with indexer access
/// * `params` - Parameters for the mint action
/// * `authority` - Authority keypair for the mint operations
/// * `payer` - Account that pays for the transaction
/// * `mint_signer` - Optional mint signer for create_mint action only
pub async fn mint_action<R: Rpc + Indexer>(
    rpc: &mut R,
    params: MintActionParams,
    authority: &Keypair,
    payer: &Keypair,
    mint_signer: Option<&Keypair>,
) -> Result<Signature, RpcError> {
    // Validate authority matches params
    if params.authority != authority.pubkey() {
        return Err(RpcError::CustomError(
            "Authority keypair does not match params authority".to_string(),
        ));
    }

    // Create the instruction
    let instruction = create_mint_action_instruction(rpc, params).await?;

    // Determine signers based on actions
    let mut signers: Vec<&Keypair> = vec![payer];

    // Add authority if different from payer
    if payer.pubkey() != authority.pubkey() {
        signers.push(authority);
    }

    // Add mint signer if needed for create_mint or DecompressMint
    if let Some(signer) = mint_signer {
        if !signers.iter().any(|s| s.pubkey() == signer.pubkey()) {
            signers.push(signer);
        }
    }

    // Send the transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}

/// Convenience function to execute a comprehensive mint action
///
/// This function simplifies calling mint_action by handling common patterns
#[allow(clippy::too_many_arguments)]
pub async fn mint_action_comprehensive<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    authority: &Keypair,
    payer: &Keypair,
    // Whether to decompress the mint to a CMint Solana account (with rent params)
    decompress_mint: Option<DecompressMintParams>,
    // Whether to compress and close the CMint Solana account
    compress_and_close_cmint: bool,
    mint_to_recipients: Vec<Recipient>,
    mint_to_decompressed_recipients: Vec<Recipient>,
    update_mint_authority: Option<Pubkey>,
    update_freeze_authority: Option<Pubkey>,
    // Parameters for mint creation (required when creating a new mint)
    new_mint: Option<crate::instructions::mint_action::NewMint>,
) -> Result<Signature, RpcError> {
    // Derive addresses
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Build actions
    let mut actions = Vec::new();

    if !mint_to_recipients.is_empty() {
        let recipients = mint_to_recipients
            .into_iter()
            .map(|recipient| MintToRecipient {
                recipient: solana_pubkey::Pubkey::from(recipient.recipient.to_bytes()),
                amount: recipient.amount,
            })
            .collect();

        actions.push(MintActionType::MintTo {
            recipients,
            token_account_version: 2, // V2 for batched merkle trees
        });
    }

    if !mint_to_decompressed_recipients.is_empty() {
        use light_token_sdk::token::derive_token_ata;

        let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

        for recipient in mint_to_decompressed_recipients {
            let recipient_pubkey = solana_pubkey::Pubkey::from(recipient.recipient.to_bytes());
            let (ata_address, _) = derive_token_ata(&recipient_pubkey, &spl_mint_pda);

            actions.push(MintActionType::MintToCToken {
                account: ata_address,
                amount: recipient.amount,
            });
        }
    }

    if let Some(new_authority) = update_mint_authority {
        actions.push(MintActionType::UpdateMintAuthority {
            new_authority: Some(new_authority),
        });
    }

    if let Some(new_authority) = update_freeze_authority {
        actions.push(MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_authority),
        });
    }

    // Add DecompressMint action if requested
    if let Some(decompress_params) = decompress_mint {
        actions.push(MintActionType::DecompressMint {
            rent_payment: decompress_params.rent_payment,
            write_top_up: decompress_params.write_top_up,
        });
    }

    // Add CompressAndCloseCMint action if requested
    if compress_and_close_cmint {
        actions.push(MintActionType::CompressAndCloseCMint { idempotent: false });
    }

    // Determine if mint_signer is needed - matches onchain logic:
    // with_mint_signer = create_mint() only
    // DecompressMint does NOT need mint_signer - it uses compressed_mint.metadata.mint_signer
    let mint_signer = if new_mint.is_some() {
        Some(mint_seed)
    } else {
        None
    };
    let params = MintActionParams {
        compressed_mint_address,
        mint_seed: mint_seed.pubkey(),
        authority: authority.pubkey(),
        payer: payer.pubkey(),
        actions,
        new_mint,
    };
    println!("params {:?}", params);
    mint_action(rpc, params, authority, payer, mint_signer).await
}
