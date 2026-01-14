//! Specific unit tests for build_mint_extension_cache and check_mint_extensions.
//!
//! Tests are organized into categories:
//! - Category 1: Failure tests for MintHasRestrictedExtensions
//! - Category 2: Failure tests for CompressAndClose
//! - Category 3: Success tests for bypass scenarios
//! - Category 4: Success tests for non-restricted mints
//! - Category 5: Direct check_mint_extensions tests

use anchor_compressed_token::ErrorCode;
use anchor_lang::{prelude::ProgramError, solana_program::pubkey::Pubkey as SolanaPubkey};
use light_account_checks::{
    account_info::test_account_info::pinocchio::get_account_info,
    packed_accounts::ProgramPackedAccounts,
};
use light_compressed_token::{
    compressed_token::transfer2::check_extensions::build_mint_extension_cache,
    extensions::check_mint_extensions,
};
use light_token_interface::instructions::{
    extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    transfer2::{
        CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
        MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
    },
};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::pubkey::Pubkey;
use spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodBool};
use spl_token_2022::{
    extension::{
        metadata_pointer::MetadataPointer, pausable::PausableConfig,
        permanent_delegate::PermanentDelegate, transfer_fee::TransferFeeConfig,
        transfer_hook::TransferHook, BaseStateWithExtensionsMut, ExtensionType,
        PodStateWithExtensionsMut,
    },
    pod::PodMint,
};

const ANCHOR_ERROR_OFFSET: u32 = 6000;
const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();
const SPL_TOKEN_ID: [u8; 32] = spl_token::ID.to_bytes();

// ============================================================================
// Helpers
// ============================================================================

/// Configuration for creating mock T22 mint with extensions.
#[derive(Default, Clone)]
struct MintConfig {
    pub has_pausable: bool,
    pub is_paused: bool,
    pub has_transfer_fee: bool,
    pub has_non_zero_fee: bool,
    pub has_transfer_hook: bool,
    pub has_non_nil_hook: bool,
    pub has_permanent_delegate: bool,
    pub has_metadata_pointer: bool,
}

/// Create mock T22 mint data with specified extensions.
fn create_mock_t22_mint(config: &MintConfig) -> Vec<u8> {
    use spl_token_2022::pod::PodCOption;

    let mut extensions = vec![];
    if config.has_pausable {
        extensions.push(ExtensionType::Pausable);
    }
    if config.has_transfer_fee {
        extensions.push(ExtensionType::TransferFeeConfig);
    }
    if config.has_transfer_hook {
        extensions.push(ExtensionType::TransferHook);
    }
    if config.has_permanent_delegate {
        extensions.push(ExtensionType::PermanentDelegate);
    }
    if config.has_metadata_pointer {
        extensions.push(ExtensionType::MetadataPointer);
    }

    let space = ExtensionType::try_calculate_account_len::<PodMint>(&extensions).unwrap();
    let mut data = vec![0u8; space];

    let mut mint_state =
        PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut data).unwrap();

    // Initialize base mint
    mint_state.base.mint_authority = PodCOption::some(SolanaPubkey::new_unique());
    mint_state.base.decimals = 9;
    mint_state.base.is_initialized = true.into();
    mint_state.base.freeze_authority = PodCOption::none();
    mint_state.base.supply = 1_000_000u64.into();
    mint_state.init_account_type().unwrap();

    // Initialize extensions
    if config.has_pausable {
        let ext = mint_state.init_extension::<PausableConfig>(true).unwrap();
        ext.authority = OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
        ext.paused = PodBool::from(config.is_paused);
    }

    if config.has_transfer_fee {
        let ext = mint_state
            .init_extension::<TransferFeeConfig>(true)
            .unwrap();
        ext.transfer_fee_config_authority =
            OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
        ext.withdraw_withheld_authority =
            OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
        if config.has_non_zero_fee {
            ext.older_transfer_fee.transfer_fee_basis_points = 100u16.into();
            ext.older_transfer_fee.maximum_fee = 1000u64.into();
            ext.newer_transfer_fee.transfer_fee_basis_points = 100u16.into();
            ext.newer_transfer_fee.maximum_fee = 1000u64.into();
        }
    }

    if config.has_transfer_hook {
        let ext = mint_state.init_extension::<TransferHook>(true).unwrap();
        if config.has_non_nil_hook {
            ext.program_id =
                OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
        }
    }

    if config.has_permanent_delegate {
        let ext = mint_state
            .init_extension::<PermanentDelegate>(true)
            .unwrap();
        ext.delegate = OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
    }

    if config.has_metadata_pointer {
        let ext = mint_state.init_extension::<MetadataPointer>(true).unwrap();
        ext.metadata_address =
            OptionalNonZeroPubkey::try_from(Some(SolanaPubkey::new_unique())).unwrap();
    }

    data
}

