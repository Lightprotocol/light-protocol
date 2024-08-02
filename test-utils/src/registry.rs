use crate::address_merkle_tree_config::{get_address_bundle_config, get_state_bundle_config};
use crate::indexer::{
    AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts, TokenDataWithContext,
};
use crate::rpc::rpc_connection::RpcConnection;
use crate::FetchedAccount;
use crate::{create_account_instruction, rpc::errors::RpcError};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, QueueAccount,
    StateMerkleTreeConfig,
};

use anchor_lang::AnchorDeserialize;
use light_compressed_token::TokenData;
use light_hasher::{DataHasher, Poseidon};
use light_registry::account_compression_cpi::sdk::{
    create_rollover_state_merkle_tree_instruction, CreateRolloverMerkleTreeInstructionInputs,
};
use light_registry::delegate::delegate_account::DelegateAccount;
use light_registry::delegate::get_escrow_token_authority;
use light_registry::delegate::process_deposit::DelegateAccountWithContext;
use light_registry::epoch::claim_forester::{
    CompressedForesterEpochAccount, CompressedForesterEpochAccountInput,
};
use light_registry::protocol_config::state::ProtocolConfigPda;
use light_registry::sdk::{
    create_delegate_instruction, create_deposit_instruction, create_forester_claim_instruction,
    create_register_forester_instruction, create_sync_delegate_instruction,
    create_update_forester_pda_instruction, CreateDelegateInstructionInputs,
    CreateDepositInstructionInputs, CreateSyncDelegateInstructionInputs,
};
use light_registry::utils::{
    get_forester_epoch_pda_address, get_forester_pda_address, get_forester_token_pool_pda,
    get_protocol_config_pda_address,
};
use light_registry::{protocol_config, ForesterAccount, ForesterConfig, ForesterEpochPda};
use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_system_program::sdk::event::PublicTransactionEvent;
use solana_sdk::account::Account;
use solana_sdk::program_pack::Pack;
use solana_sdk::signature::Signature;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Creates and asserts forester account creation.
pub async fn register_test_forester<R: RpcConnection>(
    rpc: &mut R,
    governance_authority: &Keypair,
    forester_authority: &Keypair,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let ix = create_register_forester_instruction(
        &governance_authority.pubkey(),
        &forester_authority.pubkey(),
        config,
    );
    rpc.create_and_send_transaction(
        &[ix],
        &governance_authority.pubkey(),
        &[governance_authority, forester_authority],
    )
    .await?;
    assert_registered_forester(
        rpc,
        &forester_authority.pubkey(),
        ForesterAccount {
            authority: forester_authority.pubkey(),
            config,
            active_stake_weight: 0,
            ..Default::default()
        },
    )
    .await
}

pub async fn update_test_forester<R: RpcConnection>(
    rpc: &mut R,
    forester_authority: &Keypair,
    new_forester_authority: Option<&Keypair>,
    config: ForesterConfig,
) -> Result<(), RpcError> {
    let mut pre_account_state = rpc
        .get_anchor_account::<ForesterAccount>(
            &get_forester_pda_address(&forester_authority.pubkey()).0,
        )
        .await?
        .unwrap();
    let (signers, new_forester_authority) = if let Some(new_authority) = new_forester_authority {
        pre_account_state.authority = new_authority.pubkey();

        (
            vec![forester_authority, &new_authority],
            Some(new_authority.pubkey()),
        )
    } else {
        (vec![forester_authority], None)
    };
    let ix = create_update_forester_pda_instruction(
        &forester_authority.pubkey(),
        new_forester_authority,
        config,
    );

    rpc.create_and_send_transaction(&[ix], &forester_authority.pubkey(), &signers)
        .await?;

    pre_account_state.config = config;
    assert_registered_forester(rpc, &forester_authority.pubkey(), pre_account_state).await
}

pub async fn assert_registered_forester<R: RpcConnection>(
    rpc: &mut R,
    forester: &Pubkey,
    expected_account: ForesterAccount,
) -> Result<(), RpcError> {
    let pda = get_forester_pda_address(forester).0;
    let account_data = rpc
        .get_anchor_account::<ForesterAccount>(&pda)
        .await?
        .unwrap();
    if account_data != expected_account {
        return Err(RpcError::AssertRpcError(format!(
            "Expected account data: {:?}, got: {:?}",
            expected_account, account_data
        )));
    }
    Ok(())
}

pub struct RentExemption {
    pub size: usize,
    pub lamports: u64,
}

pub async fn get_rent_exemption_for_address_merkle_tree_and_queue<R: RpcConnection>(
    rpc: &mut R,
    address_merkle_tree_config: &AddressMerkleTreeConfig,
    address_queue_config: &AddressQueueConfig,
) -> (RentExemption, RentExemption) {
    let queue_size = QueueAccount::size(address_queue_config.capacity as usize).unwrap();

    let queue_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(queue_size)
        .await
        .unwrap();
    let tree_size = account_compression::state::AddressMerkleTreeAccount::size(
        address_merkle_tree_config.height as usize,
        address_merkle_tree_config.changelog_size as usize,
        address_merkle_tree_config.roots_size as usize,
        address_merkle_tree_config.canopy_depth as usize,
        address_merkle_tree_config.address_changelog_size as usize,
    );
    let merkle_tree_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(tree_size)
        .await
        .unwrap();
    (
        RentExemption {
            lamports: merkle_tree_rent_exempt_lamports,
            size: tree_size,
        },
        RentExemption {
            lamports: queue_rent_exempt_lamports,
            size: queue_size,
        },
    )
}

