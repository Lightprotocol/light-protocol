mod common;

use common::{make_config_account, make_dummy_account};
use light_account_checks::account_info::test_account_info::solana_program::TestAccount;
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
use light_sdk_types::{
    error::LightSdkTypesError,
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    interface::program::compression::processor::{
        build_compress_pda_cpi_data, CompressAndCloseParams, CompressCtx,
    },
    CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

// ============================================================================
// Mock dispatch functions
// ============================================================================

fn mock_dispatch_compressible<'a>(
    _account: &AccountInfo<'a>,
    _meta: &CompressedAccountMetaNoLamportsNoAddress,
    pda_index: usize,
    ctx: &mut CompressCtx<'_, AccountInfo<'a>>,
) -> Result<(), LightSdkTypesError> {
    ctx.compressed_account_infos.push(CompressedAccountInfo {
        address: None,
        input: None,
        output: None,
    });
    ctx.pda_indices_to_close.push(pda_index);
    Ok(())
}

fn mock_dispatch_non_compressible<'a>(
    _account: &AccountInfo<'a>,
    _meta: &CompressedAccountMetaNoLamportsNoAddress,
    _pda_index: usize,
    ctx: &mut CompressCtx<'_, AccountInfo<'a>>,
) -> Result<(), LightSdkTypesError> {
    ctx.has_non_compressible = true;
    Ok(())
}

// ============================================================================
// Helper: build the standard 5-account layout for valid tests
// [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=system_account, [4]=pda_account
// ============================================================================

fn make_valid_accounts(
    program_id: [u8; 32],
) -> (
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
) {
    let (config_account, rent_sponsor_key) = make_config_account(program_id);
    let fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let rent_sponsor = make_dummy_account(rent_sponsor_key, [0u8; 32], 0);
    let system_account = make_dummy_account([11u8; 32], [0u8; 32], 0);
    let pda_account = make_dummy_account([10u8; 32], program_id, 100);
    (
        fee_payer,
        config_account,
        rent_sponsor,
        system_account,
        pda_account,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_system_offset_exceeds_accounts_returns_error() {
    let program_id = [42u8; 32];
    let mut fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let mut dummy2 = make_dummy_account([2u8; 32], [0u8; 32], 0);
    let mut dummy3 = make_dummy_account([3u8; 32], [0u8; 32], 0);

    let fee_payer_ai = fee_payer.get_account_info();
    let dummy2_ai = dummy2.get_account_info();
    let dummy3_ai = dummy3.get_account_info();

    // system_accounts_offset = 100 > remaining_accounts.len() = 3 -> error
    let remaining_accounts = vec![fee_payer_ai, dummy2_ai, dummy3_ai];
    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 100,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidInstructionData)
    ));
}

#[test]
fn test_empty_compressed_accounts_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidInstructionData)
    ));
}

#[test]
fn test_not_enough_remaining_accounts_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, _, _) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();

    // Only 3 accounts, but 10 compressed_accounts requested -> checked_sub underflows
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default(); 10],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::NotEnoughAccountKeys)
    ));
}

#[test]
fn test_config_wrong_owner_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    // Override config account owner to a wrong value
    config_account.owner = Pubkey::from([99u8; 32]);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::ConstraintViolation)
    ));
}

#[test]
fn test_config_wrong_discriminator_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    // Override config data with zeros (wrong discriminator)
    config_account.data = vec![0u8; 170];

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::ConstraintViolation)
    ));
}

#[test]
fn test_wrong_rent_sponsor_key_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    // Override rent_sponsor key to a value that doesn't match config.rent_sponsor
    rent_sponsor.key = Pubkey::from([77u8; 32]);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidRentSponsor)
    ));
}

#[test]
fn test_idempotent_returns_none_when_non_compressible() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_non_compressible,
        cpi_signer,
        &program_id,
    );

    assert!(matches!(result, Ok(None)));
}

#[test]
fn test_build_compress_produces_expected_instruction_data() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    // [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=system, [4]=pda
    // num_pdas=1, pda_start = 5 - 1 = 4 -> pda_index = 4
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];

    let params = CompressAndCloseParams {
        proof: ValidityProof::default(),
        compressed_accounts: vec![CompressedAccountMetaNoLamportsNoAddress::default()],
        system_accounts_offset: 3,
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result = build_compress_pda_cpi_data(
        &remaining_accounts,
        &params,
        mock_dispatch_compressible,
        cpi_signer,
        &program_id,
    );

    let expected_cpi_ix_data = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1,
        bump: 255,
        invoking_program_id: program_id.into(),
        account_infos: vec![CompressedAccountInfo {
            address: None,
            input: None,
            output: None,
        }],
        proof: None,
        ..Default::default()
    };

    let built = result.unwrap().unwrap();
    assert_eq!(built.cpi_ix_data, expected_cpi_ix_data);
    // pda_start = 5 - 1 = 4
    assert_eq!(built.pda_indices_to_close, vec![4usize]);
}
