use {
    account_compression::{
        address_merkle_tree_from_bytes_zero_copy, instruction as compression_instruction,
        queue_from_bytes_zero_copy_mut, AddressMerkleTreeAccount, AddressMerkleTreeConfig,
        AddressQueueConfig, QueueAccount,
    },
    anchor_lang::InstructionData,
    base64::{engine::general_purpose::STANDARD, Engine as _},
    borsh::BorshSerialize,
    light_batched_merkle_tree::{
        constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
        initialize_address_tree::InitAddressTreeAccountsInstructionData,
        initialize_state_tree::InitStateTreeAccountsInstructionData,
        merkle_tree::{get_merkle_tree_account_size, InstructionDataBatchAppendInputs},
        queue::get_output_queue_account_size,
    },
    light_compressed_account::instruction_data::{
        compressed_proof::CompressedProof, insert_into_queues::InsertIntoQueuesInstructionDataMut,
    },
    light_hasher::{
        bigint::bigint_to_be_bytes_array, hash_chain::create_hash_chain_from_slice, Poseidon,
    },
    light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree},
    light_merkle_tree_reference::MerkleTree,
    light_prover_client::{
        proof_client::ProofClient, proof_types::batch_append::get_batch_append_inputs,
        prover::spawn_prover,
    },
    mollusk_svm::{program::ProgramCache, Mollusk},
    mollusk_svm_result::types::TransactionResult,
    num_bigint::BigUint,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    std::{
        collections::HashSet,
        env,
        error::Error,
        fmt::Display,
        fs, io,
        path::{Path, PathBuf},
    },
};

type AppResult<T> = Result<T, Box<dyn Error>>;

