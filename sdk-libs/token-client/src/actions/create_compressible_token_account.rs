use light_client::rpc::{Rpc, RpcError};
use light_compressed_token_sdk::instructions::{
    create_compressible_token_account as create_instruction, CreateCompressibleTokenAccount,
};
use light_ctoken_types::state::TokenDataVersion;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Input parameters for creating a compressible token account
pub struct CreateCompressibleTokenAccountInputs<'a> {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub num_prepaid_epochs: u8,
    pub payer: &'a Keypair,
    pub token_account_keypair: Option<&'a Keypair>,
    pub lamports_per_write: Option<u32>,
    pub token_account_version: TokenDataVersion,
}

/// Creates a compressible token account with a pool PDA as rent recipient
///
/// # Arguments
/// * `rpc` - The RPC client
/// * `inputs` - The input parameters for creating the token account
///
/// # Returns
/// The pubkey of the created token account
pub async fn create_compressible_token_account<R: Rpc>(
    rpc: &mut R,
    inputs: CreateCompressibleTokenAccountInputs<'_>,
) -> Result<Pubkey, RpcError> {
    let CreateCompressibleTokenAccountInputs {
        owner,
        mint,
        num_prepaid_epochs,
        payer,
        token_account_keypair,
        lamports_per_write,
        token_account_version,
    } = inputs;

    // Create or use provided token account keypair
    let token_account_keypair_owned = if token_account_keypair.is_none() {
        Some(Keypair::new())
    } else {
        None
    };

    let token_account_keypair = if let Some(keypair) = token_account_keypair {
        keypair
    } else {
        token_account_keypair_owned.as_ref().unwrap()
    };
    let token_account_pubkey = token_account_keypair.pubkey();

    // Derive the CompressibleConfig PDA (version 1)
    let registry_program_id = solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
    let version: u16 = 1;
    let (compressible_config, _config_bump) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    // Derive the rent_sponsor PDA
    let (rent_sponsor, _rent_sponsor_bump) = Pubkey::find_program_address(
        &[b"rent_sponsor".as_slice(), version.to_le_bytes().as_slice()],
        &solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
    );

    // Create the instruction
    let create_token_account_ix = create_instruction(CreateCompressibleTokenAccount {
        account_pubkey: token_account_pubkey,
        mint_pubkey: mint,
        owner_pubkey: owner,
        compressible_config,
        rent_sponsor,
        pre_pay_num_epochs: num_prepaid_epochs,
        lamports_per_write,
        payer: payer.pubkey(),
        compress_to_account_pubkey: None, // Not used for regular token account creation
        token_account_version,
    })
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    // Execute account creation
    rpc.create_and_send_transaction(
        &[create_token_account_ix],
        &payer.pubkey(),
        &[payer, token_account_keypair],
    )
    .await?;

    Ok(token_account_pubkey)
}
