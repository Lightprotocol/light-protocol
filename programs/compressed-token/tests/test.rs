#![cfg(feature = "test-sbf")]

use account_compression::{state_merkle_tree_from_bytes, StateMerkleTreeAccount};
use light_hasher::Poseidon;
use light_test_utils::{
    create_account_instruction, create_and_send_transaction,
    test_env::setup_test_programs_with_accounts,
};
use psp_compressed_pda::{event::PublicTransactionEvent, utxo::Utxo};
use psp_compressed_token::{
    get_token_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initiatialize_mint_instruction, create_mint_to_instruction},
    TokenTlvData,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::initialize_mint;
pub fn create_initialize_mint_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
) -> ([Instruction; 4], Pubkey) {
    let account_create_ix = create_account_instruction(
        &payer,
        anchor_spl::token::Mint::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let mint_authority = get_token_authority_pda(authority, &mint_pubkey);
    let create_mint_instruction = initialize_mint(
        &anchor_spl::token::ID,
        &mint_keypair.pubkey(),
        &mint_authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(&payer, &mint_pubkey, rent);

    let instruction = create_initiatialize_mint_instruction(&payer, &authority, &mint_pubkey);
    let pool_pubkey = get_token_pool_pda(&mint_pubkey);
    (
        [
            account_create_ix,
            create_mint_instruction,
            transfer_ix,
            instruction,
        ],
        pool_pubkey,
    )
}

use anchor_lang::{solana_program::program_pack::Pack, AnchorDeserialize};
async fn assert_create_mint(
    context: &mut ProgramTestContext,
    authority: &Pubkey,
    mint: &Pubkey,
    pool: &Pubkey,
) {
    let mint_account: spl_token::state::Mint = spl_token::state::Mint::unpack(
        &context
            .banks_client
            .get_account(*mint)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    let mint_authority = get_token_authority_pda(authority, mint);
    assert_eq!(mint_account.supply, 0);
    assert_eq!(mint_account.decimals, 2);
    assert_eq!(mint_account.mint_authority.unwrap(), mint_authority);
    assert_eq!(mint_account.freeze_authority, None.into());
    assert_eq!(mint_account.is_initialized, true);
    let mint_account: spl_token::state::Account = spl_token::state::Account::unpack(
        &context
            .banks_client
            .get_account(*pool)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();

    assert_eq!(mint_account.amount, 0);
    assert_eq!(mint_account.delegate, None.into());
    assert_eq!(mint_account.mint, *mint);
    assert_eq!(mint_account.owner, mint_authority);
}

#[tokio::test]
async fn test_create_mint() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::Mint::LEN);
    let mint = Keypair::new();
    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    create_and_send_transaction(&mut context, &instructions, &payer_pubkey, &[&payer, &mint])
        .await
        .unwrap();
    assert_create_mint(&mut context, &payer_pubkey, &mint.pubkey(), &pool).await;
}

async fn create_mint_helper(context: &mut ProgramTestContext, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::Mint::LEN);
    let mint = Keypair::new();

    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    create_and_send_transaction(context, &instructions, &payer_pubkey, &[&payer, &mint])
        .await
        .unwrap();
    assert_create_mint(context, &payer_pubkey, &mint.pubkey(), &pool).await;
    mint.pubkey()
}

#[tokio::test]
async fn test_mint_to() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &merkle_tree_pubkey,
        vec![amount],
        vec![recipient_keypair.pubkey()],
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree =
        state_merkle_tree_from_bytes(&old_merkle_tree_account.deserialized.state_merkle_tree);
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;

    let mut mock_indexer = MockIndexer::new(merkle_tree_pubkey, indexed_array_pubkey, payer);
    mock_indexer.add_token_utxos(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_mint_to(
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        old_merkle_tree,
    )
    .await;
}

async fn assert_mint_to(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_keypair: &Keypair,
    mint: Pubkey,
    amount: u64,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree<Poseidon, 22, 0, 2800>,
) {
    let token_utxo_data = mock_indexer.token_utxos[0].token_data.clone();
    assert_eq!(token_utxo_data.amount, amount);
    assert_eq!(token_utxo_data.owner, recipient_keypair.pubkey());
    assert_eq!(token_utxo_data.mint, mint);
    assert_eq!(token_utxo_data.delegate, None.into());
    assert_eq!(token_utxo_data.is_native, None);
    assert_eq!(token_utxo_data.close_authority, None.into());
    assert_eq!(token_utxo_data.delegated_amount, 0);

    let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        mock_indexer.merkle_tree_pubkey,
    )
    .await;
    let merkle_tree =
        state_merkle_tree_from_bytes(&merkle_tree_account.deserialized.state_merkle_tree);
    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    assert_eq!(merkle_tree.root_index(), 1);
    assert_ne!(
        old_merkle_tree.root().unwrap(),
        merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    let mint_account: spl_token::state::Mint = spl_token::state::Mint::unpack(
        &context
            .banks_client
            .get_account(mint)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(mint_account.supply, amount);

    let pool = get_token_pool_pda(&mint);
    let pool_account = spl_token::state::Account::unpack(
        &context
            .banks_client
            .get_account(pool)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(pool_account.amount, amount);
}
#[derive(Debug)]
pub struct MockIndexer {
    pub merkle_tree_pubkey: Pubkey,
    pub indexed_array_pubkey: Pubkey,
    pub payer: Keypair,
    pub utxos: Vec<Utxo>,
    pub nullified_utxos: Vec<Utxo>,
    pub token_utxos: Vec<TokenUtxo>,
    pub token_nullified_utxos: Vec<TokenUtxo>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<light_hasher::Poseidon, 22, 2800>,
}

#[derive(Debug, Clone)]
pub struct TokenUtxo {
    pub index: usize,
    pub token_data: TokenTlvData,
}

impl MockIndexer {
    fn new(merkle_tree_pubkey: Pubkey, indexed_array_pubkey: Pubkey, payer: Keypair) -> Self {
        Self {
            merkle_tree_pubkey,
            indexed_array_pubkey,
            payer,
            utxos: vec![],
            nullified_utxos: vec![],
            events: vec![],
            token_utxos: vec![],
            token_nullified_utxos: vec![],
            merkle_tree:
                light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon, 22, 2800>::new()
                    .unwrap(),
        }
    }

    /// deserializes an event
    /// adds the out_utxos to the utxos
    /// removes the in_utxos from the utxos
    /// adds the in_utxos to the nullified_utxos
    pub fn add_lamport_utxos(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_utxos(event);
    }

    pub fn add_event_and_utxos(&mut self, event: PublicTransactionEvent) -> Vec<usize> {
        let mut indices = Vec::with_capacity(event.out_utxos.len());
        for utxo in event.out_utxos.iter() {
            self.utxos.push(utxo.clone());
            indices.push(self.utxos.len() - 1);
            self.merkle_tree
                .append(&utxo.hash())
                .expect("insert failed");
        }
        for utxo in event.in_utxos.iter() {
            let index = self
                .utxos
                .iter()
                .position(|x| x == utxo)
                .expect("utxo not found");
            self.utxos.remove(index);
            // TODO: nullify utxo in Merkle tree, not implemented yet
            self.nullified_utxos.push(utxo.clone());
            let index = self.utxos.iter().position(|x| x == utxo);
            match index {
                Some(index) => {
                    let token_utxo_element = self.token_utxos[index].clone();
                    self.token_utxos.remove(index);
                    self.token_nullified_utxos.push(token_utxo_element);
                }
                None => {}
            }
        }
        self.events.push(event);
        indices
    }
    /// deserializes an event
    /// adds the out_utxos to the utxos
    /// removes the in_utxos from the utxos
    /// adds the in_utxos to the nullified_utxos
    /// deserialiazes token tlv data from the out_utxos
    /// adds the token_utxos to the token_utxos
    pub fn add_token_utxos(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        let indices = self.add_event_and_utxos(event);
        for index in indices.iter() {
            let data = self.utxos[*index].data.as_ref().unwrap();
            let token_data =
                TokenTlvData::deserialize(&mut data.tlv_elements[0].data.as_slice()).unwrap();
            self.token_utxos.push(TokenUtxo {
                index: *index,
                token_data,
            });
        }
    }
}