const ACCOUNT_COMPRESSION_PROGRAM: Address =
    Address::from_str_const("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
const NOOP_PROGRAM: Address =
    Address::from_str_const("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");
const CPI_AUTHORITY: Address =
    Address::from_str_const("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");
const REGISTERED_PROGRAM: Address =
    Address::from_str_const("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
const MAINNET_STATE_TREE: Address =
    Address::from_str_const("smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9");
const MAINNET_TRANSACTION: &str =
    "2iT1exGBk773wicyqhoEiZZVjSBQhgxmoo2YHr5H177PCgU4TThbRrWqQg7y2g64Svxxa4v7GjDwFAQdTUFwfKCv";

const MAINNET_INSTRUCTIONS: [&str; 3] = [
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJwj76Jrj9WGT1H4aBMhta1RCGaDAhd5cpEkXM8DkpqNF84XKoyP4fB3zHEXeqw6BiDjCr4a85XyByX4poUJBQUd4atCRq3sXm4KtxcrSNCvjXaR5WXhfvJDgdm5cg2wYfSHuHHJgXGFFPs8fuC6Ld6zSyNqvdtb3YRh1kQw4p7ajcLyajsmhe1jfJ2UBW4dAgE7AyQXjA3hhz6UsXM91gMqjHmQ8CbkFqUWEchCGJqR7GFEJ2RtZgSRg7wtEB69L9nxhWZDEyscbikpEnWLP8DKn5PbbSJbhMo9WDh61aF4dHCotdBeVnhPUktzEqAkT8xdUkUj8HATW4WD6iPyCWDjijchzf1Je7Euh5cwcTLqHjoWp8dJ6rzyu",
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJwbhTx7UoUdSCjbdr52UVTL2gnp9EUKbWMSdYDStQ9gh2GWRtAHoU474cHcVx3y7bA4kWk2DymwFecEoCUxC49oFcS9yqTskpGBxeVfKA6MCCFTRzaEXmj7dfAEKUGJERT9RHDeH1qJk1U2kd4gbc7nZy4i2WjqjDsAkhLwqehxJdZdpHG5fJ4XRfewXMCYoT8q3MwqX4aW25RHbnin3vm8oUgf1kEFApyjvtv5ya4Xco4B1xDjCW8Fb7Qdeq5zefS84dFNJ4mK37MiGN2zTbU7ACPFrSBNFyZAYLxnjsXP2YWKAnk4zevV1tobaxhXsKy45gd52CituXJS9nj8uq1BsYzbcL1Fwhzqh9fCVCgQ8Xgerih7JYAaT",
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJws9jgoewPcbrYMrBtvTsXKVhNPiEwuLK9YwxFnZ7yJL6Tg7cKeHqZirMLAFYvS5pfhCBwBZsU6Gu5TvVUCETjXAbtpKjhAeiekLXPnL5XmP3hn23C5cezTxcGTH46fNYxzAR1p4op15ba5TXmPntBHdwhLKzp8pPhT8RHXksN2tmssN9usfjy8tX2KGGdqNaPrmdMn7cKG2aNSqF9rnjmMAXHA8gkFhaYTRzfVeFQjb7JChDc2v68zHpMnH6dtFtManhRwQxeLtSejwMZLzSB3f7z5LDCUdYjjejdaAuxc7jsXJrjeBphbaUwiRr9uYsG5tdQmzsP9TCGGmrF4etj13Ybwmnfj8qQ5hasMmqHiZYYbTFw7JaagF",
];

const LOCAL_SLOT: u64 = 1_000;
const LOCAL_ACCOUNT_LAMPORTS: u64 = 100_000_000_000;
const ADDRESS: [u8; 32] = [1; 32];

#[derive(Debug)]
struct Config {
    mainnet_elf_path: PathBuf,
    local_elf_path: PathBuf,
    noop_elf_path: PathBuf,
    mainnet_registered_program_path: PathBuf,
    mainnet_state_tree_path: PathBuf,
    output_dir: PathBuf,
}

impl Config {
    fn from_env() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir.join("fixtures");

        Self {
            mainnet_elf_path: env_path_or(
                "MOLLUSK_VAS_ELF",
                fixtures_dir.join("account-compression-mainnet.so"),
            ),
            local_elf_path: env_path_or(
                "MOLLUSK_VAS_LOCAL_ELF",
                manifest_dir.join("../../target/deploy/account_compression.so"),
            ),
            noop_elf_path: env_path_or(
                "MOLLUSK_VAS_NOOP_ELF",
                manifest_dir.join("../../third-party/solana-program-library/spl_noop.so"),
            ),
            mainnet_registered_program_path: env_path_or(
                "MOLLUSK_VAS_REGISTERED_PROGRAM_ACCOUNT",
                fixtures_dir.join(format!("{REGISTERED_PROGRAM}.bin")),
            ),
            mainnet_state_tree_path: env_path_or(
                "MOLLUSK_VAS_STATE_TREE_ACCOUNT",
                fixtures_dir.join(format!("{MAINNET_STATE_TREE}.bin")),
            ),
            output_dir: env_path_or("MOLLUSK_VAS_OUTPUT_DIR", manifest_dir.join("output")),
        }
    }
}

struct ReproCase {
    name: &'static str,
    source: String,
    slot: u64,
    elf_path: PathBuf,
    instructions: Vec<Instruction>,
    accounts: Vec<(Address, Account)>,
    expected_divergent_accounts: HashSet<Address>,
    required_mutated_accounts: HashSet<Address>,
}

fn env_path_or(name: &str, default: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(default)
}

fn other(error: impl Display) -> Box<dyn Error> {
    io::Error::other(error.to_string()).into()
}

fn local_address(byte: u8) -> Address {
    Address::new_from_array([byte; 32])
}

fn system_account(lamports: u64) -> Account {
    Account {
        lamports,
        owner: solana_sdk_ids::system_program::ID,
        ..Account::default()
    }
}

fn program_owned_account(size: usize) -> Account {
    Account {
        lamports: LOCAL_ACCOUNT_LAMPORTS,
        data: vec![0; size],
        owner: ACCOUNT_COMPRESSION_PROGRAM,
        ..Account::default()
    }
}

fn noop_program_account() -> Account {
    Account {
        owner: solana_sdk_ids::bpf_loader_upgradeable::ID,
        executable: true,
        ..Account::default()
    }
}

fn load_raw_account(path: &Path, owner: Address) -> AppResult<Account> {
    Ok(Account {
        lamports: LOCAL_ACCOUNT_LAMPORTS,
        data: fs::read(path)?,
        owner,
        ..Account::default()
    })
}

fn decoded_instruction(
    program_id: Address,
    accounts: Vec<AccountMeta>,
    data: &str,
) -> AppResult<Instruction> {
    Ok(Instruction {
        program_id,
        accounts,
        data: bs58::decode(data).into_vec()?,
    })
}

fn anchor_instruction(accounts: Vec<AccountMeta>, data: Vec<u8>) -> AppResult<Instruction> {
    Ok(Instruction {
        program_id: ACCOUNT_COMPRESSION_PROGRAM,
        accounts,
        data,
    })
}

fn insert_addresses_instruction(
    authority: Address,
    queue: Address,
    tree: Address,
    addresses: &[[u8; 32]],
) -> AppResult<Instruction> {
    let num_addresses = u8::try_from(addresses.len())?;
    let mut bytes = vec![
        0;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            0,
            0,
            num_addresses,
            0,
            0,
            1,
        )
    ];
    let (mut data, remaining) =
        InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 0, 0, num_addresses, 0, 0, 1)?;
    if !remaining.is_empty() {
        return Err(io::Error::other("address instruction buffer was not fully consumed").into());
    }
    data.num_address_queues = 1;
    let is_batched = queue == tree;
    for (input, address) in data.addresses.iter_mut().zip(addresses) {
        input.address = *address;
        input.queue_index = 0;
        input.tree_index = u8::from(!is_batched);
    }

    anchor_instruction(
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(queue, false),
            AccountMeta::new(tree, false),
        ],
        compression_instruction::InsertIntoQueues { bytes }.data(),
    )
}

