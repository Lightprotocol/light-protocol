use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token_sdk::instructions::find_spl_mint_address;
use light_ctoken_types::instructions::mint_action::Recipient;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    airdrop_lamports,
    assert_transfer2::{assert_transfer2, assert_transfer2_with_delegate},
};
use light_token_client::{
    actions::{create_mint, mint_to_compressed, transfer2},
    instructions::transfer2::{Transfer2InstructionType, TransferInput},
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
#[serial]
async fn test_transfer2_delegated_partial() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();
    // Test parameters
    let decimals = 6u8;
    let mint_authority_keypair = Keypair::new(); // Create keypair so we can sign
    let mint_seed = Keypair::new();
    // Find mint PDA for the rest of the test
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    create_mint(
        &mut rpc,
        &mint_seed,
        decimals,
        &mint_authority_keypair,
        None,
        None, // No metadata
        &payer,
    )
    .await
    .unwrap();

    let recipient_keypair = Keypair::new();
    let recipient = recipient_keypair.pubkey();
    let mint_amount = 1000u64;

    mint_to_compressed(
        &mut rpc,
        spl_mint_pda,
        vec![Recipient {
            recipient: recipient.into(),
            amount: mint_amount,
        }],
        &mint_authority_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Get the compressed token account
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, mint_amount);
    assert_eq!(compressed_accounts[0].token.delegate, None);

    // Create a delegate
    let delegate_keypair = Keypair::new();
    let delegate = delegate_keypair.pubkey();
    airdrop_lamports(&mut rpc, &delegate, 10_000_000_000)
        .await
        .unwrap();

    // Approve delegation using the new approve action
    let delegate_amount = 600u64;
    transfer2::approve(
        &mut rpc,
        &compressed_accounts,
        delegate,
        delegate_amount,
        &recipient_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Get updated compressed accounts after approval
    let compressed_accounts_after_approve = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    // Should have 2 accounts now: change account and delegated account
    assert_eq!(compressed_accounts_after_approve.len(), 2);

    // Find the delegated account
    let delegated_account = compressed_accounts_after_approve
        .iter()
        .find(|acc| acc.token.delegate == Some(delegate))
        .expect("Should find delegated account");

    assert_eq!(delegated_account.token.amount, delegate_amount);
    assert_eq!(delegated_account.token.delegate, Some(delegate));

    // Find the change account
    let change_account = compressed_accounts_after_approve
        .iter()
        .find(|acc| acc.token.delegate.is_none())
        .expect("Should find change account");

    assert_eq!(change_account.token.amount, mint_amount - delegate_amount);

    // Now delegate transfers partial amount using transfer2
    let transfer_recipient = Keypair::new().pubkey();
    let transfer_amount = 200u64;

    transfer2::transfer_delegated(
        &mut rpc,
        &[delegated_account.clone()],
        transfer_recipient,
        transfer_amount,
        &delegate_keypair,
        &payer,
    )
    .await
    .unwrap();

    // Verify the transfer using assert_transfer2_with_delegate
    assert_transfer2_with_delegate(
        &mut rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: &[delegated_account.clone()],
            to: transfer_recipient,
            amount: transfer_amount,
            is_delegate_transfer: true, // This was a delegate transfer
        })],
        Some(delegate),
    )
    .await;

    // Get the remaining delegated account after delegate's transfer
    // The change account should still have the delegate set
    let accounts_after_delegate = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient, None, None)
        .await
        .unwrap()
        .value
        .items;

    let remaining_delegated_account = accounts_after_delegate
        .into_iter()
        .find(|acc| {
            acc.token.delegate == Some(delegate)
                && acc.token.amount == (delegate_amount - transfer_amount)
        })
        .expect("Should find remaining delegated account with delegate still set");

    // Now have the OWNER transfer the remaining delegated tokens
    let owner_transfer_recipient = Keypair::new().pubkey();
    let owner_transfer_amount = 150u64;

    transfer2::transfer(
        &mut rpc,
        &[remaining_delegated_account.clone()],
        owner_transfer_recipient,
        owner_transfer_amount,
        &recipient_keypair, // Owner is signing
        &payer,
    )
    .await
    .unwrap();

    // Verify the owner's transfer
    assert_transfer2(
        &mut rpc,
        vec![Transfer2InstructionType::Transfer(TransferInput {
            compressed_token_account: &[remaining_delegated_account.clone()],
            to: owner_transfer_recipient,
            amount: owner_transfer_amount,
            is_delegate_transfer: false, // Owner is transferring, not delegate
        })],
    )
    .await;

    println!("✅ Test passed: Both delegate and owner can transfer delegated tokens!");
}
