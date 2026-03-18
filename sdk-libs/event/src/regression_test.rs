/// Regression test for index-out-of-bounds panic in create_nullifier_queue_indices.
/// Transaction: 3ybts1eFSC7QN6aU4ao6NJCgn7xTbtBVyzeLDZJf9eVN93vHZWupX4TXqHHgV18xf17eit7Uw5T135uabnpToKK4
/// Slot: 407265372 (mainnet)
/// This transaction crashed photon's indexer because `len` passed to
/// `create_nullifier_queue_indices` didn't match the number of input accounts.
#[cfg(test)]
mod tests {
    use light_compressed_account::Pubkey;

    use crate::parse::event_from_light_transaction;

    fn pubkey(s: &str) -> Pubkey {
        let bytes: [u8; 32] = bs58::decode(s).into_vec().unwrap().try_into().unwrap();
        Pubkey::from(bytes)
    }

    fn ix_data(s: &str) -> Vec<u8> {
        bs58::decode(s).into_vec().unwrap()
    }

    // Account addresses used in the transaction
    const USER: &str = "33X2Tg3gdxTwouaVSxpcNwVHJt2ZYxo3Hm7UjH2i8M3r";
    const REGISTERED_PDA: &str = "35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh";
    const NOOP: &str = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
    const CPI_CONTEXT: &str = "HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA";
    const ACCOUNT_COMPRESSION: &str = "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq";
    const SOL_POOL: &str = "CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1";
    const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
    const BMT3_TREE: &str = "bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb";
    const OQ3_QUEUE: &str = "oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ";
    const SMT5_TREE: &str = "smt5uPaQT9n6b1qAkgyonmzRxtuazA53Rddwntqistc";
    const NFQ5_QUEUE: &str = "nfq5b5xEguPtdD6uPetZduyrB5EUqad7gcUE46rALau";
    const BMT2_TREE: &str = "bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi";
    const OQ2_QUEUE: &str = "oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg";

    // Program IDs
    const LIGHT_SYSTEM: &str = "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7";
    const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";

