use anchor_lang::{pubkey, InstructionData, ToAccountMetas};
use borsh::BorshDeserialize;
use light_client::rpc::{Rpc, RpcError};
use light_compressible::{config::CompressibleConfig, rent::RentConfig};
use light_registry::utils::get_protocol_config_pda_address;
use solana_pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::LightProgramTest;

/// Helper function to create CompressibleConfig
pub async fn create_compressible_config(
    rpc: &mut LightProgramTest,
) -> Result<(Pubkey, Pubkey, Pubkey), RpcError> {
    let payer = rpc.get_payer().insecure_clone();
    let registry_program_id = solana_sdk::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
    let governance_authority = rpc
        .test_accounts
        .protocol
        .governance_authority
        .insecure_clone();
    // First, create the config counter if it doesn't exist
    let (config_counter_pda, _counter_bump) =
        Pubkey::find_program_address(&[b"compressible_config_counter"], &registry_program_id);
    let protocol_config_pda = get_protocol_config_pda_address().0;

    // Check if counter exists, if not create it
    if rpc.get_account(config_counter_pda).await?.is_none() {
        let instruction_data = light_registry::instruction::CreateConfigCounter {}; // Create config instruction

        // Create counter instruction
        let create_counter_ix = solana_sdk::instruction::Instruction {
            program_id: registry_program_id,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer.pubkey(), true),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    rpc.test_accounts.protocol.governance_authority.pubkey(),
                    true,
                ), // authority
                solana_sdk::instruction::AccountMeta::new_readonly(protocol_config_pda, false),
                solana_sdk::instruction::AccountMeta::new(config_counter_pda, false),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    solana_sdk::system_program::id(),
                    false,
                ),
            ],
            data: instruction_data.data(), // create_config_counter discriminator
        };
        let governance_authority = rpc
            .test_accounts
            .protocol
            .governance_authority
            .insecure_clone();
        rpc.create_and_send_transaction(
            &[create_counter_ix],
            &payer.pubkey(),
            &[&payer, &governance_authority],
        )
        .await?;
    }

    // Now create the config with version 1
    let version: u16 = 1;
    let (compressible_config_pda, config_bump) = Pubkey::find_program_address(
        &[b"compressible_config", &version.to_le_bytes()],
        &registry_program_id,
    );

    let instruction_data = light_registry::instruction::CreateCompressibleConfig {
        rent_config: RentConfig::default(),
        update_authority: payer.pubkey(),
        withdrawal_authority: payer.pubkey(),
        active: true,
    }; // Create config instruction

    let accounts = light_registry::accounts::CreateCompressibleConfig {
        fee_payer: payer.pubkey(),
        authority: governance_authority.pubkey(),
        system_program: Pubkey::default(),
        compressible_config: compressible_config_pda,
        protocol_config_pda,
        config_counter: config_counter_pda,
    };
    let create_config_ix = solana_sdk::instruction::Instruction {
        program_id: registry_program_id,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(), // create_compressible_config discriminator
    };

    rpc.create_and_send_transaction(
        &[create_config_ix],
        &payer.pubkey(),
        &[&payer, &governance_authority],
    )
    .await?;
    let compressible_config_account = rpc
        .get_account(compressible_config_pda)
        .await
        .unwrap()
        .unwrap();

    let (rent_sponsor, rent_sponsor_bump) = Pubkey::find_program_address(
        &[b"rent_sponsor".as_slice(), version.to_le_bytes().as_slice()],
        &pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
    );

    let (compression_authority, compression_authority_bump) = Pubkey::find_program_address(
        &[
            b"compression_authority".as_slice(),
            version.to_le_bytes().as_slice(),
        ],
        &registry_program_id,
    );

    let mut address_space = [Pubkey::default(); 4];
    address_space[0] = pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK");

    // Fund the rent_sponsor PDA so it can act as a fee payer in CPIs
    // This PDA needs funds to pay for account creation
    rpc.airdrop_lamports(&rent_sponsor, 1_000_000_000)
        .await
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to fund rent_sponsor: {:?}", e)))?;

    let expected_config_account = CompressibleConfig {
        version,
        state: 1, // true as u8
        bump: config_bump,
        update_authority: payer.pubkey(),
        withdrawal_authority: payer.pubkey(),
        rent_sponsor,
        compression_authority,
        rent_sponsor_bump,
        compression_authority_bump,
        rent_config: RentConfig::default(),
        address_space,
        _place_holder: [0u8; 32],
    };

    // Check the discriminator is correct
    assert_eq!(
        compressible_config_account.data[0..8],
        [1, 2, 3, 4, 5, 6, 7, 8]
    );

    // Deserialize and verify the account
    let deserialized_account =
        CompressibleConfig::deserialize(&mut &compressible_config_account.data[8..]).unwrap();
    println!("deserialized_account {:?}", deserialized_account);
    println!(
        "AAA pt compressible_config_pda {:?}",
        compressible_config_pda
    );
    assert_eq!(expected_config_account, deserialized_account);

    // Return config PDA, rent_sponsor, and compression_authority
    Ok((compressible_config_pda, rent_sponsor, compression_authority))
}