pub async fn get_rent_exemption_for_state_merkle_tree_and_queue<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) -> (RentExemption, RentExemption) {
    let queue_size = QueueAccount::size(queue_config.capacity as usize).unwrap();

    let queue_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(queue_size)
        .await
        .unwrap();
    let tree_size = account_compression::state::StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    let merkle_tree_rent_exempt_lamports = rpc
        .get_minimum_balance_for_rent_exemption(tree_size)
        .await
        .unwrap();
    (
        RentExemption {
            lamports: merkle_tree_rent_exempt_lamports,
            size: tree_size,
        },
        RentExemption {
            lamports: queue_rent_exempt_lamports,
            size: queue_size,
        },
    )
}

pub async fn create_rollover_address_merkle_tree_instructions<R: RpcConnection>(
    rpc: &mut R,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
) -> Vec<Instruction> {
    let (merkle_tree_config, queue_config) = get_address_bundle_config(
        rpc,
        AddressMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            queue: *nullifier_queue_pubkey,
        },
    )
    .await;
    let (merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_address_merkle_tree_and_queue(
            rpc,
            &merkle_tree_config,
            &queue_config,
        )
        .await;
    let create_nullifier_queue_instruction = create_account_instruction(
        authority,
        queue_rent_exemption.size,
        queue_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_nullifier_queue_keypair),
    );
    let create_state_merkle_tree_instruction = create_account_instruction(
        authority,
        merkle_tree_rent_exemption.size,
        merkle_tree_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_address_merkle_tree_keypair),
    );
    let instruction = light_registry::account_compression_cpi::sdk::create_rollover_address_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        },epoch
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

pub async fn perform_state_merkle_tree_roll_over<R: RpcConnection>(
    rpc: &mut R,
    authority: &Keypair,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    epoch: u64,
) -> Result<(), RpcError> {
    let instructions = create_rollover_address_merkle_tree_instructions(
        rpc,
        &authority.pubkey(),
        new_nullifier_queue_keypair,
        new_state_merkle_tree_keypair,
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        epoch,
    )
    .await;
    rpc.create_and_send_transaction(
        &instructions,
        &authority.pubkey(),
        &[
            authority,
            new_nullifier_queue_keypair,
            new_state_merkle_tree_keypair,
        ],
    )
    .await?;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub async fn create_rollover_state_merkle_tree_instructions<R: RpcConnection>(
    rpc: &mut R,
    authority: &Pubkey,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    cpi_context: &Pubkey,
    epoch: u64,
) -> Vec<Instruction> {
    let (merkle_tree_config, queue_config) = get_state_bundle_config(
        rpc,
        StateMerkleTreeAccounts {
            merkle_tree: *merkle_tree_pubkey,
            nullifier_queue: *nullifier_queue_pubkey,
            cpi_context: *cpi_context,
        },
    )
    .await;
    let (state_merkle_tree_rent_exemption, queue_rent_exemption) =
        get_rent_exemption_for_state_merkle_tree_and_queue(rpc, &merkle_tree_config, &queue_config)
            .await;
    let create_nullifier_queue_instruction = create_account_instruction(
        authority,
        queue_rent_exemption.size,
        queue_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_nullifier_queue_keypair),
    );
    let create_state_merkle_tree_instruction = create_account_instruction(
        authority,
        state_merkle_tree_rent_exemption.size,
        state_merkle_tree_rent_exemption.lamports,
        &account_compression::ID,
        Some(new_state_merkle_tree_keypair),
    );
    let instruction = create_rollover_state_merkle_tree_instruction(
        CreateRolloverMerkleTreeInstructionInputs {
            authority: *authority,
            new_queue: new_nullifier_queue_keypair.pubkey(),
            new_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
            old_queue: *nullifier_queue_pubkey,
            old_merkle_tree: *merkle_tree_pubkey,
        },
        epoch,
    );
    vec![
        create_nullifier_queue_instruction,
        create_state_merkle_tree_instruction,
        instruction,
    ]
}

pub async fn mint_standard_tokens<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    authority: &Keypair,
    recipient: &Pubkey,
    amount: u64,
    merkle_tree: &Pubkey,
) -> Result<Signature, RpcError> {
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
        .await?
        .unwrap();
    let mint = protocol_config.config.mint;
    let ix = light_registry::sdk::create_mint_to_instruction(
        &mint,
        &authority.pubkey(),
        recipient,
        amount,
        merkle_tree,
    );

    let (event, signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[ix],
            &authority.pubkey(),
            &[authority],
            None,
        )
        .await?
        .unwrap();
    indexer.add_event_and_compressed_accounts(&event);
    Ok(signature)
}

pub struct DepositInputs<'a> {
    pub sender: &'a Keypair,
    pub amount: u64,
    pub delegate_account: Option<FetchedAccount<DelegateAccount>>,
    pub input_token_data: Vec<TokenDataWithContext>,
    pub input_escrow_token_account: Option<TokenDataWithContext>,
    pub epoch: u64,
}

