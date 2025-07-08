// #![cfg(feature = "test-sbf")]

use std::assert_eq;

use anchor_lang::prelude::borsh::BorshSerialize;
use anchor_spl::token_2022::spl_token_2022;
use light_compressed_token::mint_to_compressed::instructions::{
    CompressedMintInput, CompressedMintInputs, MintToCompressedInstructionData, Recipient,
};

use anchor_lang::{prelude::AccountMeta, solana_program::program_pack::Pack, system_program};

use light_client::indexer::Indexer;

use light_program_test::{LightProgramTest, ProgramTestConfig};

use light_sdk::instruction::ValidityProof;
use light_test_utils::Rpc;
use light_verifier::CompressedProof;
use serial_test::serial;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};

struct MultiTransferInput {
    payer: Pubkey,
    current_owner: Pubkey,
    new_recipient: Pubkey,
    mint: Pubkey,
    input_amount: u64,
    transfer_amount: u64,
    input_lamports: u64,
    transfer_lamports: u64,
    change_lamports: u64,
    leaf_index: u32,
    merkle_tree: Pubkey,
    output_queue: Pubkey,
}

fn create_multi_transfer_instruction(input: &MultiTransferInput) -> Instruction {
    // Create input token data
    let input_token_data =
        light_compressed_token::multi_transfer::instruction_data::MultiInputTokenDataWithContext {
            amount: input.input_amount,
            merkle_context: light_sdk::instruction::PackedMerkleContext {
                merkle_tree_pubkey_index: 0, // Index for merkle tree in remaining accounts
                queue_pubkey_index: 1,       // Index for output queue in remaining accounts
                leaf_index: input.leaf_index,
                prove_by_index: true,
            },
            root_index: 0,
            mint: 2,  // Index in remaining accounts
            owner: 3, // Index in remaining accounts
            with_delegate: false,
            delegate: 0, // Unused
        };

    // Create output token data
    let output_token_data =
        light_compressed_token::multi_transfer::instruction_data::MultiTokenTransferOutputData {
            owner: 4, // Index for new recipient in remaining accounts
            amount: input.transfer_amount,
            merkle_tree: 1, // Index for output queue in remaining accounts
            delegate: 0,    // No delegate
            mint: 2,        // Same mint index
        };

    // Create multi-transfer instruction data
    let multi_transfer_data = light_compressed_token::multi_transfer::instruction_data::CompressedTokenInstructionDataMultiTransfer {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        proof: None,
        in_token_data: vec![input_token_data],
        out_token_data: vec![output_token_data],
        in_lamports: Some(vec![input.input_lamports]), // Include input lamports
        out_lamports: Some(vec![input.transfer_lamports]), // Include output lamports
        in_tlv: None,
        out_tlv: None,
        compressions: None,
        cpi_context: None,
    };

    // Create multi-transfer accounts in the correct order expected by processor
    let multi_transfer_accounts = vec![
        // Light system program account (index 0) - skipped in processor
        AccountMeta::new_readonly(light_system_program::ID, false), // 0: light_system_program (skipped)
        // System accounts for multi-transfer (exact order from processor)
        AccountMeta::new(input.payer, true), // 1: fee_payer (signer, mutable)
        AccountMeta::new_readonly(
            light_compressed_token::process_transfer::get_cpi_authority_pda().0,
            false,
        ), // 2: authority (CPI authority PDA, signer via CPI)
        AccountMeta::new_readonly(
            light_system_program::utils::get_registered_program_pda(&light_system_program::ID),
            false,
        ), // 3: registered_program_pda
        AccountMeta::new_readonly(
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
            false,
        ), // 4: noop_program
        AccountMeta::new_readonly(
            light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID),
            false,
        ), // 5: account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // 6: account_compression_program
        AccountMeta::new_readonly(light_compressed_token::ID, false), // 7: invoking_program (self_program)
        // No sol_pool_pda since we don't have SOL decompression
        // No sol_decompression_recipient since we don't have SOL decompression
        AccountMeta::new_readonly(system_program::ID, false), // 8: system_program
        // No cpi_context_account since we don't use CPI context
        // Remaining accounts for token transfer - trees and queues FIRST for CPI
        AccountMeta::new(input.merkle_tree, false), // 9: merkle tree (index 0 in remaining)
        AccountMeta::new(input.output_queue, false), // 10: output queue (index 1 in remaining)
        AccountMeta::new_readonly(input.mint, false), // 11: mint (index 2 in remaining)
        AccountMeta::new_readonly(input.current_owner, true), // 12: current owner (index 3 in remaining) - must be signer
        AccountMeta::new_readonly(input.new_recipient, false), // 13: new recipient (index 4 in remaining)
    ];

    Instruction {
        program_id: light_compressed_token::ID,
        accounts: multi_transfer_accounts,
        data: [vec![104], multi_transfer_data.try_to_vec().unwrap()].concat(), // 104 is MultiTransfer discriminator
    }
}

