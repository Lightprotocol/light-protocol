use anchor_lang::{InstructionData, ToAccountMetas};
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    Rpc, RpcError,
};
use light_sdk::compressible::{CompressibleConfig, CompressibleInstruction};
use solana_sdk::{
    bpf_loader_upgradeable,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};

// /// Mid-level instruction builders for anchor_compressible_user program
// /// Following Solana SDK patterns like system_instruction::transfer()
// pub struct CompressibleUserInstruction;

// impl CompressibleUserInstruction {
//     /// Creates an initialize config instruction
//     ///
//     /// Returns a ready-to-use Instruction that can be added to a transaction
//     pub fn initialize_compression_config(
//         program_id: &Pubkey,
//         payer: &Pubkey,
//         authority: &Pubkey,
//         compression_delay: u32,
//         rent_recipient: Pubkey,
//         address_space: Vec<Pubkey>,
//     ) -> Instruction {
//         let (config_pda, _) = CompressibleConfig::derive_pda(program_id);
//         let (program_data_pda, _) =
//             Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::ID);

//         let accounts = anchor_compressible_user::accounts::InitializeCompressionConfig {
//             payer: *payer,
//             config: config_pda,
//             program_data: program_data_pda,
//             authority: *authority,
//             system_program: system_program::ID,
//         };

//         let instruction_data = anchor_compressible_user::instruction::InitializeCompressionConfig {
//             compression_delay,
//             rent_recipient,
//             address_space,
//         };

//         Instruction {
//             program_id: *program_id,
//             accounts: accounts.to_account_metas(None),
//             data: instruction_data.data(),
//         }
//     }

//     /// Creates an update config instruction
//     ///
//     /// Returns a ready-to-use Instruction that can be added to a transaction
//     pub fn update_config(
//         program_id: &Pubkey,
//         authority: &Pubkey,
//         new_compression_delay: Option<u32>,
//         new_rent_recipient: Option<Pubkey>,
//         new_address_space: Option<Vec<Pubkey>>,
//         new_update_authority: Option<Pubkey>,
//     ) -> Instruction {
//         let (config_pda, _) = CompressibleConfig::derive_pda(program_id);

//         let accounts = anchor_compressible_user::accounts::UpdateCompressionConfig {
//             config: config_pda,
//             authority: *authority,
//         };

//         let instruction_data = anchor_compressible_user::instruction::UpdateCompressionConfig {
//             new_compression_delay,
//             new_rent_recipient,
//             new_address_space,
//             new_update_authority,
//         };

//         Instruction {
//             program_id: *program_id,
//             accounts: accounts.to_account_metas(None),
//             data: instruction_data.data(),
//         }
//     }
// }

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

/// Helper function to initialize config using mid-level instruction builder
pub async fn initialize_compression_config(
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

    // Use the mid-level instruction builder - much cleaner!
    let instruction = CompressibleInstruction::initialize_compression_config(
        program_id,
        &payer.pubkey(),
        &authority.pubkey(),
        compression_delay,
        rent_recipient,
        address_space,
    );

    let signers = if payer.pubkey() == authority.pubkey() {
        vec![payer]
    } else {
        vec![payer, authority]
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &signers)
        .await
}

/// Helper function to update config using mid-level instruction builder
pub async fn update_compression_config(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    program_id: &Pubkey,
    authority: &Keypair,
    new_compression_delay: Option<u32>,
    new_rent_recipient: Option<Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    // Use the mid-level instruction builder
    let instruction = CompressibleInstruction::update_compression_config(
        program_id,
        &authority.pubkey(),
        new_compression_delay,
        new_rent_recipient,
        new_address_space,
        new_update_authority,
    );

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, authority])
        .await
}
