#![cfg(feature = "test-sbf")]

use account_compression::{
    utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT, STATE_MERKLE_TREE_ROOTS,
    },
    StateMerkleTreeAccount,
};
use anchor_lang::AnchorSerialize;
use light_hasher::Poseidon;
use light_test_utils::{
    create_account_instruction, create_and_send_transaction,
    test_env::setup_test_programs_with_accounts, AccountZeroCopy,
};
use psp_compressed_pda::{event::PublicTransactionEvent, utils::CompressedProof, utxo::Utxo};
use psp_compressed_token::{
    get_token_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initialize_mint_instruction, create_mint_to_instruction},
    transfer_sdk, TokenTlvData, TokenTransferOutUtxo,
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

    let instruction = create_initialize_mint_instruction(&payer, &authority, &mint_pubkey);
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
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
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
        &old_merkle_tree,
    )
    .await;
}

// this test breaks the stack limit execute with:
// RUST_MIN_STACK=8388608  cargo test-sbf test_transfer
#[tokio::test]
async fn test_transfer() {
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
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;

    let mut mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
    );
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
        &old_merkle_tree,
    )
    .await;
    let transfer_recipient_keypair = Keypair::new();
    let in_utxos_tlv = mock_indexer.token_utxos[0].token_data.clone();
    let in_utxos = vec![mock_indexer.utxos[mock_indexer.token_utxos[0].index].clone()];

    let change_out_utxo = TokenTransferOutUtxo {
        amount: in_utxos_tlv.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
        index_mt_account: 0,
    };
    let transfer_recipient_out_utxo = TokenTransferOutUtxo {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        index_mt_account: 0,
    };
    let mock_proof = CompressedProof {
        a: [0u8; 32],
        b: [0u8; 64],
        c: [0u8; 32],
    };

    let instruction = transfer_sdk::create_transfer_instruction(
        &payer_pubkey,
        &recipient_keypair.pubkey(),
        &vec![merkle_tree_pubkey],   // in utxo Merkle trees
        &vec![indexed_array_pubkey], // in utxo indexed arrays
        &vec![merkle_tree_pubkey, merkle_tree_pubkey], // out utxo Merkle trees
        in_utxos.as_slice(),         // in utxos
        &vec![change_out_utxo, transfer_recipient_out_utxo],
        &vec![0u16],
        &mock_proof,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        [&payer, &recipient_keypair].as_slice(),
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;

    mock_indexer.add_token_utxos(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    assert_transfer(
        &mut context,
        &mock_indexer,
        &transfer_recipient_out_utxo,
        &change_out_utxo,
        &old_merkle_tree,
        &in_utxos,
    )
    .await;
    mock_indexer.nullify_utxos(&mut context).await;
}

async fn assert_mint_to<'a>(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_keypair: &Keypair,
    mint: Pubkey,
    amount: u64,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
) {
    let token_utxo_data = mock_indexer.token_utxos[0].token_data.clone();
    assert_eq!(token_utxo_data.amount, amount);
    assert_eq!(token_utxo_data.owner, recipient_keypair.pubkey());
    assert_eq!(token_utxo_data.mint, mint);
    assert_eq!(token_utxo_data.delegate, None.into());
    assert_eq!(token_utxo_data.is_native, None);
    assert_eq!(token_utxo_data.delegated_amount, 0);

    let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        mock_indexer.merkle_tree_pubkey,
    )
    .await;
    // let merkle_tree =
    //     state_merkle_tree_from_bytes(&merkle_tree_account.deserialized.state_merkle_tree);
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
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

