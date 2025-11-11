use anchor_compressed_token::ErrorCode;
use anchor_lang::AnchorSerialize;
use light_account_checks::{
    account_info::test_account_info::pinocchio::get_account_info,
    packed_accounts::ProgramPackedAccounts,
};
use light_compressed_token::transfer2::{
    accounts::Transfer2Accounts, compression::ctoken::close_for_compress_and_close,
};
use light_ctoken_types::{
    instructions::transfer2::{Compression, CompressionMode},
    state::{CToken, CompressedTokenConfig},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};
use pinocchio::pubkey::Pubkey;

/// Helper to create valid compressible CToken account data
fn create_compressible_ctoken_data(
    owner_pubkey: &[u8; 32],
    rent_sponsor_pubkey: &[u8; 32],
) -> Vec<u8> {
    // Create config for compressible CToken (no delegate, not native, no close_authority)
    let config = CompressedTokenConfig::new_compressible(false, false, false);

    // Calculate required size
    let size = CToken::byte_len(&config).unwrap();
    let mut data = vec![0u8; size];

    // Initialize using zero-copy new
    let (mut ctoken, _) = CToken::new_zero_copy(&mut data, config).unwrap();

    // Set required fields using to_bytes/to_bytes_mut methods
    *ctoken.mint = light_compressed_account::Pubkey::from([0u8; 32]);
    *ctoken.owner = light_compressed_account::Pubkey::from(*owner_pubkey);
    *ctoken.state = 1; // AccountState::Initialized

    // Set compressible extension fields
    if let Some(extensions) = ctoken.extensions.as_mut() {
        if let Some(light_ctoken_types::state::ZExtensionStructMut::Compressible(comp_ext)) =
            extensions.first_mut()
        {
            comp_ext.config_account_version.set(1);
            comp_ext.account_version = 3; // ShaFlat
            comp_ext.compression_authority.copy_from_slice(owner_pubkey);
            comp_ext.rent_sponsor.copy_from_slice(rent_sponsor_pubkey);
            comp_ext.last_claimed_slot.set(0);
        }
    }

    data
}

/// Test that close_for_compress_and_close detects duplicate compressed account indices
#[test]
fn test_close_for_compress_and_close_duplicate_detection() {
    // Create two CompressAndClose compressions with the SAME compressed_account_index (0)
    let compressions = vec![
        Compression {
            mode: CompressionMode::CompressAndClose,
            amount: 500,
            mint: 0,
            source_or_recipient: 0, // token_account index
            authority: 1,
            pool_account_index: 2, // rent_sponsor index
            pool_index: 0,         // DUPLICATE: compressed_account_index = 0
            bump: 3,               // destination index
        },
        Compression {
            mode: CompressionMode::CompressAndClose,
            amount: 300,
            mint: 0,
            source_or_recipient: 4, // different token_account index
            authority: 1,
            pool_account_index: 2, // rent_sponsor index
            pool_index: 0,         // DUPLICATE: compressed_account_index = 0 (SAME AS FIRST!)
            bump: 3,               // destination index
        },
    ];

    // Serialize to bytes
    let compression_bytes = compressions.try_to_vec().unwrap();

    // Convert to zero-copy slice
    let (compressions_zc, _) = Vec::<Compression>::zero_copy_at(&compression_bytes).unwrap();

    // Create mock account infos (we need at least 5 accounts for indices 0-4)
    let owner_pubkey_bytes = [1u8; 32];
    let rent_sponsor_pubkey_bytes = [2u8; 32];
    let owner_pubkey = Pubkey::from(owner_pubkey_bytes);
    let rent_sponsor_pubkey = Pubkey::from(rent_sponsor_pubkey_bytes);
    let dummy_owner = [0u8; 32];

    // Create valid compressible CToken account data using zero-copy initialization
    let ctoken_data =
        create_compressible_ctoken_data(&owner_pubkey_bytes, &rent_sponsor_pubkey_bytes);

    let accounts = vec![
        get_account_info(
            owner_pubkey,
            dummy_owner,
            false,
            true,
            false,
            ctoken_data.clone(),
        ), // index 0: token_account (writable)
        get_account_info(owner_pubkey, dummy_owner, true, false, false, vec![]), // index 1: authority (signer)
        get_account_info(rent_sponsor_pubkey, dummy_owner, false, true, false, vec![]), // index 2: rent_sponsor (writable)
        get_account_info(
            Pubkey::from([3u8; 32]),
            dummy_owner,
            false,
            true,
            false,
            vec![],
        ), // index 3: destination (writable)
        get_account_info(owner_pubkey, dummy_owner, false, true, false, ctoken_data), // index 4: second token_account (writable)
    ];

    let packed_accounts = ProgramPackedAccounts {
        accounts: &accounts,
    };

    // Create minimal Transfer2Accounts
    let validated_accounts = Transfer2Accounts {
        system: None,
        write_to_cpi_context_system: None,
        compressions_only_fee_payer: None,
        compressions_only_cpi_authority_pda: None,
        packed_accounts,
    };

    // Call the function - should detect duplicate and return error
    let result = close_for_compress_and_close(&compressions_zc, &validated_accounts);

    // Assert we got the expected error
    match result {
        Err(anchor_lang::prelude::ProgramError::Custom(code)) => {
            assert_eq!(
                code,
                ErrorCode::CompressAndCloseDuplicateOutput as u32,
                "Expected CompressAndCloseDuplicateOutput error, got error code: {}",
                code
            );
        }
        Err(e) => panic!(
            "Expected CompressAndCloseDuplicateOutput error, got different error type: {:?}",
            e
        ),
        Ok(_) => panic!("Expected CompressAndCloseDuplicateOutput error, but function succeeded!"),
    }
}
