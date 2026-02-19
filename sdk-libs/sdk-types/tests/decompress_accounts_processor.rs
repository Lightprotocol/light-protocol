#![cfg(feature = "token")]

mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_checks::account_info::test_account_info::solana_program::TestAccount;
use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::{compressed_proof::ValidityProof, with_account_info::CompressedAccountInfo},
};
use light_sdk_types::{
    error::LightSdkTypesError,
    instruction::PackedStateTreeInfo,
    interface::{
        account::compression_info::CompressedAccountData,
        program::decompression::processor::{
            build_decompress_accounts_cpi_data, DecompressCtx, DecompressIdempotentParams,
            DecompressVariant,
        },
    },
    CpiSigner,
};
use light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext;
use rand::Rng;
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use common::{make_config_account, make_dummy_account};

// ============================================================================
// Mock DecompressVariant implementations
// ============================================================================

/// Pushes nothing — simulates an already-initialized PDA.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
struct SkipVariant;

impl<'info> DecompressVariant<AccountInfo<'info>> for SkipVariant {
    fn decompress(
        &self,
        _meta: &PackedStateTreeInfo,
        _pda_account: &AccountInfo<'info>,
        _ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        Ok(())
    }
}

/// Carries and pushes a specific CompressedAccountInfo — simulates PDA decompression.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
struct PdaMockVariant(CompressedAccountInfo);

impl<'info> DecompressVariant<AccountInfo<'info>> for PdaMockVariant {
    fn decompress(
        &self,
        _meta: &PackedStateTreeInfo,
        _pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        ctx.compressed_account_infos.push(self.0.clone());
        Ok(())
    }
}

/// Carries and pushes a specific MultiInputTokenDataWithContext — simulates token decompression.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
struct TokenMockVariant(MultiInputTokenDataWithContext);

impl<'info> DecompressVariant<AccountInfo<'info>> for TokenMockVariant {
    fn decompress(
        &self,
        _meta: &PackedStateTreeInfo,
        _pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        ctx.in_token_data.push(self.0);
        Ok(())
    }
}

/// Unified enum for tests that mix PDA and token accounts in one params.accounts Vec.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
enum MockVariant {
    Pda(PdaMockVariant),
    Token(TokenMockVariant),
}

impl<'info> DecompressVariant<AccountInfo<'info>> for MockVariant {
    fn decompress(
        &self,
        meta: &PackedStateTreeInfo,
        pda_account: &AccountInfo<'info>,
        ctx: &mut DecompressCtx<'_, AccountInfo<'info>>,
    ) -> Result<(), LightSdkTypesError> {
        match self {
            MockVariant::Pda(v) => v.decompress(meta, pda_account, ctx),
            MockVariant::Token(v) => v.decompress(meta, pda_account, ctx),
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Standard 5-account layout for PDA-only error/happy path tests.
/// [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=system_account, [4]=pda_account
fn make_valid_accounts(
    program_id: [u8; 32],
) -> (TestAccount, TestAccount, TestAccount, TestAccount, TestAccount) {
    let (config_account, rent_sponsor_key) = make_config_account(program_id);
    let fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let rent_sponsor = make_dummy_account(rent_sponsor_key, [0u8; 32], 0);
    let system_account = make_dummy_account([11u8; 32], [0u8; 32], 0);
    let pda_account = make_dummy_account([10u8; 32], program_id, 100);
    (fee_payer, config_account, rent_sponsor, system_account, pda_account)
}

/// 9-account layout for PDA+token tests.
/// [0]=fee_payer, [1]=config, [2]=rent_sponsor,
/// [3]=ctoken_rent_sponsor, [4..6]=dummies, [6]=ctoken_compressible_config,
/// (system_accounts_offset=7, no system accounts between 7 and hot_accounts_start)
/// [7]=pda_account, [8]=token_account
fn make_valid_accounts_with_tokens(
    program_id: [u8; 32],
) -> (
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
    TestAccount,
) {
    let (config_account, rent_sponsor_key) = make_config_account(program_id);
    let fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let rent_sponsor = make_dummy_account(rent_sponsor_key, [0u8; 32], 0);
    let ctoken_rent_sponsor = make_dummy_account([3u8; 32], [0u8; 32], 0);
    let dummy4 = make_dummy_account([4u8; 32], [0u8; 32], 0);
    let dummy5 = make_dummy_account([5u8; 32], [0u8; 32], 0);
    let ctoken_config = make_dummy_account([6u8; 32], [0u8; 32], 0);
    let pda_account = make_dummy_account([10u8; 32], program_id, 100);
    let token_account = make_dummy_account([20u8; 32], program_id, 100);
    (
        fee_payer,
        config_account,
        rent_sponsor,
        ctoken_rent_sponsor,
        dummy4,
        dummy5,
        ctoken_config,
        pda_account,
        token_account,
    )
}

fn one_pda_params<V: BorshSerialize + BorshDeserialize + Clone>(
    data: V,
    system_accounts_offset: u8,
) -> DecompressIdempotentParams<V> {
    DecompressIdempotentParams {
        system_accounts_offset,
        token_accounts_offset: 1,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data,
        }],
    }
}

// ============================================================================
// Error path tests
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
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    assert!(matches!(result, Err(LightSdkTypesError::InvalidInstructionData)));
}

