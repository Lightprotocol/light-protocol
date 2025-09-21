use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address,
    mint_action::{MintActionType, MintToRecipient},
};
use light_ctoken_types::instructions::mint_action::Recipient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

use crate::instructions::mint_action::{create_mint_action_instruction, MintActionParams};

/// Executes a mint action that can perform multiple operations in a single instruction
///
/// # Arguments
/// * `rpc` - RPC client with indexer access
/// * `params` - Parameters for the mint action
/// * `authority` - Authority keypair for the mint operations
/// * `payer` - Account that pays for the transaction
/// * `mint_signer` - Optional mint signer for CreateSplMint action
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

    // Add mint signer if needed for CreateSplMint
    if let Some(signer) = mint_signer {
        if !signers.iter().any(|s| s.pubkey() == signer.pubkey()) {
            signers.push(signer);
        }
    }

    // Send the transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}

// TODO: remove
/// Convenience function to execute a comprehensive mint action
///
/// This function simplifies calling mint_action by handling common patterns
#[allow(clippy::too_many_arguments)]
pub async fn mint_action_comprehensive<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    authority: &Keypair,
    payer: &Keypair,
    mint_to_recipients: Vec<Recipient>,
    mint_to_decompressed_recipients: Vec<Recipient>,
    update_mint_authority: Option<Pubkey>,
    update_freeze_authority: Option<Pubkey>,
    // Parameters for mint creation (required if create_spl_mint is true)
    new_mint: Option<crate::instructions::mint_action::NewMint>,
) -> Result<Signature, RpcError> {
    // Derive addresses
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

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
        use light_compressed_token_sdk::instructions::{derive_ctoken_ata, find_spl_mint_address};

        let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

        for recipient in mint_to_decompressed_recipients {
            let recipient_pubkey = solana_pubkey::Pubkey::from(recipient.recipient.to_bytes());
            let (ata_address, _) = derive_ctoken_ata(&recipient_pubkey, &spl_mint_pda);

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

    // Determine if mint_signer is needed - matches onchain logic:
    // with_mint_signer = create_mint() | has_CreateSplMint_action
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

    mint_action(rpc, params, authority, payer, mint_signer).await
}
