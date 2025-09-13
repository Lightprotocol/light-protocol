use std::str::FromStr;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::instructions::compress_and_close::{
    CompressAndCloseAccounts as CTokenCompressAndCloseAccounts, CompressAndCloseIndices,
};
use light_registry::{
    accounts::CompressAndCloseContext as CompressAndCloseAccounts, instruction::CompressAndClose,
    utils::get_forester_epoch_pda_from_authority,
};
use light_sdk::instruction::PackedAccounts;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

/// Compress and close token accounts via the registry program
///
/// This function invokes the registry program's compress_and_close instruction,
/// which then CPIs to the compressed token program with the correct rent_authority PDA signer.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `solana_ctoken_accounts` - List of compressible token accounts to compress and close
/// * `authority` - Authority that can execute the compress and close
/// * `payer` - Transaction fee payer
///
/// # Returns
/// `Result<Signature, RpcError>` - Transaction signature
pub async fn compress_and_close_forester<R: Rpc + Indexer>(
    rpc: &mut R,
    solana_ctoken_accounts: &[Pubkey],
    authority: &Keypair,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    // Registry and compressed token program IDs
    let registry_program_id =
        Pubkey::from_str("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").unwrap();
    let compressed_token_program_id =
        Pubkey::from_str("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m").unwrap();

    let current_epoch = 0;

    // Derive registered forester PDA for the current epoch
    let (registered_forester_pda, _) =
        get_forester_epoch_pda_from_authority(&authority.pubkey(), current_epoch);

    // Derive CompressibleConfig PDA (version 1)
    let version: u64 = 1;
    let (compressible_config, _) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    // Derive rent_authority PDA (uses u16 version)
    let (rent_authority, _) = Pubkey::find_program_address(
        &[
            b"rent_authority".as_slice(),
            (version as u16).to_le_bytes().as_slice(),
            &[0],
        ],
        &registry_program_id,
    );

    // Validate input
    if solana_ctoken_accounts.is_empty() {
        return Err(RpcError::CustomError(
            "No token accounts provided".to_string(),
        ));
    }

    // Get output tree for compression
    let output_tree_info = rpc
        .get_random_state_tree_info()
        .map_err(|e| RpcError::CustomError(format!("Failed to get state tree info: {}", e)))?;
    let output_queue = output_tree_info
        .get_output_pubkey()
        .map_err(|e| RpcError::CustomError(format!("Failed to get output queue: {}", e)))?;

    // Prepare accounts using PackedAccounts
    let mut packed_accounts = PackedAccounts::default();

    // Add output queue first
    let output_tree_index = packed_accounts.insert_or_get(output_queue);

    // Parse the ctoken account to get required pubkeys
    use light_ctoken_types::state::{CompressedToken, ZExtensionStruct};
    use light_zero_copy::traits::ZeroCopyAt;

    // Process each token account and build indices
    let mut indices_vec = Vec::with_capacity(solana_ctoken_accounts.len());

    for solana_ctoken_account in solana_ctoken_accounts {
        // Get the ctoken account data
        let ctoken_solana_account = rpc
            .get_account(*solana_ctoken_account)
            .await
            .map_err(|e| {
                RpcError::CustomError(format!(
                    "Failed to get ctoken account {}: {}",
                    solana_ctoken_account, e
                ))
            })?
            .ok_or_else(|| {
                RpcError::CustomError(format!(
                    "CToken account {} not found",
                    solana_ctoken_account
                ))
            })?;

        let (ctoken_account, _) =
            CompressedToken::zero_copy_at(ctoken_solana_account.data.as_slice()).map_err(|e| {
                RpcError::CustomError(format!(
                    "Failed to parse ctoken account {}: {:?}",
                    solana_ctoken_account, e
                ))
            })?;

        // Pack the basic accounts
        let source_index = packed_accounts.insert_or_get(*solana_ctoken_account);
        let mint_index =
            packed_accounts.insert_or_get(Pubkey::from(ctoken_account.mint.to_bytes()));
        let owner_index =
            packed_accounts.insert_or_get(Pubkey::from(ctoken_account.owner.to_bytes()));

        // For registry flow: rent_authority is a PDA (not a signer in transaction)
        // Find rent_authority and rent_recipient from extension
        let mut rent_authority_pubkey = Pubkey::from(ctoken_account.owner.to_bytes());
        let mut rent_recipient_index = owner_index;

        if let Some(extensions) = &ctoken_account.extensions {
            for extension in extensions {
                if let ZExtensionStruct::Compressible(e) = extension {
                    if e.rent_authority != [0u8; 32] {
                        rent_authority_pubkey = Pubkey::from(e.rent_authority);
                    }
                    if e.rent_recipient != [0u8; 32] {
                        rent_recipient_index =
                            packed_accounts.insert_or_get(Pubkey::from(e.rent_recipient));
                    }
                    break;
                }
            }
        }

        // Add rent_authority as non-signer (registry will sign with PDA)
        let authority_index = packed_accounts.insert_or_get_config(
            rent_authority_pubkey,
            false, // is_signer = false (registry PDA will sign during CPI)
            true,  // is_writable
        );

        let indices = CompressAndCloseIndices {
            source_index,
            mint_index,
            owner_index,
            authority_index,
            rent_recipient_index,
            output_tree_index,
        };

        indices_vec.push(indices);
    }

    // Add light system program accounts
    let config = CTokenCompressAndCloseAccounts::default();
    packed_accounts
        .add_custom_system_accounts(config)
        .map_err(|e| RpcError::CustomError(format!("Failed to add system accounts: {:?}", e)))?;

    // Get account metas for remaining accounts
    let (remaining_account_metas, _, _) = packed_accounts.to_account_metas();
    // Build accounts using Anchor's account abstraction
    let compress_and_close_accounts = CompressAndCloseAccounts {
        authority: authority.pubkey(),
        registered_forester_pda,
        rent_authority,
        compressible_config,
        compressed_token_program: compressed_token_program_id,
    };

    // Get account metas from Anchor accounts
    let mut accounts = compress_and_close_accounts.to_account_metas(Some(true));

    // Add remaining accounts from packed accounts
    accounts.extend(remaining_account_metas);
    // Create Anchor instruction with proper discriminator
    let instruction = CompressAndClose {
        indices: indices_vec,
    };
    let instruction_data = instruction.data();

    // Create the instruction
    let compress_and_close_ix = Instruction {
        program_id: registry_program_id,
        accounts,
        data: instruction_data,
    };

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[compress_and_close_ix], &payer.pubkey(), &signers)
        .await
}
