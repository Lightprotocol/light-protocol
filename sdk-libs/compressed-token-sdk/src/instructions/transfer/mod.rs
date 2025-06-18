use light_compressed_token_types::account_infos::{AccountInfos, TransferAccountInfosIndex};
use solana_account_info::AccountInfo;

pub mod account_metas;
pub mod instruction;

pub type TransferAccountInfos<'a, 'b> =
    AccountInfos<'a, AccountInfo<'b>, TransferAccountInfosIndex>;
