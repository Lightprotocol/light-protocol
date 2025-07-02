use light_compressed_token_types::account_infos::TransferAccountInfos as TransferAccountInfosTypes;
use solana_account_info::AccountInfo;

pub mod account_infos;
pub mod account_metas;
pub mod instruction;

pub type TransferAccountInfos<'a, 'b> = TransferAccountInfosTypes<'a, AccountInfo<'b>>;