fn insert_output_leaves_instruction(
    authority: Address,
    output_queue: Address,
    leaves: &[[u8; 32]],
) -> AppResult<Instruction> {
    let num_leaves = u8::try_from(leaves.len())?;
    let mut bytes = vec![
        0;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            num_leaves, 0, 0, 1, 0, 0,
        )
    ];
    let (mut data, remaining) =
        InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, num_leaves, 0, 0, 1, 0, 0)?;
    if !remaining.is_empty() {
        return Err(io::Error::other("leaf instruction buffer was not fully consumed").into());
    }
    data.num_output_queues = 1;
    for (input, leaf) in data.leaves.iter_mut().zip(leaves) {
        input.account_index = 0;
        input.leaf = *leaf;
    }

    anchor_instruction(
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(output_queue, false),
        ],
        compression_instruction::InsertIntoQueues { bytes }.data(),
    )
}

fn build_mollusk(config: &Config, elf_path: &Path, feature_enabled: bool) -> AppResult<Mollusk> {
    let program_id = ACCOUNT_COMPRESSION_PROGRAM;
    let noop_program = NOOP_PROGRAM;
    let mut mollusk = Mollusk::default();
    mollusk.feature_set.virtual_address_space_adjustments = feature_enabled;
    mollusk.program_cache = ProgramCache::new(&mollusk.feature_set, &mollusk.compute_budget, false);
    mollusk.add_program_with_loader_and_elf(
        &program_id,
        &solana_sdk_ids::bpf_loader_upgradeable::ID,
        &fs::read(elf_path)?,
    );
    mollusk.add_program_with_loader_and_elf(
        &noop_program,
        &solana_sdk_ids::bpf_loader_upgradeable::ID,
        &fs::read(&config.noop_elf_path)?,
    );
    Ok(mollusk)
}

fn execute(
    config: &Config,
    elf_path: &Path,
    feature_enabled: bool,
    slot: u64,
    instructions: &[Instruction],
    accounts: &[(Address, Account)],
) -> AppResult<TransactionResult> {
    let mut mollusk = build_mollusk(config, elf_path, feature_enabled)?;
    mollusk.warp_to_slot(slot);
    Ok(mollusk.process_transaction_instructions(instructions, accounts))
}

fn execute_setup(
    config: &Config,
    label: &str,
    instructions: &[Instruction],
    accounts: &[(Address, Account)],
) -> AppResult<Vec<(Address, Account)>> {
    let result = execute(
        config,
        &config.local_elf_path,
        false,
        LOCAL_SLOT,
        instructions,
        accounts,
    )?;
    if result.program_result.is_err() {
        return Err(
            io::Error::other(format!("{label} failed: {:?}", result.program_result)).into(),
        );
    }
    Ok(result.resulting_accounts)
}

async fn build_cases(config: &Config) -> AppResult<Vec<ReproCase>> {
    Ok(vec![
        build_mainnet_case(config)?,
        build_local_amt1_case(config)?,
        build_local_amt2_case(config)?,
        build_local_bmt1_case(config).await?,
    ])
}

