#[derive(Debug, Clone, PartialEq)]
pub struct TransactionParams {
    pub num_input_compressed_accounts: u8,
    pub num_output_compressed_accounts: u8,
    pub num_new_addresses: u8,
    pub compress: i64,
    pub fee_config: FeeConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeeConfig {
    pub state_merkle_tree_rollover: u64,
    pub nullifier_queue_rollover: u64,
    pub address_queue_rollover: u64,
    pub network_fee: u64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            state_merkle_tree_rollover: 149,
            nullifier_queue_rollover: 29,
            address_queue_rollover: 181,
            network_fee: 1,
        }
    }
}
