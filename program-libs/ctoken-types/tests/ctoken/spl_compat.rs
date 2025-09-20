//! Tests ctoken solana account - spl token account layout compatibility
//!
//! Tests:
//! 1. test_compressed_token_equivalent_to_pod_account
//! 2. test_compressed_token_with_compressible_extension
//! 3. test_account_type_compatibility_with_spl_parsing

use light_compressed_account::Pubkey;
use light_ctoken_types::state::{
    ctoken::{CToken, CompressedTokenConfig, ZCToken},
    CompressionInfoConfig, ExtensionStructConfig,
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use rand::Rng;
use spl_pod::{bytemuck::pod_from_bytes, primitives::PodU64, solana_program_option::COption};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, PodStateWithExtensions, StateWithExtensions},
    pod::PodAccount,
    solana_program::program_pack::Pack,
    state::{Account, AccountState},
};

/// Generate random token account data using SPL Token's pack method
fn generate_random_token_account_data(rng: &mut impl Rng) -> Vec<u8> {
    let account = Account {
        mint: solana_pubkey::Pubkey::new_from_array(rng.gen::<[u8; 32]>()),
        owner: solana_pubkey::Pubkey::new_from_array(rng.gen::<[u8; 32]>()),
        amount: rng.gen::<u64>(),
        delegate: if rng.gen_bool(0.3) {
            COption::Some(solana_pubkey::Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            COption::None
        },
        state: if rng.gen_bool(0.9) {
            AccountState::Initialized
        } else {
            AccountState::Frozen
        },
        is_native: if rng.gen_bool(0.2) {
            COption::Some(rng.gen_range(1_000_000..=10_000_000u64))
        } else {
            COption::None
        },
        delegated_amount: rng.gen::<u64>(),
        close_authority: if rng.gen_bool(0.25) {
            COption::Some(solana_pubkey::Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            COption::None
        },
    };
    println!("Expected Account: {:?}", account);

    let mut account_data = vec![0u8; Account::LEN];
    Account::pack(account, &mut account_data).unwrap();
    account_data
}

/// Compare all fields between our CToken zero-copy implementation and Pod account
fn compare_compressed_token_with_pod_account(
    compressed_token: &ZCToken,
    pod_account: &PodAccount,
) -> bool {
    // Extensions should be None for basic SPL Token accounts
    if compressed_token.extensions.is_some() {
        return false;
    }

    // Compare mint
    if compressed_token.mint.to_bytes() != pod_account.mint.to_bytes() {
        println!(
            "Mint mismatch: compressed={:?}, pod={:?}",
            compressed_token.mint.to_bytes(),
            pod_account.mint.to_bytes()
        );
        return false;
    }

    // Compare owner
    if compressed_token.owner.to_bytes() != pod_account.owner.to_bytes() {
        return false;
    }

    // Compare amount
    if u64::from(*compressed_token.amount) != u64::from(pod_account.amount) {
        return false;
    }

    // Compare delegate
    let pod_delegate_option: Option<Pubkey> = if pod_account.delegate.is_some() {
        Some(
            pod_account
                .delegate
                .unwrap_or(solana_pubkey::Pubkey::default())
                .to_bytes()
                .into(),
        )
    } else {
        None
    };
    match (compressed_token.delegate, pod_delegate_option) {
        (Some(compressed_delegate), Some(pod_delegate)) => {
            if compressed_delegate.to_bytes() != pod_delegate.to_bytes() {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    // Compare state
    if compressed_token.state != pod_account.state {
        return false;
    }

    // Compare is_native
    let pod_native_option: Option<u64> = if pod_account.is_native.is_some() {
        Some(u64::from(
            pod_account.is_native.unwrap_or(PodU64::default()),
        ))
    } else {
        None
    };
    match (compressed_token.is_native, pod_native_option) {
        (Some(compressed_native), Some(pod_native)) => {
            if u64::from(*compressed_native) != pod_native {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    // Compare delegated_amount
    if u64::from(*compressed_token.delegated_amount) != u64::from(pod_account.delegated_amount) {
        return false;
    }

    // Compare close_authority
    let pod_close_option: Option<Pubkey> = if pod_account.close_authority.is_some() {
        Some(
            pod_account
                .close_authority
                .unwrap_or(solana_pubkey::Pubkey::default())
                .to_bytes()
                .into(),
        )
    } else {
        None
    };
    match (compressed_token.close_authority, pod_close_option) {
        (Some(compressed_close), Some(pod_close)) => {
            if compressed_close.to_bytes() != pod_close.to_bytes() {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    true
}

/// Compare all fields between our CToken mutable zero-copy implementation and Pod account
fn compare_compressed_token_mut_with_pod_account(
    compressed_token: &light_ctoken_types::state::ctoken::ZCompressedTokenMut,
    pod_account: &PodAccount,
) -> bool {
    // Extensions should be None for basic SPL Token accounts
    if compressed_token.extensions.is_some() {
        return false;
    }

    // Compare mint
    if compressed_token.mint.to_bytes() != pod_account.mint.to_bytes() {
        println!(
            "Mint mismatch: compressed={:?}, pod={:?}",
            compressed_token.mint.to_bytes(),
            pod_account.mint.to_bytes()
        );
        return false;
    }

    // Compare owner
    if compressed_token.owner.to_bytes() != pod_account.owner.to_bytes() {
        return false;
    }

    // Compare amount
    if u64::from(*compressed_token.amount) != u64::from(pod_account.amount) {
        return false;
    }

    // Compare delegate
    let pod_delegate_option: Option<Pubkey> = if pod_account.delegate.is_some() {
        Some(
            pod_account
                .delegate
                .unwrap_or(solana_pubkey::Pubkey::default())
                .to_bytes()
                .into(),
        )
    } else {
        None
    };
    match (compressed_token.delegate.as_ref(), pod_delegate_option) {
        (Some(compressed_delegate), Some(pod_delegate)) => {
            if compressed_delegate.to_bytes() != pod_delegate.to_bytes() {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    // Compare state
    if *compressed_token.state != pod_account.state {
        println!(
            "State mismatch: compressed={}, pod={}",
            *compressed_token.state, pod_account.state
        );
        return false;
    }

    // Compare is_native
    let pod_native_option: Option<u64> = if pod_account.is_native.is_some() {
        Some(u64::from(
            pod_account.is_native.unwrap_or(PodU64::default()),
        ))
    } else {
        None
    };
    match (compressed_token.is_native.as_ref(), pod_native_option) {
        (Some(compressed_native), Some(pod_native)) => {
            if u64::from(**compressed_native) != pod_native {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    // Compare delegated_amount
    if u64::from(*compressed_token.delegated_amount) != u64::from(pod_account.delegated_amount) {
        return false;
    }

    // Compare close_authority
    let pod_close_option: Option<Pubkey> = if pod_account.close_authority.is_some() {
        Some(
            pod_account
                .close_authority
                .unwrap_or(solana_pubkey::Pubkey::default())
                .to_bytes()
                .into(),
        )
    } else {
        None
    };
    match (compressed_token.close_authority.as_ref(), pod_close_option) {
        (Some(compressed_close), Some(pod_close)) => {
            if compressed_close.to_bytes() != pod_close.to_bytes() {
                return false;
            }
        }
        (None, None) => {
            // Both are None, which is correct
        }
        _ => {
            // One is Some, the other is None - mismatch
            return false;
        }
    }

    true
}

#[test]
fn test_compressed_token_equivalent_to_pod_account() {
    let mut rng = rand::thread_rng();

    for _ in 0..10000 {
        let mut account_data = generate_random_token_account_data(&mut rng);
        let account_data_clone = account_data.clone();
        let pod_account = pod_from_bytes::<PodAccount>(&account_data_clone).unwrap();

        // Test immutable version
        let (compressed_token, _) = CToken::zero_copy_at(&account_data).unwrap();
        println!("Compressed Token: {:?}", compressed_token);
        println!("Pod Account: {:?}", pod_account);
        assert!(compare_compressed_token_with_pod_account(
            &compressed_token,
            pod_account
        ));
        {
            let account_data_clone = account_data.clone();
            let pod_account = pod_from_bytes::<PodAccount>(&account_data_clone).unwrap();
            // Test mutable version
            let (mut compressed_token_mut, _) =
                CToken::zero_copy_at_mut(&mut account_data).unwrap();
            println!("Compressed Token Mut: {:?}", compressed_token_mut);
            println!("Pod Account: {:?}", pod_account);

            assert!(compare_compressed_token_mut_with_pod_account(
                &compressed_token_mut,
                pod_account
            ));

            // Test mutation: modify every mutable field in the zero-copy struct
            {
                // Modify mint (first 32 bytes)
                *compressed_token_mut.mint = solana_pubkey::Pubkey::new_unique().to_bytes().into();

                // Modify owner (next 32 bytes)
                *compressed_token_mut.owner = solana_pubkey::Pubkey::new_unique().to_bytes().into();
                // Modify amount
                *compressed_token_mut.amount = rng.gen::<u64>().into();

                // Modify delegate if it exists
                if let Some(ref mut delegate) = compressed_token_mut.delegate {
                    **delegate = solana_pubkey::Pubkey::new_unique().to_bytes().into();
                }

                // Modify state (0 = Uninitialized, 1 = Initialized, 2 = Frozen)
                *compressed_token_mut.state = rng.gen_range(0..=2);

                // Modify is_native if it exists
                if let Some(ref mut native_value) = compressed_token_mut.is_native {
                    **native_value = rng.gen::<u64>().into();
                }

                // Modify delegated_amount
                *compressed_token_mut.delegated_amount = rng.gen::<u64>().into();

                // Modify close_authority if it exists
                if let Some(ref mut close_auth) = compressed_token_mut.close_authority {
                    **close_auth = solana_pubkey::Pubkey::new_unique().to_bytes().into();
                }
            }
            // Clone the modified bytes and create a new Pod account to verify changes
            let modified_account_data = account_data.clone();
            let modified_pod_account =
                pod_from_bytes::<PodAccount>(&modified_account_data).unwrap();

            // Create a new immutable compressed token from the modified data to compare
            let (modified_compressed_token, _) =
                CToken::zero_copy_at(&modified_account_data).unwrap();

            println!("Modified zero copy account {:?}", modified_compressed_token);
            println!("Modified Pod Account: {:?}", modified_pod_account);
            // Use the comparison function to verify all modifications
            assert!(compare_compressed_token_with_pod_account(
                &modified_compressed_token,
                modified_pod_account
            ));
        }
    }
}

#[test]
fn test_compressed_token_with_compressible_extension() {
    use light_zero_copy::traits::ZeroCopyAtMut;

    // Test configuration with compressible extension
    let config = CompressedTokenConfig {
        delegate: false,
        is_native: false,
        close_authority: false,
        extensions: vec![ExtensionStructConfig::Compressible(CompressionInfoConfig {
            rent_config: (),
        })],
    };

    // Calculate required buffer size (165 base + 1 AccountType + 1 Option + extension data)
    let required_size = CToken::byte_len(&config).unwrap();
    println!(
        "Required size for compressible extension: {}",
        required_size
    );

    // Should be more than 165 bytes due to AccountType byte and extension
    assert!(required_size > 165);

    // Create buffer and initialize
    let mut buffer = vec![0u8; required_size];
    {
        let (compressed_token, remaining_bytes) = CToken::new_zero_copy(&mut buffer, config)
            .expect("Failed to initialize compressed token with compressible extension");

        // Verify the remaining bytes length
        assert_eq!(remaining_bytes.len(), 0);

        // Verify extensions are present
        assert!(compressed_token.extensions.is_some());
        let extensions = compressed_token.extensions.as_ref().unwrap();
        assert_eq!(extensions.len(), 1);
    } // Drop the compressed_token reference here

    // Now we can access buffer directly
    // Verify AccountType::Account byte is set at position 165
    assert_eq!(buffer[165], 2); // AccountType::Account = 2

    // Verify extension option discriminant at position 166
    assert_eq!(buffer[166], 1); // Some = 1

    // Test zero-copy deserialization round-trip
    let (deserialized_token, _) = CToken::zero_copy_at(&buffer)
        .expect("Failed to deserialize token with compressible extension");

    assert!(deserialized_token.extensions.is_some());
    let deserialized_extensions = deserialized_token.extensions.as_ref().unwrap();
    assert_eq!(deserialized_extensions.len(), 1);

    // Test mutable deserialization with a fresh buffer
    let mut buffer_copy = buffer.clone();
    let (mutable_token, _) = CToken::zero_copy_at_mut(&mut buffer_copy)
        .expect("Failed to deserialize mutable token with compressible extension");

    assert!(mutable_token.extensions.is_some());
}

#[test]
fn test_account_type_compatibility_with_spl_parsing() {
    // This test verifies our AccountType insertion makes accounts SPL Token 2022 compatible

    let config = CompressedTokenConfig {
        delegate: false,
        is_native: false,
        close_authority: false,
        extensions: vec![ExtensionStructConfig::Compressible(CompressionInfoConfig {
            rent_config: (),
        })],
    };

    let mut buffer = vec![0u8; CToken::byte_len(&config).unwrap()];
    let (_compressed_token, _) =
        CToken::new_zero_copy(&mut buffer, config).expect("Failed to create token with extension");

    let pod_account = pod_from_bytes::<PodAccount>(&buffer[..165])
        .expect("First 165 bytes should be valid SPL Token Account data");
    let pod_state = PodStateWithExtensions::<PodAccount>::unpack(&buffer)
        .expect("Pod account with extensions should succeed.");
    let base_account = pod_state.base;
    assert_eq!(pod_account, base_account);
    // Verify account structure
    assert_eq!(pod_account.state, 1); // AccountState::Initialized

    // Verify AccountType byte is at position 165
    assert_eq!(buffer[165], 2); // AccountType::Account = 2
                                // Deserialize with extensions
    let token_account_data = StateWithExtensions::<Account>::unpack(&buffer)
        .unwrap()
        .base;

    // Deserialize without extensions need to truncate buffer to correct length.
    let token_account_data_no_extensions = Account::unpack(&buffer[..165]).unwrap();
    assert_eq!(token_account_data, token_account_data_no_extensions);
    let token_account_data = StateWithExtensions::<Account>::unpack(&buffer)
        .unwrap()
        .get_first_extension_type();
    println!("token_account_data {:?}", token_account_data);
}
