#[cfg(feature = "devenv")]
use {
    crate::rpc::{RpcConnection, RpcError},
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
};

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
            // rollover fee plus additional lamports for the cpi account
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

impl FeeConfig {
    pub fn test_batched() -> Self {
        Self {
            // rollover fee plus additional lamports for the cpi account
            state_merkle_tree_rollover: 1,
            address_queue_rollover: 392, // not batched
            network_fee: 5000,
            address_network_fee: 5000,
            solana_network_fee: 5000,
        }
    }
}

#[cfg(feature = "devenv")]
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionParams {
    pub num_input_compressed_accounts: u8,
    pub num_output_compressed_accounts: u8,
    pub num_new_addresses: u8,
    pub compress: i64,
    pub fee_config: FeeConfig,
}

#[cfg(feature = "devenv")]
pub async fn assert_transaction_params(
    rpc: &mut impl RpcConnection,
    payer: &Pubkey,
    signers: &[&Keypair],
    pre_balance: u64,
    params: Option<TransactionParams>,
) -> Result<(), RpcError> {
    if let Some(transaction_params) = params {
        let mut deduped_signers = signers.to_vec();
        deduped_signers.dedup();
        let post_balance = rpc.get_account(*payer).await?.unwrap().lamports;

        // a network_fee is charged if there are input compressed accounts or new addresses
        let mut network_fee: i64 = 0;
        if transaction_params.num_input_compressed_accounts != 0
            || transaction_params.num_output_compressed_accounts != 0
        {
            network_fee += transaction_params.fee_config.network_fee as i64;
        }
        if transaction_params.num_new_addresses != 0 {
            network_fee += transaction_params.fee_config.address_network_fee as i64;
        }
        let expected_post_balance = pre_balance as i64
            - i64::from(transaction_params.num_new_addresses)
                * transaction_params.fee_config.address_queue_rollover as i64
            - i64::from(transaction_params.num_output_compressed_accounts)
                * transaction_params.fee_config.state_merkle_tree_rollover as i64
            - transaction_params.compress
            - transaction_params.fee_config.solana_network_fee * deduped_signers.len() as i64
            - network_fee;

        if post_balance as i64 != expected_post_balance {
            println!("transaction_params: {:?}", transaction_params);
            println!("pre_balance: {}", pre_balance);
            println!("post_balance: {}", post_balance);
            println!("expected post_balance: {}", expected_post_balance);
            println!(
                "diff post_balance: {}",
                post_balance as i64 - expected_post_balance
            );
            println!(
                "rollover fee: {}",
                transaction_params.fee_config.state_merkle_tree_rollover
            );
            println!(
                "address_network_fee: {}",
                transaction_params.fee_config.address_network_fee
            );
            println!("network_fee: {}", network_fee);
            println!("num signers {}", deduped_signers.len());
            return Err(RpcError::CustomError("Transaction fee error.".to_string()));
        }
    }
    Ok(())
}
