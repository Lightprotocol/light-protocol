use light_client::rpc::{Rpc, RpcError};
use light_compressed_token_sdk::instructions::{
    create_compressible_token_account as create_instruction, CreateCompressibleTokenAccount,
};
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Creates a compressible token account with a pool PDA as rent recipient
///
/// # Arguments
/// * `rpc` - The RPC client
/// * `rent_authority` - The rent authority pubkey
/// * `owner` - The owner of the token account
/// * `mint` - The mint for the token account
/// * `num_prepaid_epochs` - Number of epochs to prepay rent for
/// * `payer` - The payer keypair for the transaction
/// * `token_account_keypair` - Optional token account keypair. If None, a new one will be created
/// * `write_top_up_lamports` - Optional additional lamports for write operations
///
/// # Returns
/// The pubkey of the created token account
#[allow(clippy::too_many_arguments)]
pub async fn create_compressible_token_account<R: Rpc>(
    rpc: &mut R,
    rent_authority: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    num_prepaid_epochs: u64,
    payer: &Keypair,
    token_account_keypair: Option<&Keypair>,
    write_top_up_lamports: Option<u32>,
) -> Result<Pubkey, RpcError> {
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

    // Derive pool PDA (both for rent recipient and payer)
    let (pool_pda, pool_pda_bump) = Pubkey::find_program_address(
        &[b"pool", rent_authority.as_ref()],
        &Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
    );
    let payer_pda_bump = pool_pda_bump; // Same PDA is used for both

    // Create the instruction
    let create_token_account_ix = create_instruction(CreateCompressibleTokenAccount {
        account_pubkey: token_account_pubkey,
        mint_pubkey: mint,
        owner_pubkey: owner,
        rent_authority,
        rent_recipient: pool_pda,
        pre_pay_num_epochs: num_prepaid_epochs,
        write_top_up_lamports,
        payer_pda_bump,
        payer: payer.pubkey(),
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