fn derive_ctoken_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint.as_ref(),
        ],
        &light_compressed_token::ID,
    )
}

fn create_ctoken_ata_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> (Instruction, Pubkey) {
    let (ctoken_ata_pubkey, bump) = derive_ctoken_ata(owner, mint);

    use light_compressed_account::Pubkey as LightPubkey;
    use light_compressed_token::create_associated_token_account::instruction_data::CreateAssociatedTokenAccountInstructionData;

    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        owner: LightPubkey::from(owner.to_bytes()),
        mint: LightPubkey::from(mint.to_bytes()),
        bump,
    };

    let mut instruction_data_bytes = vec![103u8];
    instruction_data_bytes.extend_from_slice(&instruction_data.try_to_vec().unwrap());

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(ctoken_ata_pubkey, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*owner, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let create_ata_instruction = solana_sdk::instruction::Instruction {
        program_id: light_compressed_token::ID,
        accounts,
        data: instruction_data_bytes,
    };

    (create_ata_instruction, ctoken_ata_pubkey)
}

fn create_decompress_instruction(
    proof: ValidityProof,
    compressed_token_account: &[light_client::indexer::TokenAccount],
    decompress_amount: u64,
    spl_token_account: Pubkey,
    payer: Pubkey,
    output_queue: Pubkey,
) -> Instruction {
    // Process all input token accounts
    let mut in_token_data = Vec::with_capacity(8);
    let mut in_lamports = Vec::with_capacity(8);
    let mut total_amount = 0u64;

    // Calculate account indices dynamically
    let merkle_tree_index = 0;
    let output_queue_index = 1;
    let mint_index = 2;
    let owner_index = 3;
    let spl_token_account_index = 4;

    for account in compressed_token_account {
        total_amount += account.token.amount;

        in_token_data.push(
            light_compressed_token::multi_transfer::instruction_data::MultiInputTokenDataWithContext {
                amount: account.token.amount,
                merkle_context: light_sdk::instruction::PackedMerkleContext {
                    merkle_tree_pubkey_index: merkle_tree_index,
                    queue_pubkey_index: output_queue_index,
                    leaf_index: account.account.leaf_index,
                    prove_by_index: true,
                },
                root_index: 0,
                mint: mint_index,
                owner: owner_index,
                with_delegate: false,
                delegate: 0,
            }
        );

        in_lamports.push(account.account.lamports);
    }

    let remaining_amount = total_amount - decompress_amount;

    // Get merkle tree from first account
    let merkle_tree = compressed_token_account[0].account.tree_info.tree;

    // Create output token data for remaining compressed tokens (if any)
    let mut out_token_data = Vec::new();
    let mut out_lamports = Vec::new();

    if remaining_amount > 0 {
        out_token_data.push(
            light_compressed_token::multi_transfer::instruction_data::MultiTokenTransferOutputData {
                owner: owner_index,
                amount: remaining_amount,
                merkle_tree: output_queue_index,
                delegate: 0,
                mint: mint_index,
            }
        );
        out_lamports.push(compressed_token_account[0].account.lamports);
    }

    // Create compression data for decompression
    let compression_data = light_compressed_token::multi_transfer::instruction_data::Compression {
        amount: decompress_amount,
        is_compress: false, // This is decompression
        mint: mint_index,
        source_or_recipient: spl_token_account_index,
    };

    let multi_transfer_data = light_compressed_token::multi_transfer::instruction_data::CompressedTokenInstructionDataMultiTransfer {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0, // Index of output queue
        lamports_change_account_owner_index: 0, // Index of owner
        proof: None,
        in_token_data,
        out_token_data,
        in_lamports: if in_lamports.is_empty() { None } else { Some(in_lamports) },
        out_lamports: if out_lamports.is_empty() { None } else { Some(out_lamports) },
        in_tlv: None,
        out_tlv: None,
        compressions: Some(vec![compression_data]),
        cpi_context: None,
    };

    let multi_transfer_accounts = vec![
        AccountMeta::new_readonly(light_system_program::ID, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(
            light_compressed_token::process_transfer::get_cpi_authority_pda().0,
            false,
        ),
        AccountMeta::new_readonly(
            light_system_program::utils::get_registered_program_pda(&light_system_program::ID),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
            false,
        ),
        AccountMeta::new_readonly(
            light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID),
            false,
        ),
        AccountMeta::new_readonly(account_compression::ID, false),
        AccountMeta::new_readonly(light_compressed_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        // Tree accounts
        AccountMeta::new(merkle_tree, false),  // 0: merkle tree
        AccountMeta::new(output_queue, false), // 1: output queue
        AccountMeta::new_readonly(compressed_token_account[0].token.mint, false), // 2: mint
        AccountMeta::new_readonly(compressed_token_account[0].token.owner, true), // 3: current owner (signer)
        AccountMeta::new(spl_token_account, false), // 4: SPL token account for decompression
    ];

    Instruction {
        program_id: light_compressed_token::ID,
        accounts: multi_transfer_accounts,
        data: [vec![104], multi_transfer_data.try_to_vec().unwrap()].concat(),
    }
}

fn create_compressed_mint(
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    proof: CompressedProof,
    mint_bump: u8,
    address_merkle_tree_root_index: u16,
    mint_signer: Pubkey,
    payer: Pubkey,
    address_tree_pubkey: Pubkey,
    output_queue: Pubkey,
) -> Instruction {
    let instruction_data =
        light_compressed_token::mint::instructions::CreateCompressedMintInstructionData {
            decimals,
            mint_authority: mint_authority.into(),
            freeze_authority: freeze_authority.map(|auth| auth.into()),
            proof,
            mint_bump,
            address_merkle_tree_root_index,
        };

    let accounts = vec![
        // Static non-CPI accounts first
        AccountMeta::new_readonly(mint_signer, true), // 0: mint_signer (signer)
        AccountMeta::new_readonly(light_system_program::ID, false), // light system program
        // CPI accounts in exact order expected by execute_cpi_invoke
        AccountMeta::new(payer, true), // 1: fee_payer (signer, mutable)
        AccountMeta::new_readonly(
            light_compressed_token::process_transfer::get_cpi_authority_pda().0,
            false,
        ), // 2: cpi_authority_pda
        AccountMeta::new_readonly(
            light_system_program::utils::get_registered_program_pda(&light_system_program::ID),
            false,
        ), // 3: registered_program_pda
        AccountMeta::new_readonly(
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
            false,
        ), // 4: noop_program
        AccountMeta::new_readonly(
            light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID),
            false,
        ), // 5: account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // 6: account_compression_program
        AccountMeta::new_readonly(light_compressed_token::ID, false), // 7: invoking_program (self_program)
        // AccountMeta::new_readonly(light_system_program::ID, false),   // 8: sol_pool_pda placeholder
        // AccountMeta::new_readonly(light_system_program::ID, false),   // 9: decompression_recipient
        AccountMeta::new_readonly(system_program::ID, false), // 10: system_program
        // AccountMeta::new_readonly(light_system_program::ID, false), // 11: cpi_context_account placeholder
        AccountMeta::new(address_tree_pubkey, false), // 12: address_merkle_tree (mutable)
        AccountMeta::new(output_queue, false),        // 13: output_queue (mutable)
    ];

    Instruction {
        program_id: light_compressed_token::ID,
        accounts,
        data: [vec![100], instruction_data.try_to_vec().unwrap()].concat(),
    }
}