#[test]
fn test_bad_token_accounts_offset_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    // token_accounts_offset=99 > accounts.len()=1 -> split_at_checked returns None
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    let params = DecompressIdempotentParams {
        system_accounts_offset: 3,
        token_accounts_offset: 99,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: SkipVariant,
        }],
    };
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    assert!(matches!(result, Err(LightSdkTypesError::InvalidInstructionData)));
}

#[test]
fn test_not_enough_hot_accounts_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, _, _) =
        make_valid_accounts(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();

    // accounts.len()=5, remaining_accounts.len()=3 -> checked_sub(5) underflows
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
            };
            5
        ],
    };
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    assert!(matches!(result, Err(LightSdkTypesError::NotEnoughAccountKeys)));
}

#[test]
fn test_config_wrong_owner_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    config_account.owner = Pubkey::from([99u8; 32]);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    assert!(matches!(result, Err(LightSdkTypesError::ConstraintViolation)));
}

#[test]
fn test_wrong_rent_sponsor_key_returns_error() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, mut system_account, mut pda_account) =
        make_valid_accounts(program_id);

    rent_sponsor.key = Pubkey::from([77u8; 32]);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let system_ai = system_account.get_account_info();
    let pda_ai = pda_account.get_account_info();

    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, system_ai, pda_ai];
    let params = one_pda_params(SkipVariant, 3);
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    assert!(matches!(result, Err(LightSdkTypesError::InvalidRentSponsor)));
}

// ============================================================================
// Happy path tests
// ============================================================================

#[test]
fn test_pda_only_builds_correct_data() {
    let program_id = [42u8; 32];
    let (mut fee_payer, mut config_account, mut rent_sponsor, _, _) =
        make_valid_accounts(program_id);
    let mut pda_account = make_dummy_account([10u8; 32], program_id, 100);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let pda_ai = pda_account.get_account_info();

    // [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=pda (hot)
    // system_accounts_offset=3, token_accounts_offset=1 (=accounts.len() -> no tokens)
    let remaining_accounts = vec![fee_payer_ai, config_ai, rent_sponsor_ai, pda_ai];
    let params = one_pda_params(
        PdaMockVariant(CompressedAccountInfo { address: None, input: None, output: None }),
        3,
    );
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    let built = result.unwrap();
    assert!(built.has_pda_accounts);
    assert!(!built.has_token_accounts);
    assert!(!built.cpi_context);
    assert_eq!(
        built.compressed_account_infos,
        vec![CompressedAccountInfo { address: None, input: None, output: None }]
    );
    assert_eq!(built.in_token_data, Vec::<MultiInputTokenDataWithContext>::new());
}

