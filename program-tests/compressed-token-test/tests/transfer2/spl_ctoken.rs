use anchor_lang::prelude::{AccountMeta, ProgramError};
// Re-export all necessary imports for test modules
pub use anchor_spl::token_2022::spl_token_2022;
pub use light_compressed_token_sdk::ctoken::{
    derive_ctoken_ata, CompressibleParams, CreateAssociatedTokenAccount,
};
use light_compressed_token_sdk::{
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    token_pool::find_token_pool_pda_with_index,
    ValidityProof,
};
use light_ctoken_types::instructions::transfer2::{Compression, MultiTokenTransferOutputData};
use light_program_test::utils::assert::assert_rpc_error;
pub use light_program_test::{LightProgramTest, ProgramTestConfig};
pub use light_test_utils::{
    airdrop_lamports,
    spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens},
    Rpc, RpcError,
};
pub use light_token_client::actions::transfer2::{self};
use solana_sdk::pubkey::Pubkey;
pub use solana_sdk::{instruction::Instruction, signature::Keypair, signer::Signer};
pub use spl_token_2022::pod::PodAccount;

#[tokio::test]
async fn test_spl_to_ctoken_transfer() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(true, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();
    println!(
        "spl_token_account_keypair {:?}",
        spl_token_account_keypair.pubkey()
    );
    // Create recipient for compressed tokens
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create compressed token ATA for recipient
    let instruction = CreateAssociatedTokenAccount::new(
        payer.pubkey(),
        recipient.pubkey(),
        mint,
        CompressibleParams::default(),
    )
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let associated_token_account = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Get initial SPL token balance
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e)))
        .unwrap();
    let initial_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(initial_spl_balance, amount);

    // Use the new spl_to_ctoken_transfer action from light-token-client
    transfer2::spl_to_ctoken_transfer(
        &mut rpc,
        spl_token_account_keypair.pubkey(),
        associated_token_account,
        transfer_amount,
        &sender,
        &payer,
    )
    .await
    .unwrap();

    {
        // Verify SPL token balance decreased
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
            })
            .unwrap();
        let final_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(final_spl_balance, amount - transfer_amount);
    }
    {
        // Verify compressed token balance increased
        let spl_account_data = rpc
            .get_account(associated_token_account)
            .await
            .unwrap()
            .unwrap();
        let spl_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
                })
                .unwrap();
        assert_eq!(
            u64::from(spl_account.amount),
            transfer_amount,
            "Recipient should have {} compressed tokens",
            transfer_amount
        );
    }

    // Now transfer back from compressed token to SPL token account
    println!("Testing reverse transfer: ctoken to SPL");

    // Transfer from recipient's compressed token account back to sender's SPL token account
    transfer2::transfer_ctoken_to_spl(
        &mut rpc,
        associated_token_account,
        spl_token_account_keypair.pubkey(),
        transfer_amount,
        &recipient,
        mint,
        &payer,
    )
    .await
    .unwrap();

    // Verify final balances
    {
        // Verify SPL token balance is restored
        let spl_account_data = rpc
            .get_account(spl_token_account_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();
        let spl_account = spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data)
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to parse SPL token account: {}", e))
            })
            .unwrap();
        let restored_spl_balance: u64 = spl_account.amount.into();
        assert_eq!(
            restored_spl_balance, amount,
            "SPL token balance should be restored to original amount"
        );
    }

    {
        // Verify compressed token balance is now 0
        let ctoken_account_data = rpc
            .get_account(associated_token_account)
            .await
            .unwrap()
            .unwrap();
        let ctoken_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!(
                        "Failed to parse compressed token account: {}",
                        e
                    ))
                })
                .unwrap();
        assert_eq!(
            u64::from(ctoken_account.amount),
            0,
            "Compressed token account should be empty after transfer back"
        );
    }

    println!("Successfully completed round-trip transfer: SPL -> CToken -> SPL");
}