pub struct WithdrawInputs<'a> {
    pub sender: &'a Keypair,
    pub amount: u64,
    pub delegate_account: FetchedAccount<DelegateAccount>,
    pub input_escrow_token_account: TokenDataWithContext,
}

pub async fn deposit_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: DepositInputs<'a>,
) -> Result<Signature, RpcError> {
    deposit_or_withdraw_test::<R, I, true>(rpc, indexer, inputs).await
}

pub async fn withdraw_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: WithdrawInputs<'a>,
) -> Result<Signature, RpcError> {
    let inputs = DepositInputs {
        sender: inputs.sender,
        amount: inputs.amount,
        delegate_account: Some(inputs.delegate_account),
        input_token_data: Vec::new(),
        input_escrow_token_account: Some(inputs.input_escrow_token_account),
        epoch: 0,
    };
    deposit_or_withdraw_test::<R, I, false>(rpc, indexer, inputs).await
}
pub async fn deposit_or_withdraw_test<
    'a,
    R: RpcConnection,
    I: Indexer<R>,
    const IS_DEPOSIT: bool,
>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: DepositInputs<'a>,
) -> Result<Signature, RpcError> {
    let mut input_compressed_accounts = Vec::new();

    inputs.input_token_data.iter().for_each(|t| {
        input_compressed_accounts.push(t.compressed_account.clone());
    });

    if let Some(escrow_token_account) = inputs.input_escrow_token_account.as_ref() {
        input_compressed_accounts.push(escrow_token_account.compressed_account.clone());
    }
    let first_mt = if let Some(token_data_with_context) = inputs.input_escrow_token_account.as_ref()
    {
        token_data_with_context
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey
    } else {
        inputs.input_token_data[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey
    };

    if let Some(delegate_account) = inputs.delegate_account.as_ref() {
        input_compressed_accounts.push(delegate_account.comporessed_account.clone());
        println!("delegate_account: {:?}", delegate_account);
    };

    let cpi_context_account = indexer.get_state_merkle_tree_accounts(&[first_mt])[0].cpi_context;

    let input_hashes = input_compressed_accounts
        .iter()
        .map(|a| a.hash().unwrap())
        .collect::<Vec<_>>();
    println!("input_hashes: {:?}", input_hashes);
    let proof_rpc_result = indexer
        .create_proof_for_compressed_accounts(
            Some(&input_hashes),
            Some(
                &input_compressed_accounts
                    .iter()
                    .map(|a| a.merkle_context.merkle_tree_pubkey)
                    .collect::<Vec<_>>(),
            ),
            None,
            None,
            rpc,
        )
        .await;
    let delegate_account = if let Some(input_pda) = inputs.delegate_account {
        let input_delegate_compressed_account = DelegateAccountWithContext {
            delegate_account: input_pda.deserialized_account,
            merkle_context: input_pda.comporessed_account.merkle_context,
            output_merkle_tree_index: input_pda
                .comporessed_account
                .merkle_context
                .merkle_tree_pubkey,
        };
        Some(input_delegate_compressed_account)
    } else {
        None
    };
    let input_token_data = inputs
        .input_token_data
        .iter()
        .map(|t| t.token_data)
        .collect::<Vec<_>>();
    let input_compressed_accounts = inputs
        .input_token_data
        .iter()
        .map(|t| t.compressed_account.clone())
        .collect::<Vec<_>>();
    let input_escrow_token_account = inputs
        .input_escrow_token_account
        .map(|t| (t.token_data, t.compressed_account));
    let create_deposit_instruction_inputs = CreateDepositInstructionInputs {
        sender: inputs.sender.pubkey(),
        cpi_context_account,
        salt: 0,
        delegate_account,
        amount: inputs.amount,
        input_token_data,
        input_compressed_accounts,
        input_escrow_token_account,
        escrow_token_account_merkle_tree: first_mt,
        change_compressed_account_merkle_tree: first_mt,
        output_delegate_compressed_account_merkle_tree: first_mt,
        proof: proof_rpc_result.proof,
        root_indices: proof_rpc_result.root_indices,
    };
    let ix = create_deposit_instruction::<IS_DEPOSIT>(create_deposit_instruction_inputs.clone());

    let (event, signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[ix],
            &inputs.sender.pubkey(),
            &[inputs.sender],
            None,
        )
        .await?
        .unwrap();
    let (created_output_accounts, created_output_token_accounts) =
        indexer.add_event_and_compressed_accounts(&event);
    assert_deposit_or_withdrawal::<IS_DEPOSIT>(
        created_output_accounts,
        created_output_token_accounts,
        create_deposit_instruction_inputs,
        inputs.epoch,
    );
    Ok(signature)
}

/// Expecting:
/// 1. pda account with stake weight add asigned deposit amount
/// 2. escrow account with add assigned deposit amount
/// 3. change account inputs sub deposit amount
pub fn assert_deposit_or_withdrawal<const IS_DEPOSIT: bool>(
    created_output_accounts: Vec<CompressedAccountWithMerkleContext>,
    created_output_token_accounts: Vec<TokenDataWithContext>,
    inputs: CreateDepositInstructionInputs,
    epoch: u64,
) {
    let escrow_authority_pda = get_escrow_token_authority(&inputs.sender, inputs.salt).0;

    let expected_escrow_token_data =
        if let Some((mut escrow_token_data, _)) = inputs.input_escrow_token_account.clone() {
            assert_eq!(
                escrow_token_data.owner, escrow_authority_pda,
                "input owner mismatch"
            );
            if IS_DEPOSIT {
                escrow_token_data.amount += inputs.amount;
            } else {
                escrow_token_data.amount -= inputs.amount;
            }
            escrow_token_data
        } else {
            TokenData {
                owner: escrow_authority_pda,
                amount: inputs.amount,
                mint: inputs.input_token_data[0].mint,
                delegate: None,
                state: light_compressed_token::token_data::AccountState::Initialized,
            }
        };
    let output_escrow_token_data = created_output_token_accounts[0].token_data;
    assert_eq!(output_escrow_token_data, expected_escrow_token_data);

    let expected_delegate_account = if let Some(mut input_pda) = inputs.delegate_account.clone() {
        input_pda.delegate_account.escrow_token_account_hash =
            output_escrow_token_data.hash::<Poseidon>().unwrap();
        if IS_DEPOSIT {
            input_pda.delegate_account.stake_weight += inputs.amount;
        } else {
            input_pda.delegate_account.stake_weight -= inputs.amount;
        }
        // input_pda.delegate_account.last_sync_epoch = epoch;
        input_pda.delegate_account
    } else {
        DelegateAccount {
            owner: inputs.sender,
            stake_weight: inputs.amount,
            pending_undelegated_stake_weight: 0,
            pending_epoch: 0,
            delegated_stake_weight: 0,
            delegate_forester_delegate_account: None,
            last_sync_epoch: epoch,
            pending_token_amount: 0,
            escrow_token_account_hash: output_escrow_token_data.hash::<Poseidon>().unwrap(),
            pending_synced_stake_weight: 0,
            pending_delegated_stake_weight: 0,
        }
    };
    let output_delegate_account = DelegateAccount::deserialize_reader(
        &mut &created_output_accounts[0]
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data[..],
    )
    .unwrap();
    println!("assert epoch {}", epoch);
    assert_eq!(output_delegate_account, expected_delegate_account);
}

pub struct DelegateInputs<'a> {
    pub sender: &'a Keypair,
    pub amount: u64,
    pub delegate_account: FetchedAccount<DelegateAccount>,
    pub forester_pda: Pubkey,
    pub output_merkle_tree: Pubkey,
}