/// Create mock SPL Token (non-T22) mint data.
fn create_mock_spl_token_mint() -> Vec<u8> {
    // SPL Token mint is 82 bytes
    let mut data = vec![0u8; 82];
    // Set is_initialized = true (offset 45, 1 byte)
    data[45] = 1;
    // Set decimals (offset 44)
    data[44] = 9;
    data
}

/// Test configuration for instruction data.
#[derive(Default)]
struct TestConfig {
    pub has_inputs: bool,
    pub has_outputs: bool,
    pub has_compressions: bool,
    pub compression_mode: Option<CompressionMode>,
    pub has_compressed_only_in_output: bool,
    pub output_amount: u64,
}

/// Create serialized instruction data for testing.
fn create_test_inputs(config: &TestConfig) -> Vec<u8> {
    let in_token_data = if config.has_inputs {
        vec![MultiInputTokenDataWithContext {
            mint: 0,
            amount: 100,
            ..Default::default()
        }]
    } else {
        vec![]
    };

    let out_token_data = if config.has_outputs {
        vec![MultiTokenTransferOutputData {
            mint: 0,
            amount: config.output_amount,
            ..Default::default()
        }]
    } else {
        vec![]
    };

    let compressions = if config.has_compressions {
        Some(vec![Compression {
            mode: config.compression_mode.unwrap_or(CompressionMode::Compress),
            amount: 100,
            mint: 0,
            source_or_recipient: 1,
            authority: 2,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
            decimals: 0,
        }])
    } else {
        None
    };

    let out_tlv = if config.has_outputs && config.has_compressed_only_in_output {
        Some(vec![vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: false,
                compression_index: 0,
                is_ata: false,
                bump: 0,
                owner_index: 0,
            },
        )]])
    } else if config.has_outputs {
        Some(vec![vec![]]) // Empty TLV for each output
    } else {
        None
    };

    let instruction_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: 0,
        cpi_context: None,
        compressions,
        proof: None,
        in_token_data,
        out_token_data,
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv,
    };

    borsh::to_vec(&instruction_data).unwrap()
}

/// Run build_mint_extension_cache with test data.
fn run_build_cache_test(
    serialized_inputs: &[u8],
    mint_data: &[u8],
    owner: [u8; 32],
) -> Result<(), ProgramError> {
    let (inputs, _) =
        CompressedTokenInstructionDataTransfer2::zero_copy_at(serialized_inputs).unwrap();

    let mint_account = get_account_info(
        Pubkey::from(owner),
        owner,
        false,
        false,
        false,
        mint_data.to_vec(),
    );

    let accounts = [mint_account];
    let packed_accounts = ProgramPackedAccounts {
        accounts: &accounts,
    };

    build_mint_extension_cache(&inputs, &packed_accounts).map(|_| ())
}

/// Helper to assert specific error code.
fn assert_error(result: Result<(), ProgramError>, expected: ErrorCode) {
    let expected_code = ANCHOR_ERROR_OFFSET + expected as u32;
    assert!(
        matches!(result, Err(ProgramError::Custom(code)) if code == expected_code),
        "Expected error {:?} (code {}), got {:?}",
        expected,
        expected_code,
        result
    );
}

// ============================================================================
// Category 1: Failure Cases - MintHasRestrictedExtensions
// ============================================================================

#[test]
fn test_input_with_pausable_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_input_with_permanent_delegate_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_permanent_delegate: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_input_with_transfer_fee_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_fee: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_input_with_transfer_hook_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_hook: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_compress_with_pausable_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::Compress),
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_decompress_with_pausable_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::Decompress),
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_zero_amount_output_with_restricted_extension_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_outputs: true,
        output_amount: 0, // Zero amount output
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(result, ErrorCode::MintHasRestrictedExtensions);
}

// ============================================================================
// Category 2: Failure Cases - CompressAndClose
// ============================================================================

#[test]
fn test_compress_and_close_missing_compressed_only_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true, // Restricted extension
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::CompressAndClose),
        has_outputs: true,
        has_compressed_only_in_output: false, // Missing CompressedOnly
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(
        result,
        ErrorCode::CompressAndCloseMissingCompressedOnlyExtension,
    );
}

#[test]
fn test_compress_and_close_empty_tlv_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_permanent_delegate: true, // Different restricted extension
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::CompressAndClose),
        has_outputs: true,
        has_compressed_only_in_output: false, // Empty TLV
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert_error(
        result,
        ErrorCode::CompressAndCloseMissingCompressedOnlyExtension,
    );
}

// ============================================================================
// Category 3: Success Cases - Bypass Scenarios
// ============================================================================

