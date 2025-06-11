pub use light_sdk_types::CpiAccountsConfig;
use light_sdk_types::{
    CompressionCpiAccountIndex, CpiAccounts as GenericCpiAccounts, SYSTEM_ACCOUNTS_LEN,
};

use crate::{AccountInfo, AccountMeta, Pubkey};

pub type CpiAccounts<'c, 'info> = GenericCpiAccounts<'c, AccountInfo<'info>>;

pub fn to_account_metas(cpi_accounts: CpiAccounts<'_, '_>) -> Vec<AccountMeta> {
    let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.fee_payer().key,
        is_signer: true,
        is_writable: true,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.authority().key,
        is_signer: true,
        is_writable: false,
    });
    let accounts = cpi_accounts.account_infos();

    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndex::RegisteredProgramPda as usize].key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndex::NoopProgram as usize].key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndex::AccountCompressionAuthority as usize].key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndex::AccountCompressionProgram as usize].key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndex::InvokingProgram as usize].key,
        is_signer: false,
        is_writable: false,
    });
    let mut current_index = 7;
    if !cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta {
            pubkey: *cpi_accounts.light_system_program().key,
            is_signer: false,
            is_writable: false,
        });
    } else {
        account_metas.push(AccountMeta {
            pubkey: *accounts[current_index].key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }

    if !cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta {
            pubkey: *cpi_accounts.light_system_program().key,
            is_signer: false,
            is_writable: false,
        });
    } else {
        account_metas.push(AccountMeta {
            pubkey: *accounts[current_index].key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }
    // System program
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(),
        is_signer: false,
        is_writable: false,
    });
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta {
            pubkey: *cpi_accounts.light_system_program().key,
            is_signer: false,
            is_writable: false,
        });
    } else {
        account_metas.push(AccountMeta {
            pubkey: *accounts[current_index].key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }
    accounts[current_index..].iter().for_each(|acc| {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: true,
        });
    });
    account_metas
}