    #[test]
    fn test_mainnet_tx_407265372_no_panic() {
        // Tx 3ybts1eFSC7QN...ToKK4, slot 407265372
        // Before fix: panicked with "index out of bounds: the len is 3 but the index is 3"
        let program_ids = vec![
            pubkey(COMPUTE_BUDGET),      // SetComputeUnitLimit
            pubkey(COMPUTE_BUDGET),      // SetComputeUnitPrice
            pubkey(LIGHT_SYSTEM),        // Light system invoke
            pubkey(SYSTEM_PROGRAM),      // SOL transfer (inner)
            pubkey(SYSTEM_PROGRAM),      // SOL transfer (inner)
            pubkey(SYSTEM_PROGRAM),      // SOL transfer (inner)
            pubkey(ACCOUNT_COMPRESSION), // InsertIntoQueues (inner)
        ];

        let instructions: Vec<Vec<u8>> = vec![
            ix_data("K1FDJ7"),
            ix_data("3cDeqiGMb6md"),
            ix_data("7Xu3JKNhcxBjvH52amHsaGu55uKzfsGvVjkBKAcEAAByDYGHt2TQQRq8aam17wkuH3Vtu2xuLyh8nZRxaqTEZPKM88CTs2e9MMiHW1ZA2NmFwbtgeHLFSRvW2DCayZMqHZWGEPjKwXnEJjFfKCTiJDXLeHbqirZeH4M3rYpeudpPnbNH9F9vLchjWs73hKJ9aSLVJKnJNXyr6ZW4hZd8YKVk3jaS11oW2ndPQT8CzYAF79wu8uishgqpLN42RwytWTDpMNUq7mKRFj1LkKKnpv8ya9WRxDCCKHfp1zn8sc1YviTcMyFsDRBvnE7kibyhcvd6hY9PvojPZNWABNDHxMGZoUL8xoUNRiD8Fxk7DyWQBvtqwyoPSjgFmKA97yEp4Kvj8btYDP24t51GYYXZyKjfFHnShcmdoKxuGohShW1UdjAhSWMVySZ92KRXjVJm6uv7CD5uXRy5Kuqco9ZHwASTv6HE1fQCEWKDdvq8Nx8SBMZF9jPM8JKJEarj"),
            ix_data("3Bxs4HHMpGEM2775"),
            ix_data("3Bxs4PckVVt51W8w"),
            ix_data("3Bxs4PckVVt51W8w"),
            ix_data("42NS6uhgPkAU4qDJGz54pXVoPKYL4VENq5jdLryg8pPRKsdthWiNYkaBQEimb4SSscjPZ2uYSXD7TjANLcaUdRMjh7Hid94o5GpGTxM3Pg2ALYdg8Qps6w2Sn6FXc1cp2vWVaXFQicExxLSTUNSSZwKH2M2XiqDxZBSekyELNcXkJCji9heVWqiB48zJX1YDBMYKLgXu3MoFvUgGjpYRteuuw44rBYUSfrs5tNh5CdfMtNkUJVCEvr5LSWeRUYwwXT8shx53iYb186vE3Gm2qY1Up7PfHdqGH1KZmzNz6ZjU2oC2r6zUHxoAA4v7HhMiC2cgwFXMrVGnw2nfKunjEP7Xm2Q62G4uJHGH3aMucTrSKCiwc55czqV9RaUDZUrvtfbLUjwG7XcPwwaY9JusFs21sZNveGE9xm1groM6uGn8ERCc6oBtFhouRKpfQiGoWxKeSrS6K5KWEq5aJ7XsZcXkNSdNGsGtgGu4nDXDtGhbamhXUtVmXcEfMMsMfoSzm1Cj1HCP89thHHC6P52Wert8XAfeei8X8bfwRHw6SzVFTBKkP7W8vjE2PgjwD5rVprBxS5owL4HPEnuTdSoawLA5JEqucpqgvXv7qihuJZ5aEQ8q2JhayJx3hqDriN6g1Vc2br8MtGRPXuwQYAd84jJoS6puMoanPnyFccv35jaxkEwUi5vY8J88ejut9W4uP7JVBivLBXYgDyLteffxA5a6rhJtFZ"),
        ];

        let accounts: Vec<Vec<Pubkey>> = vec![
            vec![], // ComputeBudget
            vec![], // ComputeBudget
            // Light system invoke: user, registered PDA, noop, CPI context, account compression,
            // SOL pool, system program, V2 trees (bmt3, oq3), V1 trees (smt5, nfq5), V2 trees (bmt2, oq2)
            vec![
                pubkey(USER),
                pubkey(USER),
                pubkey(REGISTERED_PDA),
                pubkey(NOOP),
                pubkey(CPI_CONTEXT),
                pubkey(ACCOUNT_COMPRESSION),
                pubkey(SOL_POOL),
                pubkey(USER),
                pubkey(SYSTEM_PROGRAM),
                pubkey(BMT3_TREE),
                pubkey(OQ3_QUEUE),
                pubkey(SMT5_TREE),
                pubkey(NFQ5_QUEUE),
                pubkey(BMT2_TREE),
                pubkey(OQ2_QUEUE),
            ],
            // SOL transfers (inner)
            vec![pubkey(SOL_POOL), pubkey(USER)],
            vec![pubkey(USER), pubkey(BMT3_TREE)],
            vec![pubkey(USER), pubkey(SMT5_TREE)],
            // InsertIntoQueues (inner): CPI context, registered PDA, queues and trees
            vec![
                pubkey(CPI_CONTEXT),
                pubkey(REGISTERED_PDA),
                pubkey(OQ3_QUEUE),
                pubkey(BMT3_TREE),
                pubkey(NFQ5_QUEUE),
                pubkey(SMT5_TREE),
                pubkey(OQ2_QUEUE),
                pubkey(BMT2_TREE),
            ],
        ];

        let result = event_from_light_transaction(&program_ids, &instructions, accounts);
        assert!(
            result.is_ok(),
            "event_from_light_transaction failed: {:?}",
            result.err()
        );
    }
}