pub struct UndelegateInputs<'a> {
    pub sender: &'a Keypair,
    pub amount: u64,
    pub delegate_account: FetchedAccount<DelegateAccount>,
    pub forester_pda: Pubkey,
    pub output_merkle_tree: Pubkey,
}

pub async fn delegate_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: DelegateInputs<'a>,
) -> Result<Signature, RpcError> {
    delegate_or_undelegate_test::<R, I, true>(rpc, indexer, inputs).await
}

pub async fn undelegate_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: UndelegateInputs<'a>,
) -> Result<Signature, RpcError> {
    let inputs = DelegateInputs {
        sender: inputs.sender,
        amount: inputs.amount,
        delegate_account: inputs.delegate_account,
        forester_pda: inputs.forester_pda,
        output_merkle_tree: inputs.output_merkle_tree,
    };
    delegate_or_undelegate_test::<R, I, false>(rpc, indexer, inputs).await
}
pub async fn delegate_or_undelegate_test<
    'a,
    R: RpcConnection,
    I: Indexer<R>,
    const IS_DEPOSIT: bool,
>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: DelegateInputs<'a>,
) -> Result<Signature, RpcError> {
    let input_compressed_accounts = vec![inputs.delegate_account.comporessed_account.clone()];

    let input_hashes = input_compressed_accounts
        .iter()
        .map(|a| a.hash().unwrap())
        .collect::<Vec<_>>();
    let proof_rpc_result = indexer
        .create_proof_for_compressed_accounts(
            Some(&input_hashes),
            Some(
                &input_compressed_accounts
                    .iter()
                    .map(|a| a.merkle_context.merkle_tree_pubkey)
                    .collect::<Vec<_>>(),
            ),
            None,
            None,
            rpc,
        )
        .await;
    let delegate_account = DelegateAccountWithContext {
        delegate_account: inputs.delegate_account.deserialized_account,
        merkle_context: inputs.delegate_account.comporessed_account.merkle_context,
        output_merkle_tree_index: inputs
            .delegate_account
            .comporessed_account
            .merkle_context
            .merkle_tree_pubkey,
    };

    let create_deposit_instruction_inputs = CreateDelegateInstructionInputs {
        sender: inputs.sender.pubkey(),
        delegate_account,
        amount: inputs.amount,
        output_delegate_compressed_account_merkle_tree: inputs.output_merkle_tree,
        proof: proof_rpc_result.proof,
        forester_pda: inputs.forester_pda,
        root_index: proof_rpc_result.root_indices[0],
    };
    let ix = create_delegate_instruction::<IS_DEPOSIT>(create_deposit_instruction_inputs.clone());
    println!("trying to fetch forester pda");
    let pre_forester_pda = rpc
        .get_anchor_account::<ForesterAccount>(&inputs.forester_pda)
        .await
        .unwrap()
        .unwrap();

    let (event, signature, _) = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[ix],
            &inputs.sender.pubkey(),
            &[inputs.sender],
            None,
        )
        .await?
        .unwrap();
    let (created_output_accounts, _) = indexer.add_event_and_compressed_accounts(&event);
    assert_delegate_or_undelegate::<IS_DEPOSIT, R>(
        rpc,
        created_output_accounts,
        create_deposit_instruction_inputs,
        pre_forester_pda,
    )
    .await;
    Ok(signature)
}

