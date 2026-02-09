use anchor_spl::token_2022::spl_token_2022;
use light_program_test::utils::assert::assert_rpc_error;
use solana_sdk::program_pack::Pack;

use super::shared::*;

/// Test delegate compress on CToken accounts.
///
/// Scenarios:
/// 1. Partial compress (300 of 500 delegated) — delegate and delegated_amount updated
/// 2. Exact remaining compress (200) — delegate cleared
/// 3. Delegate cannot compress after being cleared — OwnerMismatch error
/// 4. Owner can still compress normally after delegate is cleared
#[tokio::test]
#[serial]
async fn test_delegate_compress() -> Result<(), RpcError> {
    // Setup: CToken account with compressible extension
    let mut context = setup_account_test_with_created_account(Some((0, false))).await?;
    let payer = context.payer.insecure_clone();
    let owner = context.owner_keypair.insecure_clone();
    let token_account_pubkey = context.token_account_keypair.pubkey();
    let mint_pubkey = context.mint_pubkey;

    // Fund owner for transaction fees and compressible top-up
    context
        .rpc
        .airdrop_lamports(&owner.pubkey(), 1_000_000_000)
        .await?;

    // Set CToken balance to 1000 via set_account
    {
        let mut token_account = context
            .rpc
            .get_account(token_account_pubkey)
            .await?
            .unwrap();

        let mut spl_account =
            spl_token_2022::state::Account::unpack_unchecked(&token_account.data[..165]).unwrap();
        spl_account.amount = 1000;
        spl_token_2022::state::Account::pack(spl_account, &mut token_account.data[..165]).unwrap();
        context.rpc.set_account(token_account_pubkey, token_account);
    }

    // Approve delegate for 500
    let delegate = Keypair::new();
    approve_and_assert(&mut context, delegate.pubkey(), 500, "approve_delegate_500").await;

    // Warp slot so compressible top-up assertion works
    context.rpc.warp_to_slot(4).unwrap();

    let output_queue = context
        .rpc
        .get_random_state_tree_info()
        .unwrap()
        .get_output_pubkey()
        .unwrap();

    // =========================================================================
    // Scenario 1: Partial compress (300 of 500 delegated)
    // =========================================================================
    {
        compress(
            &mut context.rpc,
            token_account_pubkey,
            300,
            owner.pubkey(),
            &delegate,
            &payer,
            9,
        )
        .await
        .unwrap();

        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: owner.pubkey(),
            mint: mint_pubkey,
            amount: 300,
            authority: delegate.pubkey(),
            output_queue,
            pool_index: None,
            decimals: 9,
            version: None,
        };
        assert_transfer2_compress(&mut context.rpc, compress_input).await;

        // Verify: amount == 700, delegated_amount == 200, delegate still set
        let account_data = context
            .rpc
            .get_account(token_account_pubkey)
            .await?
            .unwrap();
        let spl_account =
            spl_token_2022::state::Account::unpack(&account_data.data[..165]).unwrap();
        assert_eq!(
            spl_account.amount, 700,
            "Balance should be 700 after compressing 300"
        );
        assert_eq!(
            spl_account.delegated_amount, 200,
            "Delegated amount should be 200 after compressing 300 of 500"
        );
        assert_eq!(
            spl_account.delegate,
            spl_token_2022::solana_program::program_option::COption::Some(delegate.pubkey()),
            "Delegate should still be set"
        );
    }

    // =========================================================================
    // Scenario 2: Exact remaining amount (200), delegate cleared
    // =========================================================================
    {
        compress(
            &mut context.rpc,
            token_account_pubkey,
            200,
            owner.pubkey(),
            &delegate,
            &payer,
            9,
        )
        .await
        .unwrap();

        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: owner.pubkey(),
            mint: mint_pubkey,
            amount: 200,
            authority: delegate.pubkey(),
            output_queue,
            pool_index: None,
            decimals: 9,
            version: None,
        };
        assert_transfer2_compress(&mut context.rpc, compress_input).await;

        // Verify: amount == 500, delegated_amount == 0, delegate cleared
        let account_data = context
            .rpc
            .get_account(token_account_pubkey)
            .await?
            .unwrap();
        let spl_account =
            spl_token_2022::state::Account::unpack(&account_data.data[..165]).unwrap();
        assert_eq!(
            spl_account.amount, 500,
            "Balance should be 500 after compressing 200 more"
        );
        assert_eq!(
            spl_account.delegated_amount, 0,
            "Delegated amount should be 0 after compressing all delegated tokens"
        );
        assert_eq!(
            spl_account.delegate,
            spl_token_2022::solana_program::program_option::COption::None,
            "Delegate should be cleared when delegated_amount reaches 0"
        );
    }

    // =========================================================================
    // Scenario 3: Delegate cannot compress after being cleared
    // =========================================================================
    {
        let result = compress(
            &mut context.rpc,
            token_account_pubkey,
            1,
            owner.pubkey(),
            &delegate,
            &payer,
            9,
        )
        .await;

        // OwnerMismatch = 6075
        assert_rpc_error(result, 0, 6075).unwrap();
    }

    // =========================================================================
    // Scenario 4: Owner can still compress normally
    // =========================================================================
    {
        compress(
            &mut context.rpc,
            token_account_pubkey,
            100,
            owner.pubkey(),
            &owner,
            &payer,
            9,
        )
        .await
        .unwrap();

        let compress_input = CompressInput {
            compressed_token_account: None,
            solana_token_account: token_account_pubkey,
            to: owner.pubkey(),
            mint: mint_pubkey,
            amount: 100,
            authority: owner.pubkey(),
            output_queue,
            pool_index: None,
            decimals: 9,
            version: None,
        };
        assert_transfer2_compress(&mut context.rpc, compress_input).await;

        // Verify: amount == 400
        let account_data = context
            .rpc
            .get_account(token_account_pubkey)
            .await?
            .unwrap();
        let spl_account =
            spl_token_2022::state::Account::unpack(&account_data.data[..165]).unwrap();
        assert_eq!(
            spl_account.amount, 400,
            "Balance should be 400 after owner compresses 100"
        );
    }

    Ok(())
}
