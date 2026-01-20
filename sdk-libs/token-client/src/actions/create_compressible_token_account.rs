use light_client::rpc::{Rpc, RpcError};
use light_token::instruction::{CompressibleParams, CreateTokenAccount};
use light_token_interface::{has_restricted_extensions, state::TokenDataVersion};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub struct CreateCompressibleTokenAccountInputs<'a> {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub num_prepaid_epochs: u8,
    pub payer: &'a Keypair,
    pub token_account_keypair: Option<&'a Keypair>,
    pub lamports_per_write: Option<u32>,
    pub token_account_version: TokenDataVersion,
}

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

    let registry_program_id = solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
    let version: u16 = 1;
    let (compressible_config, _config_bump) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    let (rent_sponsor, _rent_sponsor_bump) = Pubkey::find_program_address(
        &[b"rent_sponsor".as_slice(), version.to_le_bytes().as_slice()],
        &solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
    );

    // Check if mint has restricted extensions that require compression_only mode
    let compression_only = match rpc.get_account(mint).await {
        Ok(Some(mint_account)) => has_restricted_extensions(&mint_account.data),
        _ => false,
    };

    let compressible_params = CompressibleParams {
        compressible_config,
        rent_sponsor,
        pre_pay_num_epochs: num_prepaid_epochs,
        lamports_per_write,
        compress_to_account_pubkey: None,
        token_account_version,
        compression_only,
    };

    let create_token_account_ix =
        CreateTokenAccount::new(payer.pubkey(), token_account_pubkey, mint, owner)
            .with_compressible(compressible_params)
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    rpc.create_and_send_transaction(
        &[create_token_account_ix],
        &payer.pubkey(),
        &[payer, token_account_keypair],
    )
    .await?;

    Ok(token_account_pubkey)
}