#[test]
fn test_token_only_builds_correct_data() {
    let program_id = [42u8; 32];
    let (
        mut fee_payer,
        mut config_account,
        mut rent_sponsor,
        mut ctoken_rent_sponsor,
        mut dummy4,
        mut dummy5,
        mut ctoken_config,
        _,
        mut token_account,
    ) = make_valid_accounts_with_tokens(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let ctoken_rs_ai = ctoken_rent_sponsor.get_account_info();
    let dummy4_ai = dummy4.get_account_info();
    let dummy5_ai = dummy5.get_account_info();
    let ctoken_config_ai = ctoken_config.get_account_info();
    let token_ai = token_account.get_account_info();

    // [0..6] fixed, [7]=token (hot); system_accounts_offset=7, no system accounts
    let remaining_accounts = vec![
        fee_payer_ai,
        config_ai,
        rent_sponsor_ai,
        ctoken_rs_ai,
        dummy4_ai,
        dummy5_ai,
        ctoken_config_ai,
        token_ai,
    ];
    // token_accounts_offset=0: all accounts are tokens, no PDAs
    let params = DecompressIdempotentParams {
        system_accounts_offset: 7,
        token_accounts_offset: 0,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: TokenMockVariant(MultiInputTokenDataWithContext::default()),
        }],
    };
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    let built = result.unwrap();
    assert!(!built.has_pda_accounts);
    assert!(built.has_token_accounts);
    assert!(!built.cpi_context);
    assert_eq!(built.compressed_account_infos, Vec::<CompressedAccountInfo>::new());
    assert_eq!(built.in_token_data, vec![MultiInputTokenDataWithContext::default()]);
}

#[test]
fn test_pda_and_token_sets_cpi_context() {
    let program_id = [42u8; 32];
    let (
        mut fee_payer,
        mut config_account,
        mut rent_sponsor,
        mut ctoken_rent_sponsor,
        mut dummy4,
        mut dummy5,
        mut ctoken_config,
        mut pda_account,
        mut token_account,
    ) = make_valid_accounts_with_tokens(program_id);

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let ctoken_rs_ai = ctoken_rent_sponsor.get_account_info();
    let dummy4_ai = dummy4.get_account_info();
    let dummy5_ai = dummy5.get_account_info();
    let ctoken_config_ai = ctoken_config.get_account_info();
    let pda_ai = pda_account.get_account_info();
    let token_ai = token_account.get_account_info();

    // [0..6] fixed, [7]=pda, [8]=token (hot); system_accounts_offset=7
    let remaining_accounts = vec![
        fee_payer_ai,
        config_ai,
        rent_sponsor_ai,
        ctoken_rs_ai,
        dummy4_ai,
        dummy5_ai,
        ctoken_config_ai,
        pda_ai,
        token_ai,
    ];
    // token_accounts_offset=1: accounts[0]=PDA, accounts[1]=token
    let params = DecompressIdempotentParams {
        system_accounts_offset: 7,
        token_accounts_offset: 1,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts: vec![
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: MockVariant::Pda(PdaMockVariant(CompressedAccountInfo {
                    address: None,
                    input: None,
                    output: None,
                })),
            },
            CompressedAccountData {
                tree_info: PackedStateTreeInfo::default(),
                data: MockVariant::Token(TokenMockVariant(
                    MultiInputTokenDataWithContext::default(),
                )),
            },
        ],
    };
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    let built = result.unwrap();
    assert!(built.has_pda_accounts);
    assert!(built.has_token_accounts);
    assert!(built.cpi_context);
    assert_eq!(
        built.compressed_account_infos,
        vec![CompressedAccountInfo { address: None, input: None, output: None }]
    );
    assert_eq!(built.in_token_data, vec![MultiInputTokenDataWithContext::default()]);
}

