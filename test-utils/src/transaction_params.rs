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
    pub address_queue_rollover: u64,
    // TODO: refactor to allow multiple state and address tree configs
    // pub state_tree_configs: Vec<StateMerkleTreeConfig>,
    // pub address_tree_configs: Vec<AddressMerkleTreeConfig>,
    pub network_fee: u64,
    pub address_network_fee: u64,
    pub solana_network_fee: i64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            // rollover fee plus additonal lamports for the cpi account
            state_merkle_tree_rollover: 300,
            address_queue_rollover: 392,
            // TODO: refactor to allow multiple state and address tree configs
            // state_tree_configs: vec![StateMerkleTreeConfig::default()],
            // address_tree_configs: vec![AddressMerkleTreeConfig::default()],
            network_fee: 5000,
            address_network_fee: 5000,
            solana_network_fee: 5000,
        }
    }
}
