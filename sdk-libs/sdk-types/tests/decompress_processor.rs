mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use common::{make_dummy_account, make_valid_accounts, SkipVariant};
use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
use light_sdk_types::{
    error::LightSdkTypesError,
    instruction::PackedStateTreeInfo,
    interface::{
        account::compression_info::CompressedAccountData,
        program::decompression::processor::{
            build_decompress_pda_cpi_data, DecompressCtx, DecompressIdempotentParams,
            DecompressVariant,
        },
    },
    CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

// ============================================================================
// Mock DecompressVariant implementations
// ============================================================================

/// Pushes one known CompressedAccountInfo to simulate decompression.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
struct DecompressVariantMock;

impl<'info> DecompressVariant<AccountInfo<'info>> for DecompressVariantMock {
    fn decompress(
        &self,
        _meta: &PackedStateTreeInfo,
        _pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        ctx.compressed_account_infos.push(CompressedAccountInfo {
            address: None,
            input: None,
            output: None,
        });
        Ok(())
    }
}

fn one_pda_params<V: BorshSerialize + BorshDeserialize + Clone>(
    data: V,
    system_accounts_offset: u8,
) -> DecompressIdempotentParams<V> {
    DecompressIdempotentParams {
        system_accounts_offset,
        token_accounts_offset: 1, // 1 PDA account, 0 token accounts
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data,
        }],
    }
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
    let params = DecompressIdempotentParams {
        system_accounts_offset: 100,
        token_accounts_offset: 1,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: SkipVariant,
        }],
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidInstructionData)
    ));
}

#[test]
fn test_empty_pda_accounts_returns_error() {
    let program_id = [42u8; 32];
    let mut fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let mut dummy2 = make_dummy_account([2u8; 32], [0u8; 32], 0);
    let mut dummy3 = make_dummy_account([3u8; 32], [0u8; 32], 0);

    let fee_payer_ai = fee_payer.get_account_info();
    let dummy2_ai = dummy2.get_account_info();
    let dummy3_ai = dummy3.get_account_info();

    // token_accounts_offset = 0 -> pda_accounts = accounts[0..0] = [] -> empty -> error
    let remaining_accounts = vec![fee_payer_ai, dummy2_ai, dummy3_ai];
    let params = DecompressIdempotentParams {
        system_accounts_offset: 3,
        token_accounts_offset: 0,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: SkipVariant,
        }],
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidInstructionData)
    ));
}

#[test]
fn test_not_enough_hot_accounts_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, _, _) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();

    // Only 3 remaining_accounts, 5 accounts in params -> checked_sub(5) on len=3 fails
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai];

    let params = DecompressIdempotentParams {
        system_accounts_offset: 3,
        token_accounts_offset: 5,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: SkipVariant,
            },
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: SkipVariant,
            },
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: SkipVariant,
            },
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: SkipVariant,
            },
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: SkipVariant,
            },
        ],
    };
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

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
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

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
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

    assert!(matches!(
        result,
        Err(LightSdkTypesError::InvalidRentSponsor)
    ));
}

#[test]
fn test_idempotent_returns_none_when_all_initialized() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    // SkipVariant pushes nothing -> compressed_account_infos.is_empty() -> Ok(None)
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

    assert!(matches!(result, Ok(None)));
}

#[test]
fn test_build_decompress_produces_expected_instruction_data() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    // [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=system, [4]=pda
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    // DecompressVariantMock pushes one CompressedAccountInfo -> CPI data built
    let params = one_pda_params(DecompressVariantMock, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

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
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
struct ErrorVariant;

impl<'info> DecompressVariant<AccountInfo<'info>> for ErrorVariant {
    fn decompress(
        &self,
        _meta: &PackedStateTreeInfo,
        _pda_account: &AccountInfo<'info>,
        _ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        Err(LightSdkTypesError::ConstraintViolation)
    }
}

#[test]
fn test_decompress_variant_error_propagates() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    let params = one_pda_params(ErrorVariant, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

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

    config_account.data = vec![0u8; 170];

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner {
        program_id,
        cpi_signer: [0u8; 32],
        bump: 255,
    };

    let result =
        build_decompress_pda_cpi_data(&remaining_accounts, &params, cpi_signer, &program_id, 0);

    assert!(matches!(
        result,
        Err(LightSdkTypesError::ConstraintViolation)
    ));
}
