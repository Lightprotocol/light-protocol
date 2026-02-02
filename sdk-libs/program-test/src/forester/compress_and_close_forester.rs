use light_account::PackedAccounts;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::compressed_token::CompressAndCloseAccounts as CTokenCompressAndCloseAccounts;
use light_compressible::config::CompressibleConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::registry_sdk::{
    build_compress_and_close_instruction, get_forester_epoch_pda_from_authority,
    CompressAndCloseIndices, REGISTRY_PROGRAM_ID,
};

/// Compress and close token accounts via the registry program
///
/// This function invokes the registry program's compress_and_close instruction,
/// which then CPIs to the compressed token program with the correct compression_authority PDA signer.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `solana_ctoken_accounts` - List of compressible token accounts to compress and close
/// * `authority` - Authority that can execute the compress and close
/// * `payer` - Transaction fee payer
/// * `destination` - Optional destination for compression incentive (defaults to payer)
///
/// # Returns
/// `Result<Signature, RpcError>` - Transaction signature
pub async fn compress_and_close_forester<R: Rpc + Indexer>(
    rpc: &mut R,
    solana_ctoken_accounts: &[Pubkey],
    authority: &Keypair,
    payer: &Keypair,
    destination: Option<Pubkey>,
) -> Result<Signature, RpcError> {
    // Compressed token program ID
    let compressed_token_program_id =
        Pubkey::from_str_const("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

    let current_epoch = 0;

    // Derive registered forester PDA for the current epoch
    let (registered_forester_pda, _) =
        get_forester_epoch_pda_from_authority(&authority.pubkey(), current_epoch);

    let config = CompressibleConfig::light_token_v1(Pubkey::default(), Pubkey::default());

    let compressible_config = CompressibleConfig::derive_v1_config_pda(&REGISTRY_PROGRAM_ID).0;

    // Derive compression_authority PDA (uses u16 version)
    let compression_authority = config.compression_authority;
    println!("config compression_authority {:?}", compression_authority);

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
    packed_accounts.insert_or_get(output_queue);

    // Parse the ctoken account to get required pubkeys
    use light_token_interface::state::Token;
    use light_zero_copy::traits::ZeroCopyAt;

    let mut indices_vec = Vec::with_capacity(solana_ctoken_accounts.len());

    let mut compression_authority_pubkey: Option<Pubkey> = None;

    for solana_ctoken_account_pubkey in solana_ctoken_accounts {
        // Get the ctoken account data
        let ctoken_solana_account = rpc
            .get_account(*solana_ctoken_account_pubkey)
            .await
            .map_err(|e| {
                RpcError::CustomError(format!(
                    "Failed to get ctoken account {}: {}",
                    solana_ctoken_account_pubkey, e
                ))
            })?
            .ok_or_else(|| {
                RpcError::CustomError(format!(
                    "Light Token account {} not found",
                    solana_ctoken_account_pubkey
                ))
            })?;

        let (ctoken_account, _) = Token::zero_copy_at(ctoken_solana_account.data.as_slice())
            .map_err(|e| {
                RpcError::CustomError(format!(
                    "Failed to parse ctoken account {}: {:?}",
                    solana_ctoken_account_pubkey, e
                ))
            })?;

        // Pack the basic accounts
        let source_index = packed_accounts.insert_or_get(*solana_ctoken_account_pubkey);
        let mint_index =
            packed_accounts.insert_or_get(Pubkey::from(ctoken_account.mint.to_bytes()));

        // Get compression info from Compressible extension
        let compressible_ext = ctoken_account.get_compressible_extension().ok_or_else(|| {
            RpcError::CustomError(
                "Missing Compressible extension on Light Token account".to_string(),
            )
        })?;
        let current_authority = Pubkey::from(compressible_ext.info.compression_authority);
        let rent_sponsor_pubkey = Pubkey::from(compressible_ext.info.rent_sponsor);

        if compression_authority_pubkey.is_none() {
            compression_authority_pubkey = Some(current_authority);
        }

        let compressed_token_owner = if compressible_ext.info.compress_to_pubkey == 1 {
            *solana_ctoken_account_pubkey
        } else {
            Pubkey::from(ctoken_account.owner.to_bytes())
        };

        let owner_index = packed_accounts.insert_or_get(compressed_token_owner);
        let rent_sponsor_index = packed_accounts.insert_or_get(rent_sponsor_pubkey);

        // Get delegate if present
        let delegate_index = if ctoken_account.delegate != [0u8; 32] {
            let delegate_pubkey = Pubkey::from(ctoken_account.delegate);
            packed_accounts.insert_or_get(delegate_pubkey)
        } else {
            0 // 0 means no delegate
        };

        let indices = CompressAndCloseIndices {
            source_index,
            mint_index,
            owner_index,
            rent_sponsor_index,
            delegate_index,
        };

        indices_vec.push(indices);
    }

    let destination_pubkey = destination.unwrap_or_else(|| payer.pubkey());
    let destination_index = packed_accounts.insert_or_get_config(destination_pubkey, false, true);

    let compression_authority_pubkey = compression_authority_pubkey.ok_or_else(|| {
        RpcError::CustomError("No compression authority found in accounts".to_string())
    })?;

    let authority_index =
        packed_accounts.insert_or_get_config(compression_authority_pubkey, false, true);

    let ctoken_config = CTokenCompressAndCloseAccounts {
        compressed_token_program: compressed_token_program_id,
        cpi_authority_pda: Pubkey::find_program_address(
            &[b"cpi_authority"],
            &compressed_token_program_id,
        )
        .0,
        cpi_context: None,
        self_program: None, // Critical: None means no light_system_cpi_authority is added
    };
    packed_accounts
        .add_custom_system_accounts(ctoken_config)
        .map_err(|e| RpcError::CustomError(format!("Failed to add system accounts: {:?}", e)))?;

    // Get account metas for remaining accounts
    let (remaining_account_metas, _, _) = packed_accounts.to_account_metas();

    // Build the compress_and_close instruction using local SDK
    let compress_and_close_ix = build_compress_and_close_instruction(
        authority.pubkey(),
        registered_forester_pda,
        compression_authority,
        compressible_config,
        authority_index,
        destination_index,
        indices_vec,
        remaining_account_metas,
    );

    // Prepare signers
    let mut signers = vec![payer];
    if authority.pubkey() != payer.pubkey() {
        signers.push(authority);
    }

    // Send transaction
    rpc.create_and_send_transaction(&[compress_and_close_ix], &payer.pubkey(), &signers)
        .await
}
