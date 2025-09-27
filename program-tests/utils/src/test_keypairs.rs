use light_program_test::accounts::test_keypairs::*;
use solana_sdk::signature::{read_keypair_file, Keypair};

pub fn from_target_folder() -> TestKeypairs {
    let prefix = String::from("../../../light-keypairs/");
    let target_prefix = String::from("../../target/");
    let state_merkle_tree = read_keypair_file(format!(
        "{}smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT.json",
        prefix
    ))
    .unwrap();
    let nullifier_queue = read_keypair_file(
        "../../../light-keypairs/nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148.json",
    )
    .unwrap();
    let governance_authority = read_keypair_file(format!(
        "{}governance-authority-keypair.json",
        target_prefix
    ))
    .unwrap();
    let forester = read_keypair_file(format!("{}forester-keypair.json", target_prefix)).unwrap();
    let address_merkle_tree = read_keypair_file(format!(
        "{}amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2.json",
        prefix
    ))
    .unwrap();
    let address_merkle_tree_queue = read_keypair_file(format!(
        "{}aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F.json",
        prefix
    ))
    .unwrap();
    let cpi_context_account = read_keypair_file(format!(
        "{}cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4.json",
        prefix
    ))
    .unwrap();
    let system_program = read_keypair_file(format!(
        "{}SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.json",
        prefix
    ))
    .unwrap();
    let registry_program = read_keypair_file(format!(
        "{}Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX.json",
        prefix
    ))
    .unwrap();
    TestKeypairs {
        state_merkle_tree,
        nullifier_queue,
        governance_authority,
        forester,
        address_merkle_tree,
        address_merkle_tree_queue,
        cpi_context_account,
        system_program,
        registry_program,
        batched_state_merkle_tree: Keypair::try_from(
            BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR.as_slice(),
        )
        .unwrap(),
        batched_output_queue: Keypair::try_from(BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR.as_slice())
            .unwrap(),
        batched_cpi_context: Keypair::try_from(BATCHED_CPI_CONTEXT_TEST_KEYPAIR.as_slice())
            .unwrap(),
        batch_address_merkle_tree: Keypair::try_from(
            BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR.as_slice(),
        )
        .unwrap(),
        state_merkle_tree_2: Keypair::new(),
        nullifier_queue_2: Keypair::new(),
        cpi_context_2: Keypair::new(),
        group_pda_seed: Keypair::new(),
    }
}

pub fn for_regenerate_accounts() -> TestKeypairs {
    let prefix = String::from("../../../light-keypairs/");
    let state_merkle_tree = read_keypair_file(format!(
        "{}smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT.json",
        prefix
    ))
    .unwrap();

    let nullifier_queue = read_keypair_file(
        "../../../light-keypairs/nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148.json",
    )
    .unwrap();

    let governance_authority = Keypair::try_from(PAYER_KEYPAIR.as_slice()).unwrap();

    let forester = Keypair::try_from(FORESTER_TEST_KEYPAIR.as_slice()).unwrap();
    let address_merkle_tree = read_keypair_file(format!(
        "{}amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2.json",
        prefix
    ))
    .unwrap();
    let address_merkle_tree_queue = read_keypair_file(format!(
        "{}aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F.json",
        prefix
    ))
    .unwrap();
    let cpi_context_account = read_keypair_file(format!(
        "{}cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4.json",
        prefix
    ))
    .unwrap();
    let system_program = read_keypair_file(format!(
        "{}SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.json",
        prefix
    ))
    .unwrap();
    let registry_program = read_keypair_file(format!(
        "{}Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX.json",
        prefix
    ))
    .unwrap();
    let state_merkle_tree_2 = read_keypair_file(format!(
        "{}smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho.json",
        prefix
    ))
    .unwrap();
    let nullifier_queue_2 = read_keypair_file(format!(
        "{}nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X.json",
        prefix
    ))
    .unwrap();
    let cpi_context_2 = read_keypair_file(format!(
        "{}cpi2cdhkH5roePvcudTgUL8ppEBfTay1desGh8G8QxK.json",
        prefix
    ))
    .unwrap();

    TestKeypairs {
        state_merkle_tree,
        nullifier_queue,
        governance_authority,
        forester,
        address_merkle_tree,
        address_merkle_tree_queue,
        cpi_context_account,
        system_program,
        registry_program,
        batched_state_merkle_tree: Keypair::try_from(
            BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR.as_slice(),
        )
        .unwrap(),
        batched_output_queue: Keypair::try_from(BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR.as_slice())
            .unwrap(),
        batched_cpi_context: Keypair::try_from(BATCHED_CPI_CONTEXT_TEST_KEYPAIR.as_slice())
            .unwrap(),
        batch_address_merkle_tree: Keypair::try_from(
            BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR.as_slice(),
        )
        .unwrap(),
        state_merkle_tree_2,
        nullifier_queue_2,
        cpi_context_2,
        group_pda_seed: Keypair::new(),
    }
}
