//! Tests token solana account - spl token account layout compatibility
//!
//! Tests:
//! 1. test_compressed_token_equivalent_to_pod_account
//! 2. test_compressed_token_with_pausable_extension
//! 3. test_account_type_compatibility_with_spl_parsing

use light_compressed_account::Pubkey;
use light_token_interface::state::{
    token::{Token, TokenConfig, ZToken, ZTokenMut, BASE_TOKEN_ACCOUNT_SIZE},
    extensions::ExtensionStructConfig,
    ACCOUNT_TYPE_TOKEN_ACCOUNT,
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

fn default_config() -> TokenConfig {
    TokenConfig {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        state: 1,
        extensions: None,
    }
}

/// Generate random token account data using SPL Token's pack method
/// Creates a buffer large enough for the full Token meta struct
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

    // Create buffer for SPL-compatible token account (165 bytes, no extensions)
    let mut account_data = vec![0u8; BASE_TOKEN_ACCOUNT_SIZE as usize];
    Account::pack(account, &mut account_data[..Account::LEN]).unwrap();
    // Note: No account_type byte for pure SPL accounts without extensions
    account_data
}

/// Compare all fields between our Token zero-copy implementation and Pod account
fn compare_compressed_token_with_pod_account(
    compressed_token: &ZToken,
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
    if u64::from(compressed_token.amount) != u64::from(pod_account.amount) {
        return false;
    }

    // Compare delegate using getter
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
    match (compressed_token.delegate(), pod_delegate_option) {
        (Some(compressed_delegate), Some(pod_delegate)) => {
            if compressed_delegate.to_bytes() != pod_delegate.to_bytes() {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare state
    if compressed_token.state != pod_account.state {
        return false;
    }

    // Compare is_native using getter
    let pod_native_option: Option<u64> = if pod_account.is_native.is_some() {
        Some(u64::from(
            pod_account.is_native.unwrap_or(PodU64::default()),
        ))
    } else {
        None
    };
    match (compressed_token.is_native_value(), pod_native_option) {
        (Some(compressed_native), Some(pod_native)) => {
            if compressed_native != pod_native {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare delegated_amount
    if u64::from(compressed_token.delegated_amount) != u64::from(pod_account.delegated_amount) {
        return false;
    }

    // Compare close_authority using getter
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
    match (compressed_token.close_authority(), pod_close_option) {
        (Some(compressed_close), Some(pod_close)) => {
            if compressed_close.to_bytes() != pod_close.to_bytes() {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    true
}

/// Compare all fields between our Token mutable zero-copy implementation and Pod account
fn compare_compressed_token_mut_with_pod_account(
    compressed_token: &ZTokenMut,
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
    if u64::from(compressed_token.amount) != u64::from(pod_account.amount) {
        return false;
    }

    // Compare delegate using getter
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
    match (compressed_token.delegate(), pod_delegate_option) {
        (Some(compressed_delegate), Some(pod_delegate)) => {
            if compressed_delegate.to_bytes() != pod_delegate.to_bytes() {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare state
    if compressed_token.state != pod_account.state {
        println!(
            "State mismatch: compressed={}, pod={}",
            compressed_token.state, pod_account.state
        );
        return false;
    }

    // Compare is_native using getter
    let pod_native_option: Option<u64> = if pod_account.is_native.is_some() {
        Some(u64::from(
            pod_account.is_native.unwrap_or(PodU64::default()),
        ))
    } else {
        None
    };
    match (compressed_token.is_native_value(), pod_native_option) {
        (Some(compressed_native), Some(pod_native)) => {
            if compressed_native != pod_native {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare delegated_amount
    if u64::from(compressed_token.delegated_amount) != u64::from(pod_account.delegated_amount) {
        return false;
    }

    // Compare close_authority using getter
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
    match (compressed_token.close_authority(), pod_close_option) {
        (Some(compressed_close), Some(pod_close)) => {
            if compressed_close.to_bytes() != pod_close.to_bytes() {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    true
}

#[test]
fn test_compressed_token_equivalent_to_pod_account() {
    let mut rng = rand::thread_rng();

    for _ in 0..10000 {
        let mut account_data = generate_random_token_account_data(&mut rng);
        let account_data_clone = account_data.clone();
        // Pod account only knows about the first 165 bytes
        let pod_account = pod_from_bytes::<PodAccount>(&account_data_clone[..165]).unwrap();

        // Test immutable version
        let (compressed_token, _) = Token::zero_copy_at(&account_data).unwrap();
        println!("Compressed Token: {:?}", compressed_token);
        println!("Pod Account: {:?}", pod_account);
        assert!(compare_compressed_token_with_pod_account(
            &compressed_token,
            pod_account
        ));
        {
            let account_data_clone = account_data.clone();
            // Pod account only knows about the first 165 bytes
            let pod_account = pod_from_bytes::<PodAccount>(&account_data_clone[..165]).unwrap();
            // Test mutable version
            let (compressed_token_mut, _) = Token::zero_copy_at_mut(&mut account_data).unwrap();
            println!("Compressed Token Mut: {:?}", compressed_token_mut);
            println!("Pod Account: {:?}", pod_account);

            assert!(compare_compressed_token_mut_with_pod_account(
                &compressed_token_mut,
                pod_account
            ));
        }
    }
}

#[test]
fn test_compressed_token_with_pausable_extension() {
    let config = TokenConfig {
        extensions: Some(vec![ExtensionStructConfig::PausableAccount(())]),
        ..default_config()
    };

    let required_size = Token::byte_len(&config).unwrap();
    println!("Required size for pausable extension: {}", required_size);

    // Should be more than 165 bytes due to AccountType byte and extension
    assert!(required_size > 165);

    let mut buffer = vec![0u8; required_size];
    {
        let (_, remaining_bytes) = Token::new_zero_copy(&mut buffer, config)
            .expect("Failed to initialize compressed token with pausable extension");

        assert_eq!(remaining_bytes.len(), 0);
        // Note: new_zero_copy now writes extensions directly to bytes but returns extensions: None
        // Extensions are parsed when deserializing with zero_copy_at
    }

    // Test zero-copy deserialization round-trip - extensions are parsed from bytes
    let (deserialized_token, _) =
        Token::zero_copy_at(&buffer).expect("Failed to deserialize token with pausable extension");

    assert!(deserialized_token.extensions.is_some());
    let deserialized_extensions = deserialized_token.extensions.as_ref().unwrap();
    assert_eq!(deserialized_extensions.len(), 1);

    // Test mutable deserialization with a fresh buffer
    let mut buffer_copy = buffer.clone();
    let (mutable_token, _) = Token::zero_copy_at_mut(&mut buffer_copy)
        .expect("Failed to deserialize mutable token with pausable extension");

    assert!(mutable_token.extensions.is_some());
}

#[test]
fn test_account_type_compatibility_with_spl_parsing() {
    let config = TokenConfig {
        extensions: Some(vec![ExtensionStructConfig::PausableAccount(())]),
        ..default_config()
    };

    let mut buffer = vec![0u8; Token::byte_len(&config).unwrap()];
    {
        let (mut compressed_token, _) = Token::new_zero_copy(&mut buffer, config)
            .expect("Failed to create token with extension");
        // Set state to Initialized (1) for SPL compatibility - required for SPL parsing
        compressed_token.base.state = 1;
    }

    let pod_account = pod_from_bytes::<PodAccount>(&buffer[..165])
        .expect("First 165 bytes should be valid SPL Token Account data");
    let pod_state = PodStateWithExtensions::<PodAccount>::unpack(&buffer[..165])
        .expect("Pod account with extensions should succeed.");
    let base_account = pod_state.base;
    assert_eq!(pod_account, base_account);

    // Verify AccountType byte is at position 165
    assert_eq!(buffer[165], ACCOUNT_TYPE_TOKEN_ACCOUNT);

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

/// Test PartialEq between ZToken and Token with Pausable extension.
#[test]
fn test_pausable_extension_partial_eq() {
    use light_token_interface::state::{
        token::AccountState as CtokenAccountState,
        extensions::{ExtensionStruct, PausableAccountExtension},
    };

    let config = TokenConfig {
        extensions: Some(vec![ExtensionStructConfig::PausableAccount(())]),
        ..default_config()
    };

    let mut buffer = vec![0u8; Token::byte_len(&config).unwrap()];
    let _ = Token::new_zero_copy(&mut buffer, config).unwrap();

    // new_zero_copy now sets fields from config
    let expected = Token {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        amount: 0,
        delegate: None,
        state: CtokenAccountState::Initialized, // state: 1 from default_config
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: Some(vec![ExtensionStruct::PausableAccount(
            PausableAccountExtension,
        )]),
    };

    let (zctoken, _) = Token::zero_copy_at(&buffer).unwrap();
    assert_eq!(zctoken, expected);
}