#[test]
fn test_input_with_restricted_no_outputs_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true, // Restricted, but no outputs = bypass
        is_paused: true,    // Even paused is OK with bypass
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: false, // No outputs = bypass
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "Should succeed with bypass, got {:?}",
        result
    );
}

#[test]
fn test_compress_and_close_with_compressed_only_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        is_paused: true, // Even paused is OK for CompressAndClose
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::CompressAndClose),
        has_outputs: true,
        has_compressed_only_in_output: true, // Has required extension
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "Should succeed with CompressedOnly, got {:?}",
        result
    );
}

#[test]
fn test_decompress_no_outputs_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_fee: true,
        has_non_zero_fee: true, // Would fail if checked
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::Decompress),
        has_outputs: false, // No outputs = bypass
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "Should succeed with bypass, got {:?}",
        result
    );
}

#[test]
fn test_compress_no_outputs_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_hook: true,
        has_non_nil_hook: true, // Would fail if checked
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_compressions: true,
        compression_mode: Some(CompressionMode::Compress),
        has_outputs: false, // No outputs = bypass
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "Should succeed with bypass, got {:?}",
        result
    );
}

// ============================================================================
// Category 4: Success Cases - Non-Restricted Mints
// ============================================================================

#[test]
fn test_spl_token_mint_succeeds() {
    let mint_data = create_mock_spl_token_mint();
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    // SPL Token mint is owned by spl_token::ID, not spl_token_2022::ID
    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_ID);
    assert!(result.is_ok(), "SPL Token should succeed, got {:?}", result);
}

#[test]
fn test_t22_mint_no_extensions_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig::default()); // No extensions
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "T22 without extensions should succeed, got {:?}",
        result
    );
}

#[test]
fn test_t22_mint_with_metadata_only_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_metadata_pointer: true, // Not a restricted extension
        ..Default::default()
    });
    let inputs = create_test_inputs(&TestConfig {
        has_inputs: true,
        has_outputs: true,
        output_amount: 100,
        ..Default::default()
    });

    let result = run_build_cache_test(&inputs, &mint_data, SPL_TOKEN_2022_ID);
    assert!(
        result.is_ok(),
        "MetadataPointer should succeed, got {:?}",
        result
    );
}

// ============================================================================
// Category 5: Direct check_mint_extensions Tests
// ============================================================================

#[test]
fn test_check_mint_extensions_paused_mint() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        is_paused: true,
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    // Call check_mint_extensions directly with deny_restricted=false
    // This bypasses the restricted check and reaches the paused check
    let result = check_mint_extensions(&mint_account, false);
    assert_error(result.map(|_| ()), ErrorCode::MintPaused);
}

#[test]
fn test_check_mint_extensions_non_zero_fee() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_fee: true,
        has_non_zero_fee: true,
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    let result = check_mint_extensions(&mint_account, false);
    assert_error(
        result.map(|_| ()),
        ErrorCode::NonZeroTransferFeeNotSupported,
    );
}

#[test]
fn test_check_mint_extensions_non_nil_hook() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_transfer_hook: true,
        has_non_nil_hook: true,
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    let result = check_mint_extensions(&mint_account, false);
    assert_error(result.map(|_| ()), ErrorCode::TransferHookNotSupported);
}

#[test]
fn test_check_mint_extensions_deny_restricted_fails() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true, // Restricted extension
        is_paused: false,   // Not paused, but still restricted
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    // deny_restricted=true should fail even if mint state is valid
    let result = check_mint_extensions(&mint_account, true);
    assert_error(result.map(|_| ()), ErrorCode::MintHasRestrictedExtensions);
}

#[test]
fn test_check_mint_extensions_deny_restricted_non_restricted_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_metadata_pointer: true, // Not a restricted extension
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    // deny_restricted=true should succeed with non-restricted mint
    let result = check_mint_extensions(&mint_account, true);
    assert!(
        result.is_ok(),
        "Non-restricted mint should succeed, got {:?}",
        result
    );
}

#[test]
fn test_check_mint_extensions_valid_mint_succeeds() {
    let mint_data = create_mock_t22_mint(&MintConfig {
        has_pausable: true,
        is_paused: false, // Not paused
        has_transfer_fee: true,
        has_non_zero_fee: false, // Zero fee
        has_transfer_hook: true,
        has_non_nil_hook: false, // Nil hook
        ..Default::default()
    });
    let mint_account = get_account_info(
        Pubkey::from(SPL_TOKEN_2022_ID),
        SPL_TOKEN_2022_ID,
        false,
        false,
        false,
        mint_data,
    );

    // deny_restricted=false with all valid states should succeed
    let result = check_mint_extensions(&mint_account, false);
    assert!(
        result.is_ok(),
        "Valid mint should succeed, got {:?}",
        result
    );
}