pub async fn assert_delegate_or_undelegate<const IS_DEPOSIT: bool, R: RpcConnection>(
    rpc: &mut R,
    created_output_accounts: Vec<CompressedAccountWithMerkleContext>,
    inputs: CreateDelegateInstructionInputs,
    pre_forester_pda: ForesterAccount,
) {
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let protocol_config: ProtocolConfigPda = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
        .await
        .unwrap()
        .unwrap();
    let current_slot = rpc.get_slot().await.unwrap();

    let forester_pda: ForesterAccount = rpc
        .get_anchor_account::<ForesterAccount>(&inputs.forester_pda)
        .await
        .unwrap()
        .unwrap();
    {
        let expected_forester_pda = {
            let mut input_pda = pre_forester_pda.clone();
            input_pda
                .sync(current_slot, &protocol_config.config)
                .unwrap();

            if IS_DEPOSIT {
                input_pda.pending_undelegated_stake_weight += inputs.amount;
            } else {
                input_pda.active_stake_weight -= inputs.amount;
            }
            input_pda
        };
        assert_eq!(forester_pda, expected_forester_pda);
    }
    let current_epoch = forester_pda.last_registered_epoch;

    let expected_delegate_account = {
        let mut input_pda = inputs.delegate_account.clone();
        if current_epoch > input_pda.delegate_account.pending_epoch {
            input_pda.delegate_account.stake_weight +=
                input_pda.delegate_account.pending_undelegated_stake_weight;
            input_pda.delegate_account.pending_undelegated_stake_weight = 0;
            // last sync epoch is only relevant for syncing the delegate account with the forester rewards
            // self.last_sync_epoch = current_epoch;
            input_pda.delegate_account.delegated_stake_weight +=
                input_pda.delegate_account.pending_delegated_stake_weight;
            input_pda.delegate_account.pending_delegated_stake_weight = 0;
            // self.pending_epoch = 0;
        }
        input_pda.delegate_account.pending_epoch = current_epoch;
        // input_pda.delegate_account.last_sync_epoch = current_epoch;
        if IS_DEPOSIT {
            input_pda.delegate_account.stake_weight -= inputs.amount;
            input_pda.delegate_account.pending_delegated_stake_weight += inputs.amount;
        } else {
            input_pda.delegate_account.delegated_stake_weight -= inputs.amount;
            input_pda.delegate_account.pending_undelegated_stake_weight += inputs.amount;
        }
        if input_pda.delegate_account.delegated_stake_weight != 0
            || input_pda.delegate_account.pending_delegated_stake_weight != 0
        {
            input_pda
                .delegate_account
                .delegate_forester_delegate_account = Some(inputs.forester_pda);
        } else {
            input_pda
                .delegate_account
                .delegate_forester_delegate_account = None;
        }
        input_pda.delegate_account
    };
    let output_delegate_account = DelegateAccount::deserialize_reader(
        &mut &created_output_accounts[0]
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data[..],
    )
    .unwrap();
    assert_eq!(output_delegate_account, expected_delegate_account);
}

pub struct ClaimForesterInputs<'a> {
    pub sender: &'a Keypair,
    pub amount: u64,
    pub delegate_account: FetchedAccount<DelegateAccount>,
    pub forester_pda: Pubkey,
    pub no_sync: bool,
    pub output_merkle_tree: Pubkey,
}

pub async fn forester_claim_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    forester: &Keypair,
    epoch: u64,
    output_merkle_tree: Pubkey,
) -> Result<Signature, RpcError> {
    let ix = create_forester_claim_instruction(forester.pubkey(), epoch, output_merkle_tree);
    let forester_pda = get_forester_pda_address(&forester.pubkey()).0;
    let pre_forester_pda = rpc
        .get_anchor_account::<ForesterAccount>(&forester_pda)
        .await
        .unwrap()
        .unwrap();
    let forester_epoch_pda_pubkey = get_forester_epoch_pda_address(&forester_pda, epoch).0;
    let pre_forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();

    let token_pool = get_forester_token_pool_pda(&forester_pda);
    let pre_token_pool_account = rpc.get_account(token_pool).await.unwrap().unwrap();
    let (event, signature, _) = rpc
        .create_and_send_transaction_with_events::<PublicTransactionEvent>(
            &[ix],
            &forester.pubkey(),
            &[forester],
            None,
        )
        .await?
        .unwrap();
    println!("events {:?}", event);
    let (created_output_accounts, _) = indexer.add_event_and_compressed_accounts(&event[0]);
    let (_, created_token_accounts) = indexer.add_event_and_compressed_accounts(&event[1]);
    // currently we only get the first of two public transaction events
    assert_forester_claim::<R>(
        rpc,
        created_output_accounts,
        created_token_accounts,
        forester,
        epoch,
        pre_forester_pda,
        pre_token_pool_account,
        pre_forester_epoch_pda,
    )
    .await;
    Ok(signature)
}

