use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    Rpc, RpcError,
};
use solana_sdk::{
    bpf_loader_upgradeable,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Create mock program data account
pub fn create_mock_program_data(authority: Pubkey) -> Vec<u8> {
    let mut data = vec![0u8; 1024];
    data[0..4].copy_from_slice(&3u32.to_le_bytes());
    data[4..12].copy_from_slice(&0u64.to_le_bytes());
    data[12] = 1;
    data[13..45].copy_from_slice(authority.as_ref());
    data
}

/// For testing without ledger, LiteSVM does not create program data accounts,
/// so we need to do it manually.
pub fn setup_mock_program_data(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
) -> Pubkey {
    let (program_data_pda, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);
    let mock_data = create_mock_program_data(payer.pubkey());
    let mock_account = solana_sdk::account::Account {
        lamports: 1_000_000,
        data: mock_data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    };
    rpc.set_account(program_data_pda, mock_account);
    program_data_pda
}

/// Helper function to initialize config
pub async fn initialize_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    config_pda: Pubkey,
    program_data_pda: Pubkey,
    authority: &Keypair,
    compression_delay: u32,
    rent_recipient: Pubkey,
    address_space: Vec<Pubkey>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    if address_space.is_empty() {
        return Err(RpcError::CustomError(
            "At least one address space must be provided".to_string(),
        ));
    }

    let accounts = anchor_compressible_user::accounts::InitializeConfig {
        payer: payer.pubkey(),
        config: config_pda,
        program_data: program_data_pda,
        authority: authority.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let instruction_data = anchor_compressible_user::instruction::InitializeConfig {
        compression_delay,
        rent_recipient,
        address_space,
    };
    let instruction = Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };

    let signers = if payer.pubkey() == authority.pubkey() {
        vec![payer]
    } else {
        vec![payer, authority]
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}