#[tokio::test]
#[serial]
async fn test_create_compressed_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_authority = mint_authority_keypair.pubkey();
    let freeze_authority = Pubkey::new_unique();
    let mint_signer = Keypair::new();

    // Get address tree for creating compressed mint address
    let address_tree_pubkey = rpc.get_address_merkle_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;
    let state_merkle_tree = rpc.get_random_state_tree_info().unwrap().tree;

    // Find mint PDA and bump
    let (mint_pda, mint_bump) = Pubkey::find_program_address(
        &[b"compressed_mint", mint_signer.pubkey().as_ref()],
        &light_compressed_token::ID,
    );

    // Use the mint PDA as the seed for the compressed account address
    let address_seed = mint_pda.to_bytes();

    let compressed_mint_address = light_compressed_account::address::derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &light_compressed_token::ID.to_bytes(),
    );

    // Get validity proof for address creation
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_program_test::AddressWithTree {
                address: compressed_mint_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let address_merkle_tree_root_index = rpc_result.addresses[0].root_index;

    // Create instruction
    let instruction = create_compressed_mint(
        decimals,
        mint_authority,
        Some(freeze_authority),
        rpc_result.proof.0.unwrap(),
        mint_bump,
        address_merkle_tree_root_index,
        mint_signer.pubkey(),
        payer.pubkey(),
        address_tree_pubkey,
        output_queue,
    );

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &mint_signer])
        .await
        .unwrap();

    // Verify the compressed mint was created
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    // Create expected compressed mint for comparison
    let expected_compressed_mint = light_compressed_token::create_mint::CompressedMint {
        spl_mint: mint_pda,
        supply: 0,
        decimals,
        is_decompressed: false,
        mint_authority: Some(mint_authority),
        freeze_authority: Some(freeze_authority),
        num_extensions: 0,
    };

    // Verify the account exists and has correct properties
    assert_eq!(
        compressed_mint_account.address.unwrap(),
        compressed_mint_address
    );
    assert_eq!(compressed_mint_account.owner, light_compressed_token::ID);
    assert_eq!(compressed_mint_account.lamports, 0);

    // Verify the compressed mint data
    let compressed_account_data = compressed_mint_account.data.unwrap();
    assert_eq!(
        compressed_account_data.discriminator,
        light_compressed_token::constants::COMPRESSED_MINT_DISCRIMINATOR
    );

    // Deserialize and verify the CompressedMint struct matches expected
    let actual_compressed_mint: light_compressed_token::create_mint::CompressedMint =
        anchor_lang::AnchorDeserialize::deserialize(&mut compressed_account_data.data.as_slice())
            .unwrap();

    assert_eq!(actual_compressed_mint, expected_compressed_mint);

    // Test mint_to_compressed functionality
    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;
    let lamports = Some(10000u64);

    // Get state tree for output token accounts
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();
    let state_tree_pubkey = state_tree_info.tree;
    let state_output_queue = state_tree_info.queue;
    println!("state_tree_pubkey {:?}", state_tree_pubkey);
    println!("state_output_queue {:?}", state_output_queue);

    // Prepare compressed mint inputs for minting
    let compressed_mint_inputs = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Will be set in remaining accounts
            queue_pubkey_index: 1,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: 0,
        address: compressed_mint_address,
        compressed_mint_input: CompressedMintInput {
            spl_mint: expected_compressed_mint.spl_mint.into(),
            supply: expected_compressed_mint.supply, // Current supply
            decimals: expected_compressed_mint.decimals,
            is_decompressed: expected_compressed_mint.is_decompressed, // Pure compressed mint
            freeze_authority_is_set: expected_compressed_mint.freeze_authority.is_some(),
            freeze_authority: expected_compressed_mint
                .freeze_authority
                .unwrap_or_default()
                .into(),
            num_extensions: 0,
        },
        output_merkle_tree_index: 3,
    };

    // Create mint_to_compressed instruction
    let mint_to_instruction_data = MintToCompressedInstructionData {
        compressed_mint_inputs,
        lamports,
        recipients: vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        proof: None, // No proof needed for this test
    };

    // Create accounts in the correct order for manual parsing
    let mint_to_accounts = vec![
        // Static non-CPI accounts first
        AccountMeta::new_readonly(mint_authority, true), // 0: authority (signer)
        // AccountMeta::new(mint_pda, false),               // 1: mint (mutable)
        // AccountMeta::new(Pubkey::new_unique(), false), // 2: token_pool_pda (mutable)
        // AccountMeta::new_readonly(spl_token::ID, false), // 3: token_program
        AccountMeta::new_readonly(light_system_program::ID, false), // 4: light_system_program
        // CPI accounts in exact order expected by InvokeCpiWithReadOnly
        AccountMeta::new(payer.pubkey(), true), // 5: fee_payer (signer, mutable)
        AccountMeta::new_readonly(
            light_compressed_token::process_transfer::get_cpi_authority_pda().0,
            false,
        ), // 6: cpi_authority_pda
        AccountMeta::new_readonly(
            light_system_program::utils::get_registered_program_pda(&light_system_program::ID),
            false,
        ), // 7: registered_program_pda
        AccountMeta::new_readonly(
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
            false,
        ), // 8: noop_program
        AccountMeta::new_readonly(
            light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID),
            false,
        ), // 9: account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // 10: account_compression_program
        AccountMeta::new_readonly(light_compressed_token::ID, false), // 11: self_program
        AccountMeta::new(light_system_program::utils::get_sol_pool_pda(), false), // 12: sol_pool_pda (mutable)
        AccountMeta::new_readonly(Pubkey::default(), false), // 13: system_program
        AccountMeta::new(state_merkle_tree, false),          // 14: mint_merkle_tree (mutable)
        AccountMeta::new(output_queue, false),               // 15: mint_in_queue (mutable)
        AccountMeta::new(output_queue, false),               // 16: mint_out_queue (mutable)
        AccountMeta::new(output_queue, false),               // 17: tokens_out_queue (mutable)
    ];
    println!("mint_to_accounts {:?}", mint_to_accounts);
    println!("output_queue {:?}", output_queue);
    println!("output_queue {:?}", output_queue);
    println!(
        "light_system_program::utils::get_sol_pool_pda() {:?}",
        light_system_program::utils::get_sol_pool_pda()
    );

    let mut mint_instruction = Instruction {
        program_id: light_compressed_token::ID,
        accounts: mint_to_accounts,
        data: [vec![101], mint_to_instruction_data.try_to_vec().unwrap()].concat(),
    };

    // Add remaining accounts: compressed mint's address tree, then output state tree
    mint_instruction.accounts.extend_from_slice(&[
        AccountMeta::new(state_tree_pubkey, false), // Compressed mint's queue
    ]);

    // Execute mint_to_compressed
    // Note: We need the mint authority to sign since it's the authority for minting
    rpc.create_and_send_transaction(
        &[mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    // Verify minted token account
    let token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        token_accounts.len(),
        1,
        "Should have exactly one token account"
    );
    let token_account = &token_accounts[0].token;
    assert_eq!(
        token_account.mint, mint_pda,
        "Token account should have correct mint"
    );
    assert_eq!(
        token_account.amount, mint_amount,
        "Token account should have correct amount"
    );
    assert_eq!(
        token_account.owner, recipient,
        "Token account should have correct owner"
    );

    // Verify updated compressed mint supply
    let updated_compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    let updated_compressed_mint: light_compressed_token::create_mint::CompressedMint =
        anchor_lang::AnchorDeserialize::deserialize(
            &mut updated_compressed_mint_account
                .data
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

    assert_eq!(
        updated_compressed_mint.supply, mint_amount,
        "Compressed mint supply should be updated to match minted amount"
    );

    // Test create_spl_mint functionality
    println!("Creating SPL mint for the compressed mint...");

    // Find token pool PDA and bump
    let (token_pool_pda, token_pool_bump) =
        light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index(
            &mint_pda, 0,
        );

    // Prepare compressed mint inputs for create_spl_mint
    let compressed_mint_inputs_for_spl = CompressedMintInputs {
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: 0, // Will be set in remaining accounts
            queue_pubkey_index: 1,
            leaf_index: updated_compressed_mint_account.leaf_index,
            prove_by_index: true,
        },
        root_index: address_merkle_tree_root_index,
        address: compressed_mint_address,
        compressed_mint_input: CompressedMintInput {
            spl_mint: mint_pda.into(),
            supply: mint_amount, // Current supply after minting
            decimals,
            is_decompressed: false, // Not yet decompressed
            freeze_authority_is_set: true,
            freeze_authority: freeze_authority.into(),
            num_extensions: 0,
        },
        output_merkle_tree_index: 2,
    };

    // Create create_spl_mint instruction data using the non-anchor pattern
    let create_spl_mint_instruction_data =
        light_compressed_token::create_spl_mint::instructions::CreateSplMintInstructionData {
            mint_bump,
            token_pool_bump,
            decimals,
            mint_authority: mint_authority.into(),
            freeze_authority: Some(freeze_authority.into()),
            compressed_mint_inputs: compressed_mint_inputs_for_spl,
            proof: None, // No proof needed for this test
        };

    // Build accounts manually for non-anchor instruction (following account order from accounts.rs)
    let create_spl_mint_accounts = vec![
        // Static non-CPI accounts first
        AccountMeta::new_readonly(mint_authority, true), // 0: authority
        AccountMeta::new(mint_pda, false),               // 1: mint
        AccountMeta::new_readonly(mint_signer.pubkey(), false), // 2: mint_signer
        AccountMeta::new(token_pool_pda, false),         // 3: token_pool_pda
        AccountMeta::new_readonly(spl_token_2022::ID, false), // 4: token_program
        AccountMeta::new_readonly(light_system_program::ID, false), // 5: light_system_program
        // CPI accounts in exact order expected by light-system-program
        AccountMeta::new(payer.pubkey(), true), // 5: fee_payer
        AccountMeta::new_readonly(
            light_compressed_token::process_transfer::get_cpi_authority_pda().0,
            false,
        ), // 6: cpi_authority_pda
        AccountMeta::new_readonly(
            light_system_program::utils::get_registered_program_pda(&light_system_program::ID),
            false,
        ), // 7: registered_program_pda
        AccountMeta::new_readonly(
            Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
            false,
        ), // 8: noop_program
        AccountMeta::new_readonly(
            light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID),
            false,
        ), // 9: account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // 10: account_compression_program
        AccountMeta::new_readonly(light_compressed_token::ID, false), // 11: self_program
        AccountMeta::new_readonly(system_program::ID, false),      // 13: system_program
        AccountMeta::new(state_merkle_tree, false),                // 14: in_merkle_tree
        AccountMeta::new(output_queue, false),                     // 15: in_output_queue
        AccountMeta::new(output_queue, false),                     // 16: out_output_queue
    ];
    println!("create_spl_mint_accounts {:?}", create_spl_mint_accounts);

    let mut create_spl_mint_instruction = Instruction {
        program_id: light_compressed_token::ID,
        accounts: create_spl_mint_accounts,
        data: [
            vec![102],
            create_spl_mint_instruction_data.try_to_vec().unwrap(),
        ]
        .concat(), // 102 = CreateSplMint discriminator
    };

    // Add remaining accounts (address tree for compressed mint updates)
    create_spl_mint_instruction.accounts.extend_from_slice(&[
        AccountMeta::new(address_tree_pubkey, false), // Address tree for compressed mint
    ]);

    // Execute create_spl_mint
    rpc.create_and_send_transaction(
        &[create_spl_mint_instruction],
        &payer.pubkey(),
        &[&payer, &mint_authority_keypair],
    )
    .await
    .unwrap();

    // Verify SPL mint was created
    let mint_account_data = rpc.get_account(mint_pda).await.unwrap().unwrap();
    let spl_mint = spl_token_2022::state::Mint::unpack(&mint_account_data.data).unwrap();
    assert_eq!(
        spl_mint.decimals, decimals,
        "SPL mint should have correct decimals"
    );
    assert_eq!(
        spl_mint.supply, mint_amount,
        "SPL mint should have minted supply"
    );
    assert_eq!(
        spl_mint.mint_authority.unwrap(),
        mint_authority,
        "SPL mint should have correct authority"
    );

    // Verify token pool was created and has the supply
    let token_pool_account_data = rpc.get_account(token_pool_pda).await.unwrap().unwrap();
    let token_pool = spl_token_2022::state::Account::unpack(&token_pool_account_data.data).unwrap();
    assert_eq!(
        token_pool.mint, mint_pda,
        "Token pool should have correct mint"
    );
    assert_eq!(
        token_pool.amount, mint_amount,
        "Token pool should have the minted supply"
    );

    // Verify compressed mint is now marked as decompressed
    let final_compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    let final_compressed_mint: light_compressed_token::create_mint::CompressedMint =
        anchor_lang::AnchorDeserialize::deserialize(
            &mut final_compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .unwrap();

    assert!(
        final_compressed_mint.is_decompressed,
        "Compressed mint should now be marked as decompressed"
    );

    // Test decompression functionality
    println!("Testing token decompression...");

    // Create SPL token account for the recipient
    let recipient_token_keypair = Keypair::new(); // Create keypair for token account
    light_test_utils::spl::create_token_2022_account(
        &mut rpc,
        &mint_pda,
        &recipient_token_keypair,
        &payer,
        true, // token_22
    )
    .await
    .unwrap();

    // Get the compressed token account for decompression
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_token_accounts.len(),
        1,
        "Should have one compressed token account"
    );
    let _input_compressed_account = compressed_token_accounts[0].clone();

    // Decompress half of the tokens (500 out of 1000)
    let _decompress_amount = mint_amount / 2;
    let _output_merkle_tree_pubkey = state_tree_pubkey;

    // Since we need a keypair to sign, and tokens were minted to a pubkey, let's skip decompression test for now
    // and just verify the basic create_spl_mint functionality worked
    println!("✅ SPL mint creation and token pool setup completed successfully!");
    println!(
        "Note: Decompression test skipped - would need token owner keypair to sign transaction"
    );

    // The SPL mint and token pool have been successfully created and verified
    println!("✅ create_spl_mint test completed successfully!");
    println!("   - SPL mint created with supply: {}", mint_amount);
    println!("   - Token pool created with balance: {}", mint_amount);
    println!(
        "   - Compressed mint marked as decompressed: {}",
        final_compressed_mint.is_decompressed
    );

    // Add a simple multi-transfer test: 1 input -> 1 output
    println!("🔄 Testing multi-transfer...");

    let new_recipient_keypair = Keypair::new();
    let new_recipient = new_recipient_keypair.pubkey();
    let transfer_amount = mint_amount; // Transfer all tokens (1000)

    let input_lamports = token_accounts[0].account.lamports; // Get the lamports from the token account
    let transfer_lamports = (input_lamports * transfer_amount) / mint_amount; // Proportional lamports transfer
    let change_lamports = 0; // No change in lamports since we're transferring proportionally
    println!("owner {:?}", recipient);
    let multi_transfer_input = MultiTransferInput {
        payer: payer.pubkey(),
        current_owner: recipient,
        new_recipient,
        mint: mint_pda,
        input_amount: mint_amount,
        transfer_amount,
        input_lamports,
        transfer_lamports,
        change_lamports,
        leaf_index: token_accounts[0].account.leaf_index,
        merkle_tree: state_tree_pubkey,
        output_queue: state_output_queue,
    };

    let multi_transfer_instruction = create_multi_transfer_instruction(&multi_transfer_input);
    println!(
        "Multi-transfer instruction: {:?}",
        multi_transfer_instruction.accounts
    );
    // Execute the multi-transfer instruction
    rpc.create_and_send_transaction(
        &[multi_transfer_instruction],
        &payer.pubkey(),
        &[&payer, &recipient_keypair], // Both payer and recipient need to sign
    )
    .await
    .unwrap();

    // Verify the transfer was successful
    let new_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&new_recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        new_token_accounts.len(),
        1,
        "New recipient should have exactly one token account"
    );
    assert_eq!(
        new_token_accounts[0].token.amount, transfer_amount,
        "New recipient should have the transferred amount"
    );
    assert_eq!(
        new_token_accounts[0].token.mint, mint_pda,
        "New recipient token should have correct mint"
    );

    println!("✅ Multi-transfer executed successfully!");
    println!(
        "   - Transferred {} tokens from {} to {}",
        transfer_amount, recipient, new_recipient
    );

    let compressed_token_account = &new_token_accounts[0];
    let decompress_amount = 300u64;
    let remaining_amount = transfer_amount - decompress_amount;

    // Get the output queue from the token account's tree info
    let output_queue = compressed_token_account.account.tree_info.queue;

    // Create compressed token associated token account for decompression
    let (ctoken_ata_pubkey, _bump) = derive_ctoken_ata(&new_recipient, &mint_pda);
    let (create_ata_instruction, _) =
        create_ctoken_ata_instruction(&payer.pubkey(), &new_recipient, &mint_pda);
    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Get validity proof for the compressed token account
    let validity_proof = rpc
        .get_validity_proof(vec![compressed_token_account.account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Create decompression instruction using the wrapper
    let decompress_instruction = create_decompress_instruction(
        validity_proof.proof,
        std::slice::from_ref(compressed_token_account),
        decompress_amount,
        ctoken_ata_pubkey,
        payer.pubkey(),
        output_queue,
    );

    println!("🔓 Sending decompression transaction...");
    println!("   - Decompress amount: {}", decompress_amount);
    println!("   - Remaining amount: {}", remaining_amount);
    println!("   - SPL token account: {}", ctoken_ata_pubkey);
    println!(" metas {:?}", decompress_instruction.accounts);
    // Send the decompression transaction
    let tx_result = rpc
        .create_and_send_transaction(
            &[decompress_instruction],
            &payer.pubkey(),
            &[&payer, &new_recipient_keypair],
        )
        .await;

    match tx_result {
        Ok(_) => {
            println!("✅ Decompression transaction sent successfully!");

            // Verify the decompression worked
            let ctoken_account = rpc.get_account(ctoken_ata_pubkey).await.unwrap().unwrap();

            let token_account =
                spl_token_2022::state::Account::unpack(&ctoken_account.data).unwrap();
            println!("   - CToken ATA balance: {}", token_account.amount);

            // Assert that the token account contains the expected decompressed amount
            assert_eq!(
                token_account.amount, decompress_amount,
                "Token account should contain exactly the decompressed amount"
            );

            // Check remaining compressed tokens
            let remaining_compressed = rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(&new_recipient, None, None)
                .await
                .unwrap()
                .value
                .items;

            if !remaining_compressed.is_empty() {
                println!(
                    "   - Remaining compressed tokens: {}",
                    remaining_compressed[0].token.amount
                );
            }
        }
        Err(e) => {
            println!("❌ Decompression transaction failed: {:?}", e);
            panic!("Decompression transaction failed");
        }
    }
}

/// Creates a `InitializeAccount3` instruction.
pub fn initialize_account3(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<solana_sdk::instruction::Instruction, anchor_lang::prelude::ProgramError> {
    let data = spl_token_2022::instruction::TokenInstruction::InitializeAccount3 {
        owner: *owner_pubkey,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
    ];

    Ok(solana_sdk::instruction::Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

/// Creates a `CloseAccount` instruction.
pub fn close_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<solana_sdk::instruction::Instruction, anchor_lang::prelude::ProgramError> {
    let data = spl_token_2022::instruction::TokenInstruction::CloseAccount.pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, true), // signer
    ];

    Ok(solana_sdk::instruction::Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

#[tokio::test]
async fn test_create_and_close_token_account() {
    use spl_pod::bytemuck::pod_from_bytes;
    use spl_token_2022::pod::PodAccount;
    use spl_token_2022::state::AccountState;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey (we don't need actual mint for this test)
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Create a new keypair for the token account
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    // First create the account using system program
    let create_account_system_ix = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &token_account_pubkey,
        rpc.get_minimum_balance_for_rent_exemption(165)
            .await
            .unwrap(), // SPL token account size
        165,
        &light_compressed_token::ID, // Our program owns the account
    );

    // Then use SPL token SDK format but with our compressed token program ID
    // This tests that our create_token_account instruction is compatible with SPL SDKs
    let initialize_account_ix = initialize_account3(
        &light_compressed_token::ID, // Use our program ID instead of spl_token_2022::ID
        &token_account_pubkey,
        &mint_pubkey,
        &owner_pubkey,
    )
    .unwrap();

    // Execute both instructions in one transaction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[create_account_system_ix, initialize_account_ix],
        Some(&payer_pubkey),
        &[&payer, &token_account_keypair],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create token account using SPL SDK");

    // Verify the token account was created correctly
    let account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify account exists and has correct owner
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert_eq!(account_info.data.len(), 165); // SPL token account size

    let pod_account = pod_from_bytes::<PodAccount>(&account_info.data)
        .expect("Failed to parse token account data");

    // Verify the token account fields
    assert_eq!(Pubkey::from(pod_account.mint), mint_pubkey);
    assert_eq!(Pubkey::from(pod_account.owner), owner_pubkey);
    assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
    assert_eq!(pod_account.state, AccountState::Initialized as u8);

    // Now test closing the account using SPL SDK format
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Airdrop some lamports to destination account so it exists
    rpc.context.airdrop(&destination_pubkey, 1_000_000).unwrap();

    // Get initial lamports before closing
    let initial_token_account_lamports = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let initial_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Create close account instruction using SPL SDK format
    let close_account_ix = close_account(
        &light_compressed_token::ID,
        &token_account_pubkey,
        &destination_pubkey,
        &owner_pubkey,
    )
    .unwrap();

    // Execute the close instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let close_transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[close_account_ix],
        Some(&payer_pubkey),
        &[&payer, &owner_keypair], // Need owner to sign
        blockhash,
    );

    rpc.process_transaction(close_transaction)
        .await
        .expect("Failed to close token account using SPL SDK");

    // Verify the account was closed (data should be cleared, lamports should be 0)
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(account) = closed_account {
        // Account still exists, but should have 0 lamports and cleared data
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // Verify lamports were transferred to destination
    let final_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        final_destination_lamports,
        initial_destination_lamports + initial_token_account_lamports,
        "Destination should receive all lamports from closed account"
    );
}

#[tokio::test]
async fn test_create_associated_token_account() {
    use spl_pod::bytemuck::pod_from_bytes;
    use spl_token_2022::pod::PodAccount;
    use spl_token_2022::state::AccountState;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mock mint pubkey
    let mint_pubkey = Pubkey::new_unique();

    // Create owner for the associated token account
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // Calculate the expected associated token account address
    let (expected_ata_pubkey, bump) = Pubkey::find_program_address(
        &[
            owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );

    // Build the create_associated_token_account instruction
    use light_compressed_account::Pubkey as LightPubkey;
    use light_compressed_token::create_associated_token_account::instruction_data::CreateAssociatedTokenAccountInstructionData;

    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        owner: LightPubkey::from(owner_pubkey.to_bytes()),
        mint: LightPubkey::from(mint_pubkey.to_bytes()),
        bump,
    };

    let mut instruction_data_bytes = vec![103u8]; // CreateAssociatedTokenAccount discriminator
    instruction_data_bytes.extend_from_slice(&instruction_data.try_to_vec().unwrap());

    // Create the accounts for the instruction
    let accounts = vec![
        AccountMeta::new(payer_pubkey, true), // fee_payer (signer)
        AccountMeta::new(expected_ata_pubkey, false), // associated_token_account
        AccountMeta::new_readonly(mint_pubkey, false), // mint
        AccountMeta::new_readonly(owner_pubkey, false), // owner
        AccountMeta::new_readonly(system_program::ID, false), // system_program
    ];

    let instruction = solana_sdk::instruction::Instruction {
        program_id: light_compressed_token::ID,
        accounts,
        data: instruction_data_bytes,
    };

    // Execute the instruction
    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );

    rpc.process_transaction(transaction.clone())
        .await
        .expect("Failed to create associated token account");

    // Verify the associated token account was created correctly
    let account_info = rpc.get_account(expected_ata_pubkey).await.unwrap().unwrap();

    // Verify account exists and has correct owner
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert_eq!(account_info.data.len(), 165); // SPL token account size

    let pod_account = pod_from_bytes::<PodAccount>(&account_info.data)
        .expect("Failed to parse token account data");

    // Verify the token account fields
    assert_eq!(Pubkey::from(pod_account.mint), mint_pubkey);
    assert_eq!(Pubkey::from(pod_account.owner), owner_pubkey);
    assert_eq!(u64::from(pod_account.amount), 0); // Should start with zero balance
    assert_eq!(pod_account.state, AccountState::Initialized as u8);

    // Verify the PDA derivation is correct
    let (derived_ata_pubkey, derived_bump) = Pubkey::find_program_address(
        &[
            owner_pubkey.as_ref(),
            light_compressed_token::ID.as_ref(),
            mint_pubkey.as_ref(),
        ],
        &light_compressed_token::ID,
    );
    assert_eq!(expected_ata_pubkey, derived_ata_pubkey);
    assert_eq!(bump, derived_bump);
}
