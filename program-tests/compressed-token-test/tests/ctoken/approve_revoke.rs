//! Tests for CToken approve and revoke instructions
//!
//! Tests verify that approve and revoke work correctly for compressible
//! CToken accounts with extensions.

use borsh::BorshDeserialize;
use light_ctoken_interface::state::{
    AccountState, CToken, ExtensionStruct, PausableAccountExtension,
    PermanentDelegateAccountExtension, TokenDataVersion, TransferFeeAccountExtension,
    TransferHookAccountExtension, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use light_ctoken_sdk::ctoken::{
    ApproveCToken, CompressibleParams, CreateCTokenAccount, RevokeCToken,
};
use light_program_test::program_test::TestRpc;
use light_test_utils::{Rpc, RpcError};
use serial_test::serial;
use solana_sdk::{program_pack::Pack, signature::Keypair, signer::Signer};

use super::extensions::setup_extensions_test;

/// Test approve and revoke with a compressible CToken account with extensions.
/// 1. Create compressible CToken account with all extensions
/// 2. Set token balance to 100 using set_account
/// 3. Approve 10 tokens to delegate
/// 4. Assert delegate and delegated_amount fields
/// 5. Revoke delegation
/// 6. Assert delegate cleared and delegated_amount is 0
#[tokio::test]
#[serial]
async fn test_approve_revoke_compressible() -> Result<(), RpcError> {
    use anchor_spl::token_2022::spl_token_2022;

    let mut context = setup_extensions_test().await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let owner = Keypair::new();
    let delegate = Keypair::new();

    // 1. Create compressible CToken account with all extensions
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to create instruction: {}", e))
            })?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 2. Set token balance to 100 using set_account
    let token_balance = 100u64;
    let mut token_account_info = context
        .rpc
        .get_account(account_pubkey)
        .await?
        .ok_or_else(|| RpcError::AssertRpcError("Token account not found".to_string()))?;

    let mut spl_token_account =
        spl_token_2022::state::Account::unpack_unchecked(&token_account_info.data[..165])
            .map_err(|e| RpcError::AssertRpcError(format!("Failed to unpack: {:?}", e)))?;
    spl_token_account.amount = token_balance;
    spl_token_2022::state::Account::pack(spl_token_account, &mut token_account_info.data[..165])
        .map_err(|e| RpcError::AssertRpcError(format!("Failed to pack: {:?}", e)))?;
    context.rpc.set_account(account_pubkey, token_account_info);

    // Verify initial state
    let account_data_initial = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_initial = CToken::deserialize(&mut &account_data_initial.data[..])
        .expect("Failed to deserialize CToken");
    assert_eq!(ctoken_initial.amount, token_balance);
    assert!(ctoken_initial.delegate.is_none());
    assert_eq!(ctoken_initial.delegated_amount, 0);

    // Extract CompressionInfo for expected comparisons
    let compression_info = ctoken_initial
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Should have Compressible extension");

    // Fund the owner for compressible top-up
    context
        .rpc
        .airdrop_lamports(&owner.pubkey(), 1_000_000_000)
        .await?;

    // 3. Approve 10 tokens to delegate
    let approve_amount = 10u64;
    let approve_ix = ApproveCToken {
        token_account: account_pubkey,
        delegate: delegate.pubkey(),
        owner: owner.pubkey(),
        amount: approve_amount,
    }
    .instruction()
    .map_err(|e| {
        RpcError::AssertRpcError(format!("Failed to create approve instruction: {}", e))
    })?;

    context
        .rpc
        .create_and_send_transaction(&[approve_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 4. Assert delegate and delegated_amount fields after approve
    let account_data_approved = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_approved = CToken::deserialize(&mut &account_data_approved.data[..])
        .expect("Failed to deserialize CToken after approve");

    let expected_approved = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: token_balance,
        delegate: Some(delegate.pubkey().to_bytes().into()),
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: approve_amount,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
    };

    assert_eq!(
        ctoken_approved, expected_approved,
        "CToken after approve should have delegate set and delegated_amount=10"
    );

    // 5. Revoke delegation
    let revoke_ix = RevokeCToken {
        token_account: account_pubkey,
        owner: owner.pubkey(),
    }
    .instruction()
    .map_err(|e| RpcError::AssertRpcError(format!("Failed to create revoke instruction: {}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[revoke_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 6. Assert delegate cleared and delegated_amount is 0 after revoke
    let account_data_revoked = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_revoked = CToken::deserialize(&mut &account_data_revoked.data[..])
        .expect("Failed to deserialize CToken after revoke");

    let expected_revoked = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: token_balance,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
    };

    assert_eq!(
        ctoken_revoked, expected_revoked,
        "CToken after revoke should have delegate cleared and delegated_amount=0"
    );

    println!("Successfully tested approve and revoke with compressible CToken");
    Ok(())
}