#[test]
fn test_randomized_pda_and_token_decompression() {
    let program_id = [42u8; 32];
    let mut rng = rand::thread_rng();

    // Pick random counts in 0..=5, ensuring at least one account total.
    let mut n_pdas: usize = rng.gen_range(0..=5);
    let mut n_tokens: usize = rng.gen_range(0..=5);
    if n_pdas + n_tokens == 0 {
        // Clamp: give at least one of each type.
        n_pdas = 1;
        n_tokens = 1;
    }

    // Build expected PDA infos with random addresses.
    let expected_pda_infos: Vec<CompressedAccountInfo> = (0..n_pdas)
        .map(|_| CompressedAccountInfo {
            address: Some(rng.gen::<[u8; 32]>()),
            input: None,
            output: None,
        })
        .collect();

    // Build expected token data with random fields.
    let expected_token_data: Vec<MultiInputTokenDataWithContext> = (0..n_tokens)
        .map(|_| MultiInputTokenDataWithContext {
            owner: rng.gen::<u8>(),
            amount: rng.gen(),
            has_delegate: rng.gen(),
            delegate: rng.gen::<u8>(),
            mint: rng.gen::<u8>(),
            version: rng.gen::<u8>(),
            root_index: rng.gen(),
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen(),
                queue_pubkey_index: rng.gen(),
                leaf_index: rng.gen(),
                prove_by_index: rng.gen(),
            },
        })
        .collect();

    // Build accounts: 7-account header + n_pdas PDAs + n_tokens token accounts.
    let (config_account, rent_sponsor_key) = make_config_account(program_id);
    let mut fee_payer = make_dummy_account([1u8; 32], [0u8; 32], 0);
    let mut config_account = config_account;
    let mut rent_sponsor = make_dummy_account(rent_sponsor_key, [0u8; 32], 0);
    let mut ctoken_rent_sponsor = make_dummy_account([3u8; 32], [0u8; 32], 0);
    let mut dummy4 = make_dummy_account([4u8; 32], [0u8; 32], 0);
    let mut dummy5 = make_dummy_account([5u8; 32], [0u8; 32], 0);
    let mut ctoken_config = make_dummy_account([6u8; 32], [0u8; 32], 0);
    let mut pda_accounts: Vec<TestAccount> = (0..n_pdas)
        .map(|i| make_dummy_account([(10 + i) as u8; 32], program_id, 100))
        .collect();
    let mut token_accounts: Vec<TestAccount> = (0..n_tokens)
        .map(|i| make_dummy_account([(20 + i) as u8; 32], program_id, 100))
        .collect();

    let fee_payer_ai = fee_payer.get_account_info();
    let config_ai = config_account.get_account_info();
    let rent_sponsor_ai = rent_sponsor.get_account_info();
    let ctoken_rs_ai = ctoken_rent_sponsor.get_account_info();
    let dummy4_ai = dummy4.get_account_info();
    let dummy5_ai = dummy5.get_account_info();
    let ctoken_config_ai = ctoken_config.get_account_info();
    let mut pda_ais: Vec<AccountInfo<'_>> =
        pda_accounts.iter_mut().map(|a| a.get_account_info()).collect();
    let mut token_ais: Vec<AccountInfo<'_>> =
        token_accounts.iter_mut().map(|a| a.get_account_info()).collect();

    let mut remaining_accounts = vec![
        fee_payer_ai,
        config_ai,
        rent_sponsor_ai,
        ctoken_rs_ai,
        dummy4_ai,
        dummy5_ai,
        ctoken_config_ai,
    ];
    remaining_accounts.append(&mut pda_ais);
    remaining_accounts.append(&mut token_ais);

    // Build params.accounts: PDAs first, then tokens.
    let mut accounts: Vec<CompressedAccountData<MockVariant>> = Vec::new();
    for info in &expected_pda_infos {
        accounts.push(CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: MockVariant::Pda(PdaMockVariant(info.clone())),
        });
    }
    for token in &expected_token_data {
        accounts.push(CompressedAccountData {
            tree_info: PackedStateTreeInfo::default(),
            data: MockVariant::Token(TokenMockVariant(*token)),
        });
    }

    let params = DecompressIdempotentParams {
        system_accounts_offset: 7,
        token_accounts_offset: n_pdas as u8,
        output_queue_index: 0,
        proof: ValidityProof::default(),
        accounts,
    };
    let cpi_signer = CpiSigner { program_id, cpi_signer: [0u8; 32], bump: 255 };

    let result = build_decompress_accounts_cpi_data(
        &remaining_accounts,
        &params,
        cpi_signer,
        &program_id,
        0,
    );

    let built = result.unwrap();
    assert_eq!(
        built.compressed_account_infos, expected_pda_infos,
        "n_pdas={n_pdas} n_tokens={n_tokens}"
    );
    assert_eq!(
        built.in_token_data, expected_token_data,
        "n_pdas={n_pdas} n_tokens={n_tokens}"
    );
    assert_eq!(
        built.has_pda_accounts,
        n_pdas > 0,
        "n_pdas={n_pdas} n_tokens={n_tokens}"
    );
    assert_eq!(
        built.has_token_accounts,
        n_tokens > 0,
        "n_pdas={n_pdas} n_tokens={n_tokens}"
    );
    assert_eq!(
        built.cpi_context,
        n_pdas > 0 && n_tokens > 0,
        "n_pdas={n_pdas} n_tokens={n_tokens}"
    );
}
