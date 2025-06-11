use light_sdk_types::{
    CompressionCpiAccountIndexSmall, CpiAccountsSmall as GenericCpiAccountsSmall,
    PROGRAM_ACCOUNTS_LEN,
};

use crate::{AccountInfo, AccountMeta};

pub type CpiAccountsSmall<'c, 'info> = GenericCpiAccountsSmall<'c, AccountInfo<'info>>;

pub fn to_account_metas_small(cpi_accounts: CpiAccountsSmall<'_, '_>) -> Vec<AccountMeta> {
    // TODO: do a version with a const array instead of vector.
    let mut account_metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

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
        pubkey: *accounts[CompressionCpiAccountIndexSmall::RegisteredProgramPda as usize].key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *accounts[CompressionCpiAccountIndexSmall::AccountCompressionAuthority as usize]
            .key,
        is_signer: false,
        is_writable: false,
    });

    let mut index = CompressionCpiAccountIndexSmall::SolPoolPda as usize;
    if cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta {
            pubkey: *accounts[index].key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta {
            pubkey: *accounts[index].key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta {
            pubkey: *accounts[index].key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }
    assert_eq!(cpi_accounts.system_accounts_end_offset(), index);

    accounts[index..].iter().for_each(|acc| {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: true,
        });
    });
    account_metas
}