pub async fn assert_forester_claim<R: RpcConnection>(
    rpc: &mut R,
    created_output_accounts: Vec<CompressedAccountWithMerkleContext>,
    created_token_accounts: Vec<TokenDataWithContext>,
    forester: &Keypair,
    epoch: u64,
    pre_forester_pda: ForesterAccount,
    pre_token_pool_account: Account,
    pre_forester_epoch_pda: ForesterEpochPda,
) {
    let forester_pda_pubkey = get_forester_pda_address(&forester.pubkey()).0;
    // assert compressed account
    let rewards = {
        let deserialized_compressed_epoch_account =
            CompressedForesterEpochAccount::deserialize_reader(
                &mut &created_output_accounts[0]
                    .compressed_account
                    .data
                    .as_ref()
                    .unwrap()
                    .data[..],
            )
            .unwrap();
        let expected_compressed_epoch_account = CompressedForesterEpochAccount {
            rewards_earned: deserialized_compressed_epoch_account.rewards_earned,
            epoch,
            stake_weight: pre_forester_epoch_pda.stake_weight, // this doesn't have to be true since the active stake weight can change in registration phase
            previous_hash: pre_forester_pda.last_compressed_forester_epoch_pda_hash,
            forester_pda_pubkey,
        };
        println!(
            "deserialized_compressed_epoch_account {:?}",
            deserialized_compressed_epoch_account
        );
        assert!(expected_compressed_epoch_account.rewards_earned > 0);
        assert_eq!(
            deserialized_compressed_epoch_account,
            expected_compressed_epoch_account
        );
        deserialized_compressed_epoch_account.rewards_earned
    };
    // assert token pool update
    let mint = {
        let pre_amount = spl_token::state::Account::unpack(&pre_token_pool_account.data)
            .unwrap()
            .amount;
        let token_pool_pda_pubkey = get_forester_token_pool_pda(&forester_pda_pubkey);
        let post_account = rpc
            .get_account(token_pool_pda_pubkey)
            .await
            .unwrap()
            .unwrap();
        let unpacked_post_account = spl_token::state::Account::unpack(&post_account.data).unwrap();
        assert_eq!((unpacked_post_account.amount - pre_amount), rewards);
        unpacked_post_account.mint
    };

    let forester_pda = rpc
        .get_anchor_account::<ForesterAccount>(&forester_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    {
        let current_epoch = get_current_epoch(rpc).await.unwrap();
        let expected_forester_pda = {
            let mut input_pda = pre_forester_pda.clone();
            let protocol_config_pda = get_protocol_config_pda_address().0;
            let protocol_config: ProtocolConfigPda = rpc
                .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
                .await
                .unwrap()
                .unwrap();
            let current_slot = rpc.get_slot().await.unwrap();
            input_pda
                .sync(current_slot, &protocol_config.config)
                .unwrap();
            input_pda.last_compressed_forester_epoch_pda_hash = created_output_accounts[0]
                .compressed_account
                .data
                .as_ref()
                .unwrap()
                .data_hash;
            input_pda.active_stake_weight += rewards;
            input_pda.last_claimed_epoch = epoch;
            input_pda.current_epoch = current_epoch;
            input_pda
        };
        assert_eq!(forester_pda, expected_forester_pda);
    }
    let forester_epoch_pda_pubkey = get_forester_epoch_pda_address(&forester_pda_pubkey, epoch).0;
    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
        .await
        .unwrap();
    assert!(
        forester_epoch_pda.is_none(),
        "Forester epoch pda should be closed."
    );
    // assert forester fee compressed token account
    {
        let token_data = created_token_accounts[0].token_data;
        let forester_fee = pre_forester_pda.config.fee as f64 / 100.0;
        let forester_rewards = rewards as f64 / (1.0 - forester_fee) * forester_fee;
        let expected_token_data = TokenData {
            owner: forester.pubkey(),
            amount: forester_rewards as u64,
            mint,
            delegate: None,
            state: light_compressed_token::token_data::AccountState::Initialized,
        };
        assert_eq!(token_data, expected_token_data);
    }
}

pub async fn get_current_epoch<R: RpcConnection>(rpc: &mut R) -> Result<u64, RpcError> {
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let protocol_config: ProtocolConfigPda = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda)
        .await
        .unwrap()
        .unwrap();
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config
        .config
        .get_current_registration_epoch(current_slot);
    Ok(current_epoch)
}

pub struct SyncDelegateInputs<'a> {
    pub sender: &'a Keypair,
    // pub amount: u64,
    pub delegate_account: FetchedAccount<DelegateAccount>,
    pub forester: Pubkey,
    pub output_merkle_tree: Pubkey,
    pub input_escrow_token_account: Option<TokenDataWithContext>,
    pub compressed_forester_epoch_pdas: Vec<Option<FetchedAccount<CompressedForesterEpochAccount>>>,
    // TODO: remove and get from epoch - 1 of compressed_forester_epoch_pdas
    pub previous_hash: [u8; 32],
    pub sync_delegate_token_account: bool,
    // pub last_account_merkle_context: MerkleContext,
}