fn build_mainnet_case(config: &Config) -> AppResult<ReproCase> {
    let program_id = ACCOUNT_COMPRESSION_PROGRAM;
    let authority = CPI_AUTHORITY;
    let registered_program = REGISTERED_PROGRAM;
    let state_tree = MAINNET_STATE_TREE;
    let account_metas = vec![
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new_readonly(registered_program, false),
        AccountMeta::new(state_tree, false),
    ];
    let instructions = MAINNET_INSTRUCTIONS
        .iter()
        .map(|data| decoded_instruction(program_id, account_metas.clone(), data))
        .collect::<AppResult<Vec<_>>>()?;

    Ok(ReproCase {
        name: "mainnet-smt",
        source: format!("mainnet transaction {MAINNET_TRANSACTION}"),
        slot: 431_327_345,
        elf_path: config.mainnet_elf_path.clone(),
        instructions,
        accounts: vec![
            (authority, system_account(1)),
            (
                registered_program,
                load_raw_account(
                    &config.mainnet_registered_program_path,
                    ACCOUNT_COMPRESSION_PROGRAM,
                )?,
            ),
            (
                state_tree,
                load_raw_account(&config.mainnet_state_tree_path, ACCOUNT_COMPRESSION_PROGRAM)?,
            ),
        ],
        expected_divergent_accounts: HashSet::from([state_tree]),
        required_mutated_accounts: HashSet::from([state_tree]),
    })
}

fn build_local_amt1_case(config: &Config) -> AppResult<ReproCase> {
    let program_id = ACCOUNT_COMPRESSION_PROGRAM;
    let noop = NOOP_PROGRAM;
    let authority = local_address(1);
    let tree = local_address(2);
    let queue = local_address(3);
    let tree_config = AddressMerkleTreeConfig::default();
    let queue_config = AddressQueueConfig::default();
    let height = tree_config.height as usize;
    let canopy_depth = tree_config.canopy_depth as usize;
    let tree_size = AddressMerkleTreeAccount::size(
        height,
        tree_config.changelog_size as usize,
        tree_config.roots_size as usize,
        canopy_depth,
        tree_config.address_changelog_size as usize,
    );
    let queue_size = QueueAccount::size(queue_config.capacity as usize).map_err(other)?;
    let init = anchor_instruction(
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(tree, false),
            AccountMeta::new(queue, false),
            AccountMeta::new_readonly(program_id, false),
        ],
        compression_instruction::InitializeAddressMerkleTreeAndQueue {
            index: 0,
            program_owner: None,
            forester: None,
            address_merkle_tree_config: tree_config,
            address_queue_config: queue_config,
        }
        .data(),
    )?;
    let initial_accounts = vec![
        (authority, system_account(LOCAL_ACCOUNT_LAMPORTS)),
        (tree, program_owned_account(tree_size)),
        (queue, program_owned_account(queue_size)),
    ];
    let initialized = execute_setup(config, "AMT1 initialization", &[init], &initial_accounts)?;
    let insert = insert_addresses_instruction(authority, queue, tree, &[ADDRESS])?;
    let mut prestate = execute_setup(config, "AMT1 queue insertion", &[insert], &initialized)?;

    let tree_account = account(&prestate, &tree)?;
    let onchain_tree =
        address_merkle_tree_from_bytes_zero_copy(&tree_account.data).map_err(other)?;
    let changelog_index = u16::try_from(onchain_tree.changelog_index())?;
    let indexed_changelog_index = u16::try_from(onchain_tree.indexed_changelog_index())?;

    let address_value = BigUint::from_bytes_be(&ADDRESS);
    let mut reference_tree =
        IndexedMerkleTree::<Poseidon, usize>::new(height, canopy_depth).map_err(other)?;
    reference_tree.init().map_err(other)?;
    let mut indexed_array = IndexedArray::<Poseidon, usize>::default();
    indexed_array.init().map_err(other)?;
    if onchain_tree.root() != reference_tree.root() {
        return Err(io::Error::other("local AMT1 root does not match reference tree").into());
    }
    let (low_address, low_address_next_value) = indexed_array
        .find_low_element_for_nonexistent(&address_value)
        .map_err(other)?;
    let proof = reference_tree
        .get_proof_of_leaf(low_address.index, false)
        .map_err(other)?;
    let low_address_proof: [[u8; 32]; 16] = proof.as_slice().try_into().map_err(|_| {
        io::Error::other("AMT1 proof must contain height - canopy_depth = 16 nodes")
    })?;

    let value_index = {
        let queue_account = account_mut(&mut prestate, &queue)?;
        let queue_set =
            unsafe { queue_from_bytes_zero_copy_mut(&mut queue_account.data).map_err(other)? };
        let (_, value_index) = queue_set
            .find_element(&address_value, None)
            .map_err(other)?
            .ok_or_else(|| io::Error::other("inserted AMT1 address is missing from the queue"))?;
        value_index
    };

    let update = anchor_instruction(
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new(queue, false),
            AccountMeta::new(tree, false),
            AccountMeta::new_readonly(noop, false),
            AccountMeta::new_readonly(program_id, false),
        ],
        compression_instruction::UpdateAddressMerkleTree {
            changelog_index,
            indexed_changelog_index,
            value: u16::try_from(value_index)?,
            low_address_index: u64::try_from(low_address.index)?,
            low_address_value: bigint_to_be_bytes_array(&low_address.value)?,
            low_address_next_index: u64::try_from(low_address.next_index)?,
            low_address_next_value: bigint_to_be_bytes_array(&low_address_next_value)?,
            low_address_proof,
        }
        .data(),
    )?;
    prestate.push((noop, noop_program_account()));

    Ok(ReproCase {
        name: "local-amt1-update",
        source: "fresh local AMT1 and queue".to_string(),
        slot: LOCAL_SLOT,
        elf_path: config.local_elf_path.clone(),
        instructions: vec![update],
        accounts: prestate,
        expected_divergent_accounts: HashSet::new(),
        required_mutated_accounts: HashSet::from([tree, queue]),
    })
}

