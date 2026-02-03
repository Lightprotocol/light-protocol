#![allow(dead_code)]

use light_account::derive_rent_sponsor_pda;
use light_client::interface::InitializeRentFreeConfig;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Shared test environment with initialized compression config.
pub struct TestEnv {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub program_id: Pubkey,
    pub config_pda: Pubkey,
    pub rent_sponsor: Pubkey,
}

/// Sets up a test environment with program, config, and rent sponsor initialized.
pub async fn setup_test_env() -> TestEnv {
    let program_id = Pubkey::new_from_array(pinocchio_light_program_test::ID);
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("pinocchio_light_program_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    TestEnv {
        rpc,
        payer,
        program_id,
        config_pda,
        rent_sponsor,
    }
}

/// Creates a compressed mint using the ctoken SDK.
/// Returns (mint_pda, mint_seed_keypair).
pub async fn setup_create_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, Keypair) {
    use light_token::instruction::{CreateMint, CreateMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint, mint_seed)
}

pub async fn assert_onchain_exists(rpc: &mut LightProgramTest, pda: &Pubkey, name: &str) {
    assert!(
        rpc.get_account(*pda).await.unwrap().is_some(),
        "{} account ({}) should exist on-chain",
        name,
        pda
    );
}

pub async fn assert_onchain_closed(rpc: &mut LightProgramTest, pda: &Pubkey, name: &str) {
    let acc = rpc.get_account(*pda).await.unwrap();
    assert!(
        acc.is_none() || acc.unwrap().lamports == 0,
        "{} account ({}) should be closed",
        name,
        pda
    );
}

pub async fn assert_compressed_token_exists(
    rpc: &mut LightProgramTest,
    owner: &Pubkey,
    expected_amount: u64,
    name: &str,
) {
    let accs = rpc
        .get_compressed_token_accounts_by_owner(owner, None, None)
        .await
        .unwrap()
        .value
        .items;
    assert!(
        !accs.is_empty(),
        "{} compressed token account should exist for owner {}",
        name,
        owner
    );
    assert_eq!(
        accs[0].token.amount, expected_amount,
        "{} token amount mismatch",
        name
    );
}

pub async fn assert_rent_sponsor_paid_for_accounts(
    rpc: &mut LightProgramTest,
    rent_sponsor: &Pubkey,
    rent_sponsor_balance_before: u64,
    created_accounts: &[Pubkey],
) {
    let rent_sponsor_balance_after = rpc
        .get_account(*rent_sponsor)
        .await
        .expect("get rent sponsor account")
        .map(|a| a.lamports)
        .unwrap_or(0);

    let mut total_account_lamports = 0u64;
    for account in created_accounts {
        let account_lamports = rpc
            .get_account(*account)
            .await
            .expect("get created account")
            .map(|a| a.lamports)
            .unwrap_or(0);
        total_account_lamports += account_lamports;
    }

    let rent_sponsor_paid = rent_sponsor_balance_before.saturating_sub(rent_sponsor_balance_after);

    assert!(
        rent_sponsor_paid >= total_account_lamports,
        "Rent sponsor should have paid at least {} lamports for accounts, but only paid {}. \
         Before: {}, After: {}",
        total_account_lamports,
        rent_sponsor_paid,
        rent_sponsor_balance_before,
        rent_sponsor_balance_after
    );
}

pub fn expected_compression_info(
    actual: &light_account::CompressionInfo,
) -> light_account::CompressionInfo {
    *actual
}

pub fn parse_token(data: &[u8]) -> light_token_interface::state::token::Token {
    borsh::BorshDeserialize::deserialize(&mut &data[..]).unwrap()
}

/// Build instruction data: discriminator + borsh-serialized params.
pub fn build_instruction_data<T: borsh::BorshSerialize>(disc: &[u8; 8], params: &T) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(disc);
    borsh::BorshSerialize::serialize(params, &mut data).unwrap();
    data
}