pub async fn sync_delegate_test<'a, R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    inputs: SyncDelegateInputs<'a>,
) -> Result<Signature, RpcError> {
    let mut input_compressed_accounts = vec![inputs.delegate_account.comporessed_account.clone()];
    let input_escrow_token_account =
        if let Some(escrow_token_account) = inputs.input_escrow_token_account {
            input_compressed_accounts.push(escrow_token_account.compressed_account.clone());
            Some((
                escrow_token_account.token_data,
                escrow_token_account.compressed_account,
            ))
        } else {
            None
        };

    let input_hashes = input_compressed_accounts
        .iter()
        .map(|a| a.hash().unwrap())
        .collect::<Vec<_>>();
    println!("input_hashes {:?}", input_hashes);
    let proof_rpc_result = indexer
        .create_proof_for_compressed_accounts(
            Some(&input_hashes),
            Some(
                &input_compressed_accounts
                    .iter()
                    .map(|a| a.merkle_context.merkle_tree_pubkey)
                    .collect::<Vec<_>>(),
            ),
            None,
            None,
            rpc,
        )
        .await;
    let delegate_account = DelegateAccountWithContext {
        delegate_account: inputs.delegate_account.deserialized_account,
        merkle_context: inputs.delegate_account.comporessed_account.merkle_context,
        output_merkle_tree_index: inputs
            .delegate_account
            .comporessed_account
            .merkle_context
            .merkle_tree_pubkey,
    };
    let first_mt = inputs
        .delegate_account
        .comporessed_account
        .merkle_context
        .merkle_tree_pubkey;

    let cpi_context_account = indexer.get_state_merkle_tree_accounts(&[first_mt])[0].cpi_context;

    let mut compressed_forester_epoch_pdas = inputs
        .compressed_forester_epoch_pdas
        .iter()
        .filter(|a| a.is_some())
        .map(|a| {
            let x = a.as_ref().unwrap().deserialized_account;

            CompressedForesterEpochAccountInput {
                rewards_earned: x.rewards_earned,
                epoch: x.epoch,
                stake_weight: x.stake_weight,
            }
        })
        .filter(|a| {
            if inputs.delegate_account.deserialized_account.last_sync_epoch == 0 {
                true
            } else {
                a.epoch > inputs.delegate_account.deserialized_account.last_sync_epoch
            }
        })
        .collect::<Vec<_>>();
    compressed_forester_epoch_pdas.reverse();
    println!(
        "compressed_forester_epoch_pdas {:?}",
        compressed_forester_epoch_pdas.len()
    );
    if compressed_forester_epoch_pdas.is_empty() {
        return Ok(Signature::default());
    }
    let last_account = inputs
        .compressed_forester_epoch_pdas
        .iter()
        .filter(|a| a.is_some())
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .comporessed_account
        .clone();
    let create_instruction_inputs = CreateSyncDelegateInstructionInputs {
        sender: inputs.sender.pubkey(),
        delegate_account,
        output_delegate_compressed_account_merkle_tree: inputs.output_merkle_tree,
        proof: proof_rpc_result.proof,
        salt: 0,
        cpi_context_account,
        output_token_account_merkle_tree: inputs.output_merkle_tree,
        root_indices: proof_rpc_result.root_indices,
        input_escrow_token_account,
        forester_pubkey: inputs.forester,
        previous_hash: inputs.previous_hash,
        compressed_forester_epoch_pdas: compressed_forester_epoch_pdas.clone(),
        last_account_root_index: 0, //TODO: add once we verify the proof onchain
        sync_delegate_token_account: inputs.sync_delegate_token_account,
        last_account_merkle_context: last_account.merkle_context,
    };
    let ix = create_sync_delegate_instruction(create_instruction_inputs.clone());
    println!("delegate_account {:?}", delegate_account);
    let forester_pda_pubkey = get_forester_pda_address(&inputs.forester).0;

    let token_pool = get_forester_token_pool_pda(&forester_pda_pubkey);
    let pre_token_pool_account = rpc.get_account(token_pool).await.unwrap().unwrap();
    let (event, signature, _) = rpc
        .create_and_send_transaction_with_events::<PublicTransactionEvent>(
            &[ix],
            &inputs.sender.pubkey(),
            &[inputs.sender],
            None,
        )
        .await?
        .unwrap();
    let (created_output_accounts, created_token_accounts) =
        indexer.add_event_and_compressed_accounts(&event[0]);
    println!("created_output_accounts {:?}", created_output_accounts);
    println!("created_token_accounts {:?}", created_token_accounts);
    assert_sync_delegate::<R>(
        rpc,
        created_output_accounts,
        created_token_accounts,
        create_instruction_inputs,
        pre_token_pool_account,
        compressed_forester_epoch_pdas,
    )
    .await;
    Ok(signature)
}