async fn assert_transfer<'a>(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_out_utxo: &TokenTransferOutUtxo,
    change_out_utxo: &TokenTransferOutUtxo,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
    in_utxos: &Vec<Utxo>,
) {
    let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        mock_indexer.merkle_tree_pubkey,
    )
    .await;
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(merkle_tree.root_index(), 3);

    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    assert_ne!(
        old_merkle_tree.root().unwrap(),
        merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    let pos = mock_indexer
        .token_utxos
        .iter()
        .position(|x| x.token_data.owner == recipient_out_utxo.owner)
        .expect("transfer recipient utxo not found in mock indexer");
    let transfer_recipient_token_utxo = mock_indexer.token_utxos[pos].clone();
    assert_eq!(
        transfer_recipient_token_utxo.token_data.amount,
        recipient_out_utxo.amount
    );
    assert_eq!(
        transfer_recipient_token_utxo.token_data.mint,
        transfer_recipient_token_utxo.token_data.mint
    );
    assert_eq!(
        transfer_recipient_token_utxo.token_data.owner,
        recipient_out_utxo.owner
    );
    assert_eq!(
        transfer_recipient_token_utxo.token_data.delegate,
        None.into()
    );
    assert_eq!(transfer_recipient_token_utxo.token_data.is_native, None);
    assert_eq!(transfer_recipient_token_utxo.token_data.delegated_amount, 0);
    let transfer_recipient_utxo = mock_indexer.utxos[transfer_recipient_token_utxo.index].clone();
    assert_eq!(transfer_recipient_utxo.lamports, 0);
    assert!(transfer_recipient_utxo.data.is_some());
    assert_eq!(
        transfer_recipient_utxo
            .data
            .as_ref()
            .unwrap()
            .tlv_elements
            .len(),
        1
    );
    let mut data = Vec::new();
    transfer_recipient_token_utxo
        .token_data
        .serialize(&mut data)
        .unwrap();
    assert_eq!(
        transfer_recipient_utxo.data.as_ref().unwrap().tlv_elements[0].data,
        data
    );
    assert_eq!(
        transfer_recipient_utxo.data.as_ref().unwrap().tlv_elements[0].owner,
        psp_compressed_token::ID
    );
    assert_eq!(transfer_recipient_utxo.owner, psp_compressed_token::ID);

    let pos = mock_indexer
        .token_utxos
        .iter()
        .position(|x| {
            x.token_data.owner == change_out_utxo.owner
                && x.token_data.amount == change_out_utxo.amount
        })
        .expect("transfer recipient utxo not found in mock indexer");
    let change_token_utxo = mock_indexer.token_utxos[pos].clone();
    assert_eq!(change_token_utxo.token_data.amount, change_out_utxo.amount);
    assert_eq!(
        change_token_utxo.token_data.mint,
        transfer_recipient_token_utxo.token_data.mint
    );
    assert_eq!(change_token_utxo.token_data.owner, change_out_utxo.owner);
    assert_eq!(change_token_utxo.token_data.delegate, None.into());
    assert_eq!(change_token_utxo.token_data.is_native, None);
    assert_eq!(change_token_utxo.token_data.delegated_amount, 0);

    let change_utxo = mock_indexer.utxos[change_token_utxo.index].clone();
    assert_eq!(change_utxo.lamports, 0);
    assert!(change_utxo.data.is_some());
    assert_eq!(change_utxo.data.as_ref().unwrap().tlv_elements.len(), 1);
    let mut data = Vec::new();
    change_token_utxo.token_data.serialize(&mut data).unwrap();
    assert_eq!(
        change_utxo.data.as_ref().unwrap().tlv_elements[0].data,
        data
    );
    assert_eq!(
        change_utxo.data.as_ref().unwrap().tlv_elements[0].owner,
        psp_compressed_token::ID
    );
    assert_eq!(change_utxo.owner, psp_compressed_token::ID);

    // assert in utxos are nullified
    for utxo in in_utxos.iter() {
        let _nullified_utxo = mock_indexer
            .nullified_utxos
            .iter()
            .find(|x| *x == utxo)
            .expect("utxo not nullified");
    }
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
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<light_hasher::Poseidon>,
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
            merkle_tree: light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(
                STATE_MERKLE_TREE_HEIGHT,
                STATE_MERKLE_TREE_ROOTS,
                STATE_MERKLE_TREE_CANOPY_DEPTH,
            )
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
        let mut indices = Vec::with_capacity(event.out_utxos.len());
        for utxo in event.out_utxos.iter() {
            self.utxos.push(utxo.clone());
            indices.push(self.utxos.len() - 1);
            self.merkle_tree
                .append(&utxo.hash())
                .expect("insert failed");
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

    /// Check utxos in the queue array which are not nullified yet
    /// Iterate over these utxos and nullify them
    pub async fn nullify_utxos(&mut self, context: &mut ProgramTestContext) {
        let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
            context,
            self.indexed_array_pubkey,
        )
        .await;
        let indexed_array = array.deserialized().indexed_array;
        let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
            context,
            self.merkle_tree_pubkey,
        )
        .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut utxo_to_nullify = Vec::new();

        for (i, element) in indexed_array.iter().enumerate() {
            if element.merkle_tree_overwrite_sequence_number == 0 && element.element != [0u8; 32] {
                utxo_to_nullify.push((i, element));
            }
        }

        for (index_in_indexed_array, utxo) in utxo_to_nullify.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(&utxo.element).unwrap();
            let proof: Vec<[u8; 32]> = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index)
                .unwrap()
                .to_array::<16>()
                .unwrap()
                .to_vec();

            let instructions = [
                account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                    vec![change_log_index].as_slice(),
                    vec![(*index_in_indexed_array) as u16].as_slice(),
                    vec![0u64].as_slice(),
                    vec![proof].as_slice(),
                    &context.payer.pubkey(),
                    &self.merkle_tree_pubkey,
                    &self.indexed_array_pubkey,
                ),
            ];

            create_and_send_transaction(
                context,
                &instructions,
                &self.payer.pubkey(),
                &[&self.payer],
            )
            .await
            .unwrap();
            let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
                context,
                self.indexed_array_pubkey,
            )
            .await;
            let indexed_array = array.deserialized().indexed_array;
            assert_eq!(indexed_array[*index_in_indexed_array].element, utxo.element);
            let merkle_tree_account =
                light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
                    context,
                    self.merkle_tree_pubkey,
                )
                .await;
            assert_eq!(
                indexed_array[*index_in_indexed_array].merkle_tree_overwrite_sequence_number,
                merkle_tree_account
                    .deserialized()
                    .load_merkle_tree()
                    .unwrap()
                    .sequence_number as u64
                    + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as u64
            );
        }
    }
}
