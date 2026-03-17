use borsh::{BorshDeserialize, BorshSerialize};
use light_account_checks::{
    account_info::test_account_info::solana_program::TestAccount, discriminator::Discriminator,
};
use light_compressible::rent::RentConfig;
use light_sdk_types::{
    error::LightSdkTypesError,
    instruction::PackedStateTreeInfo,
    interface::program::{
        config::{LightConfig, LIGHT_CONFIG_SEED, RENT_SPONSOR_SEED},
        decompression::processor::{DecompressCtx, DecompressVariant},
    },
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

/// Creates a fully valid LightConfig TestAccount for the given program_id.
/// Returns (config_account, rent_sponsor_key).
pub fn make_config_account(program_id: [u8; 32]) -> (TestAccount, [u8; 32]) {
    let prog = Pubkey::from(program_id);

    let config_bump_u16 = 0u16;
    let (config_pda, bump) =
        Pubkey::find_program_address(&[LIGHT_CONFIG_SEED, &config_bump_u16.to_le_bytes()], &prog);
    let (rent_sponsor_pda, rent_sponsor_bump) =
        Pubkey::find_program_address(&[RENT_SPONSOR_SEED], &prog);

    let config = LightConfig {
        version: 1,
        write_top_up: 1000,
        update_authority: [1u8; 32],
        rent_sponsor: rent_sponsor_pda.to_bytes(),
        compression_authority: [2u8; 32],
        rent_config: RentConfig::default(),
        config_bump: 0,
        bump,
        rent_sponsor_bump,
        address_space: vec![[3u8; 32]],
    };

    let mut data = LightConfig::LIGHT_DISCRIMINATOR.to_vec();
    config.serialize(&mut data).unwrap();

    let mut account = TestAccount::new(config_pda, Pubkey::from(program_id), data.len());
    account.data = data;

    (account, rent_sponsor_pda.to_bytes())
}

/// Creates a dummy writable TestAccount with the given key, owner, and data size.
pub fn make_dummy_account(key: [u8; 32], owner: [u8; 32], size: usize) -> TestAccount {
    TestAccount::new(Pubkey::from(key), Pubkey::from(owner), size)
}

/// Standard 5-account layout used by compress and decompress tests.
/// [0]=fee_payer, [1]=config, [2]=rent_sponsor, [3]=system_account, [4]=pda_account
pub fn make_valid_accounts(
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

/// Simulates an already-initialized PDA: pushes nothing to compressed_account_infos.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
#[allow(dead_code)]
pub struct SkipVariant;

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