/// Expecting:
/// 1. update compressed token escrow account
/// 2. update delegate account
/// 3. updated token pool account
pub async fn assert_sync_delegate<R: RpcConnection>(
    rpc: &mut R,
    created_output_accounts: Vec<CompressedAccountWithMerkleContext>,
    created_output_token_account: Vec<TokenDataWithContext>,
    inputs: CreateSyncDelegateInstructionInputs,
    pre_token_pool_account: Account,
    input_compressed_epochs: Vec<CompressedForesterEpochAccountInput>,
) {
    let rewards = if let Some((token_data, _input_escrow_token_account)) =
        inputs.input_escrow_token_account
    {
        let actual_amount = created_output_token_account[0].token_data.amount - token_data.amount;
        println!("actual_amount {:?}", actual_amount);
        println!(
            "created_output_token_account {:?}",
            created_output_token_account
        );
        println!("input_compressed_epochs {:?}", input_compressed_epochs);
        // account holds all stakeweight should get all rewards
        if inputs
            .delegate_account
            .delegate_account
            .delegated_stake_weight
            == input_compressed_epochs[0].stake_weight
        {
            let sum_rewards = input_compressed_epochs
                .iter()
                .map(|a| a.rewards_earned)
                .sum::<u64>();
            // TODO: can I check that users cannot invoke if there is nothing to claim?
            // assert_eq!(sum_rewards, actual_amount);
            // assert!(0 < actual_amount);
            assert!(actual_amount <= sum_rewards);
        } else {
            let sum_rewards = input_compressed_epochs
                .iter()
                .map(|a| a.rewards_earned)
                .sum::<u64>();
            // assert!(0 < actual_amount);
            // assert_eq!(sum_rewards, actual_amount);
            assert!(actual_amount <= sum_rewards);
        }
        Some(actual_amount)
    } else {
        None
    };

    // assert token pool update
    if let Some(rewards) = rewards {
        let pre_amount = spl_token::state::Account::unpack(&pre_token_pool_account.data)
            .unwrap()
            .amount;
        let forester_pda_pubkey = get_forester_pda_address(&inputs.forester_pubkey).0;
        let token_pool_pda_pubkey = get_forester_token_pool_pda(&forester_pda_pubkey);
        let post_account = rpc
            .get_account(token_pool_pda_pubkey)
            .await
            .unwrap()
            .unwrap();
        let unpacked_post_account = spl_token::state::Account::unpack(&post_account.data).unwrap();
        assert_eq!((pre_amount - unpacked_post_account.amount), rewards);
    }

    let updated_delegate_account = created_output_accounts[0].clone();
    let deserialized_delegate_account = DelegateAccount::deserialize_reader(
        &mut &updated_delegate_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data[..],
    )
    .unwrap();
    let epoch = input_compressed_epochs.last().unwrap().epoch;
    println!("\n\n epoch {:?} \n\n", epoch);
    println!(
        "input compressed token accounts: {:?}",
        input_compressed_epochs
    );
    let expected_delegate_account = if let Some(rewards) = rewards {
        println!("if");
        let expected_delegate_account = {
            let mut input_pda = inputs.delegate_account.delegate_account.clone();
            if epoch > input_pda.pending_epoch {
                input_pda.stake_weight += input_pda.pending_undelegated_stake_weight;
                input_pda.pending_undelegated_stake_weight = 0;

                input_pda.delegated_stake_weight += input_pda.pending_delegated_stake_weight;
                input_pda.pending_delegated_stake_weight = 0;
                input_pda.pending_token_amount = 0;
            }
            input_pda.delegated_stake_weight += rewards;
            // pending epoch doesnt change because it is just responsible for
            // syncing delegated
            // input_pda.pending_epoch = 0;
            // input_pda.pending_epoch = epoch;
            input_pda.sync_pending_stake_weight(epoch);

            input_pda.last_sync_epoch = epoch;
            input_pda.pending_synced_stake_weight =
                deserialized_delegate_account.pending_synced_stake_weight;
            input_pda.escrow_token_account_hash = created_output_token_account[0]
                .token_data
                .hash::<Poseidon>()
                .unwrap();

            input_pda
        };
        expected_delegate_account
    } else {
        println!("else");
        let expected_delegate_account = {
            let mut input_pda = inputs.delegate_account.delegate_account.clone();
            input_pda.stake_weight += input_pda.pending_undelegated_stake_weight;
            input_pda.pending_undelegated_stake_weight = 0;
            let sum_rewards = input_compressed_epochs
                .iter()
                .map(|a| a.rewards_earned)
                .sum::<u64>();
            input_pda.delegated_stake_weight +=
                sum_rewards + input_pda.pending_delegated_stake_weight;
            input_pda.pending_delegated_stake_weight = 0;
            input_pda.pending_token_amount += sum_rewards;
            // input_pda.pending_epoch = epoch;
            input_pda.sync_pending_stake_weight(epoch);

            input_pda.last_sync_epoch = epoch;
            input_pda.pending_synced_stake_weight =
                deserialized_delegate_account.pending_synced_stake_weight;
            println!(
                "token_data {:?}",
                created_output_token_account[0].token_data
            );
            input_pda.escrow_token_account_hash = created_output_token_account[0]
                .token_data
                .hash::<Poseidon>()
                .unwrap();
            input_pda
        };
        expected_delegate_account
    };
    let actual_amount = deserialized_delegate_account.pending_synced_stake_weight;
    let last_epoch_reward = input_compressed_epochs
        .iter()
        .last()
        .unwrap()
        .rewards_earned;
    // assert!(0 < actual_amount);
    assert!(actual_amount <= last_epoch_reward);
    assert_eq!(deserialized_delegate_account, expected_delegate_account);
}