fn build_local_amt2_case(config: &Config) -> AppResult<ReproCase> {
    let program_id = ACCOUNT_COMPRESSION_PROGRAM;
    let authority = local_address(11);
    let tree = local_address(12);
    let params = InitAddressTreeAccountsInstructionData::test_default();
    let tree_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
    );
    let init = anchor_instruction(
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(tree, false),
            AccountMeta::new_readonly(program_id, false),
        ],
        compression_instruction::InitializeBatchedAddressMerkleTree {
            bytes: params.try_to_vec()?,
        }
        .data(),
    )?;
    let initial_accounts = vec![
        (authority, system_account(LOCAL_ACCOUNT_LAMPORTS)),
        (tree, program_owned_account(tree_size)),
    ];
    let prestate = execute_setup(config, "AMT2 initialization", &[init], &initial_accounts)?;
    let insert = insert_addresses_instruction(authority, tree, tree, &[ADDRESS])?;

    Ok(ReproCase {
        name: "local-amt2-insert",
        source: "fresh local AMT2".to_string(),
        slot: LOCAL_SLOT,
        elf_path: config.local_elf_path.clone(),
        instructions: vec![insert],
        accounts: prestate,
        expected_divergent_accounts: HashSet::new(),
        required_mutated_accounts: HashSet::from([tree]),
    })
}

async fn build_local_bmt1_case(config: &Config) -> AppResult<ReproCase> {
    let program_id = ACCOUNT_COMPRESSION_PROGRAM;
    let noop = NOOP_PROGRAM;
    let authority = local_address(21);
    let tree = local_address(22);
    let output_queue = local_address(23);
    let params = InitStateTreeAccountsInstructionData::test_default();
    let tree_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
    );
    let queue_size = get_output_queue_account_size(
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
    );
    let init = anchor_instruction(
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(tree, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new_readonly(program_id, false),
        ],
        compression_instruction::InitializeBatchedStateMerkleTree {
            bytes: params.try_to_vec()?,
        }
        .data(),
    )?;
    let initial_accounts = vec![
        (authority, system_account(LOCAL_ACCOUNT_LAMPORTS)),
        (tree, program_owned_account(tree_size)),
        (output_queue, program_owned_account(queue_size)),
    ];
    let mut prestate = execute_setup(config, "BMT1 initialization", &[init], &initial_accounts)?;

    let leaves = (0..params.output_queue_batch_size)
        .map(|index| {
            let mut leaf = [0; 32];
            leaf[24..].copy_from_slice(&index.to_be_bytes());
            leaf
        })
        .collect::<Vec<_>>();
    for (chunk_index, chunk) in leaves.chunks(10).enumerate() {
        let insert = insert_output_leaves_instruction(authority, output_queue, chunk)?;
        prestate = execute_setup(
            config,
            &format!("BMT1 output insertion {chunk_index}"),
            &[insert],
            &prestate,
        )?;
    }

    let append_inputs = generate_bmt_append_inputs(&leaves).await?;
    let mut append_data = Vec::new();
    append_inputs.serialize(&mut append_data)?;
    println!(
        "generated BMT append inputs (base64): {}",
        STANDARD.encode(&append_data)
    );
    let append = anchor_instruction(
        vec![
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(noop, false),
            AccountMeta::new(tree, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new(authority, true),
        ],
        compression_instruction::BatchAppend { data: append_data }.data(),
    )?;
    prestate.push((noop, noop_program_account()));

    Ok(ReproCase {
        name: "local-bmt1-batch-append",
        source: "fresh local BMT1 and output queue".to_string(),
        slot: LOCAL_SLOT,
        elf_path: config.local_elf_path.clone(),
        instructions: vec![append],
        accounts: prestate,
        expected_divergent_accounts: HashSet::new(),
        required_mutated_accounts: HashSet::from([tree, output_queue]),
    })
}

