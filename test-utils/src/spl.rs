use solana_program_test::ProgramTestContext;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::state::Mint;

use crate::{create_and_send_transaction, test_env::COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID};

pub async fn create_mint(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
    freeze_authority: Option<&Pubkey>,
    mint_keypair: Option<&Keypair>,
) -> Pubkey {
    let keypair = Keypair::new();
    let mint_keypair = match mint_keypair {
        Some(mint_keypair) => mint_keypair,
        None => &keypair,
    };
    let mint_pubkey = (*mint_keypair).pubkey();
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);

    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        mint_rent,
        &COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID,
        Some(mint_keypair),
    );

    let create_mint_ix = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint_pubkey,
        mint_authority,
        freeze_authority,
        decimals,
    )
    .unwrap();
    create_and_send_transaction(
        context,
        &[account_create_ix, create_mint_ix],
        &payer.pubkey(),
        &[payer],
    )
    .await
    .unwrap();
    mint_pubkey
}