#[tokio::test]
async fn test_failing_ctoken_to_spl_with_compress_and_close() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(true, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account and mint tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Create recipient for compressed tokens
    let recipient = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create non-compressible token ATA for recipient (required for CompressAndClose without rent_sponsor)
    let (associated_token_account, bump) = derive_ctoken_ata(&recipient.pubkey(), &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: recipient.pubkey(),
        mint,
        associated_token_account,
        compressible: None,
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create ATA instruction: {}", e)))
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Transfer SPL to CToken
    transfer2::spl_to_ctoken_transfer(
        &mut rpc,
        spl_token_account_keypair.pubkey(),
        associated_token_account,
        transfer_amount,
        &sender,
        &payer,
    )
    .await
    .unwrap();

    // Verify compressed token balance after initial transfer
    {
        let ctoken_account_data = rpc
            .get_account(associated_token_account)
            .await
            .unwrap()
            .unwrap();
        let ctoken_account =
            spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165])
                .map_err(|e| {
                    RpcError::AssertRpcError(format!("Failed to parse CToken account: {}", e))
                })
                .unwrap();
        assert_eq!(
            u64::from(ctoken_account.amount),
            transfer_amount,
            "Recipient should have {} compressed tokens",
            transfer_amount
        );
    }

    // Now transfer back using CompressAndClose instead of regular transfer
    println!("Testing reverse transfer with CompressAndClose: ctoken to SPL");

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);

    let transfer_ix = CtokenToSplTransferAndClose {
        source_ctoken_account: associated_token_account,
        destination_spl_token_account: spl_token_account_keypair.pubkey(),
        amount: transfer_amount,
        authority: recipient.pubkey(),
        mint,
        payer: payer.pubkey(),
        token_pool_pda,
        token_pool_pda_bump,
        spl_token_program: anchor_spl::token::ID,
    }
    .instruction()
    .unwrap();

    // Execute transaction
    let result = rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &recipient])
        .await;
    assert_rpc_error(result, 0, 3).unwrap();
}

pub struct CtokenToSplTransferAndClose {
    pub source_ctoken_account: Pubkey,
    pub destination_spl_token_account: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub payer: Pubkey,
    pub token_pool_pda: Pubkey,
    pub token_pool_pda_bump: u8,
    pub spl_token_program: Pubkey,
}

impl CtokenToSplTransferAndClose {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let packed_accounts = vec![
            // Mint (index 0)
            AccountMeta::new_readonly(self.mint, false),
            // Source ctoken account (index 1) - writable
            AccountMeta::new(self.source_ctoken_account, false),
            // Destination SPL token account (index 2) - writable
            AccountMeta::new(self.destination_spl_token_account, false),
            // Authority (index 3) - signer
            AccountMeta::new(self.authority, true),
            // Token pool PDA (index 4) - writable
            AccountMeta::new(self.token_pool_pda, false),
            // SPL Token program (index 5) - needed for CPI
            AccountMeta::new_readonly(self.spl_token_program, false),
        ];

        // First operation: compress from ctoken account to pool using compress_and_close
        let compress_to_pool = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::compress_and_close_ctoken(
                self.amount,
                0, // mint index
                1, // source ctoken account index
                3, // authority index
                0, // no rent sponsor
                0, // no compressed account
                3, // destination is authority
            )),
            delegate_is_set: false,
            method_used: true,
        };

        // Second operation: decompress from pool to SPL token account using decompress_spl
        let decompress_to_spl = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(Compression::decompress_spl(
                self.amount,
                0, // mint index
                2, // destination SPL token account index
                4, // pool_account_index
                0, // pool_index (TODO: make dynamic)
                self.token_pool_pda_bump,
            )),
            delegate_is_set: false,
            method_used: true,
        };

        let inputs = Transfer2Inputs {
            validity_proof: ValidityProof::new(None),
            transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
            meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
                self.payer,
                packed_accounts,
            ),
            in_lamports: None,
            out_lamports: None,
            token_accounts: vec![compress_to_pool, decompress_to_spl],
            output_queue: 0, // Decompressed accounts only, no output queue needed
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