async fn generate_bmt_append_inputs(
    output_queue_leaves: &[[u8; 32]],
) -> AppResult<InstructionDataBatchAppendInputs> {
    let batch_size = 10;
    let leaves = output_queue_leaves
        .get(..batch_size)
        .ok_or_else(|| io::Error::other("BMT output queue needs at least 10 leaves"))?
        .to_vec();
    let mut tree = MerkleTree::<Poseidon>::new(DEFAULT_BATCH_STATE_TREE_HEIGHT as usize, 0);
    let old_root = tree.root();
    let mut old_leaves = Vec::with_capacity(batch_size);
    let mut proofs = Vec::with_capacity(batch_size);
    for index in 0..batch_size {
        match tree.get_leaf(index) {
            Ok(leaf) => old_leaves.push(leaf),
            Err(_) => {
                old_leaves.push([0; 32]);
                if index <= tree.get_next_index() {
                    tree.append(&[0; 32])?;
                }
            }
        }
        proofs.push(tree.get_proof_of_leaf(index, true)?.to_vec());
    }
    for (index, leaf) in leaves.iter().enumerate() {
        tree.update(leaf, index)?;
    }
    let leaves_hash_chain = create_hash_chain_from_slice(&leaves)?;
    let (circuit_inputs, _) =
        get_batch_append_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
            old_root,
            0,
            leaves,
            leaves_hash_chain,
            old_leaves,
            proofs,
            u32::try_from(batch_size)?,
            &[],
        )?;

    spawn_prover().await;
    let (proof_result, new_root) = ProofClient::local()
        .generate_batch_append_proof(circuit_inputs)
        .await?;
    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof_result.proof.a,
            b: proof_result.proof.b,
            c: proof_result.proof.c,
        },
    })
}

fn account<'a>(accounts: &'a [(Address, Account)], address: &Address) -> AppResult<&'a Account> {
    accounts
        .iter()
        .find_map(|(key, account)| (key == address).then_some(account))
        .ok_or_else(|| io::Error::other(format!("missing account {address}")).into())
}

fn account_mut<'a>(
    accounts: &'a mut [(Address, Account)],
    address: &Address,
) -> AppResult<&'a mut Account> {
    accounts
        .iter_mut()
        .find_map(|(key, account)| (key == address).then_some(account))
        .ok_or_else(|| io::Error::other(format!("missing account {address}")).into())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let config = Config::from_env();
    let cases = build_cases(&config).await?;
    let mut failures = Vec::new();

    for case in &cases {
        if let Err(error) = run_and_compare_case(&config, case) {
            failures.push(format!("{}: {error}", case.name));
        }
    }

    if failures.is_empty() {
        println!("\nall VAS comparisons matched their expectations");
        return Ok(());
    }

    Err(io::Error::other(format!(
        "{} VAS comparison(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    ))
    .into())
}

fn run_and_compare_case(config: &Config, case: &ReproCase) -> AppResult<()> {
    println!("\n=== {} ===", case.name);
    println!("source: {}", case.source);
    println!("slot: {}", case.slot);

    let disabled = execute(
        config,
        &case.elf_path,
        false,
        case.slot,
        &case.instructions,
        &case.accounts,
    )?;
    let enabled = execute(
        config,
        &case.elf_path,
        true,
        case.slot,
        &case.instructions,
        &case.accounts,
    )?;
    println!(
        "disabled result: {:?}, CU: {}",
        disabled.program_result, disabled.compute_units_consumed
    );
    println!(
        "enabled result:  {:?}, CU: {}",
        enabled.program_result, enabled.compute_units_consumed
    );

    if disabled.program_result.is_err() || enabled.program_result.is_err() {
        return Err(io::Error::other("one or both executions failed").into());
    }

    assert_required_mutations(case, &disabled, "disabled")?;
    assert_required_mutations(case, &enabled, "enabled")?;

    let mut divergent_accounts = HashSet::new();
    for (address, _) in &case.accounts {
        let disabled_account = disabled.get_account(address).ok_or_else(|| {
            io::Error::other(format!("disabled result is missing account {address}"))
        })?;
        let enabled_account = enabled.get_account(address).ok_or_else(|| {
            io::Error::other(format!("enabled result is missing account {address}"))
        })?;

        if disabled_account != enabled_account {
            divergent_accounts.insert(*address);
            report_account_diff(config, case, address, disabled_account, enabled_account)?;
        } else {
            println!("account {address}: equal");
        }
    }

    if divergent_accounts != case.expected_divergent_accounts {
        return Err(io::Error::other(format!(
            "divergent accounts {:?}, expected {:?}",
            divergent_accounts, case.expected_divergent_accounts
        ))
        .into());
    }

    println!("case result: account comparison matched expectation");
    Ok(())
}

fn assert_required_mutations(
    case: &ReproCase,
    result: &TransactionResult,
    mode: &str,
) -> AppResult<()> {
    for address in &case.required_mutated_accounts {
        let before = account(&case.accounts, address)?;
        let after = result.get_account(address).ok_or_else(|| {
            io::Error::other(format!("{mode} result is missing account {address}"))
        })?;
        if before == after {
            return Err(io::Error::other(format!(
                "{mode} execution did not mutate required account {address}"
            ))
            .into());
        }
    }
    Ok(())
}

fn report_account_diff(
    config: &Config,
    case: &ReproCase,
    address: &Address,
    disabled: &Account,
    enabled: &Account,
) -> AppResult<()> {
    println!("account {address}: DIVERGED");
    if disabled.lamports != enabled.lamports {
        println!(
            "  lamports: disabled={} enabled={}",
            disabled.lamports, enabled.lamports
        );
    }
    if disabled.owner != enabled.owner {
        println!(
            "  owner: disabled={} enabled={}",
            disabled.owner, enabled.owner
        );
    }
    if disabled.executable != enabled.executable {
        println!(
            "  executable: disabled={} enabled={}",
            disabled.executable, enabled.executable
        );
    }
    if disabled.rent_epoch != enabled.rent_epoch {
        println!(
            "  rent epoch: disabled={} enabled={}",
            disabled.rent_epoch, enabled.rent_epoch
        );
    }

    if disabled.data != enabled.data {
        let ranges = diff_ranges(&disabled.data, &enabled.data);
        let differing_bytes = disabled
            .data
            .iter()
            .zip(&enabled.data)
            .filter(|(left, right)| left != right)
            .count()
            + disabled.data.len().abs_diff(enabled.data.len());
        println!(
            "  data: disabled_len={} enabled_len={} differing_bytes={} ranges={}",
            disabled.data.len(),
            enabled.data.len(),
            differing_bytes,
            format_ranges(&ranges)
        );
    }

    let output_dir = config.output_dir.join(case.name);
    fs::create_dir_all(&output_dir)?;
    fs::write(
        output_dir.join(format!("{address}-disabled.raw")),
        &disabled.data,
    )?;
    fs::write(
        output_dir.join(format!("{address}-enabled.raw")),
        &enabled.data,
    )?;
    Ok(())
}

fn diff_ranges(left: &[u8], right: &[u8]) -> Vec<(usize, usize)> {
    let max_len = left.len().max(right.len());
    let mut ranges = Vec::new();
    let mut range_start = None;

    for index in 0..max_len {
        let differs = left.get(index) != right.get(index);
        match (differs, range_start) {
            (true, None) => range_start = Some(index),
            (false, Some(start)) => {
                ranges.push((start, index));
                range_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = range_start {
        ranges.push((start, max_len));
    }
    ranges
}

fn format_ranges(ranges: &[(usize, usize)]) -> String {
    ranges
        .iter()
        .map(|(start, end)| {
            if end - start == 1 {
                start.to_string()
            } else {
                format!("{start}..{end}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
