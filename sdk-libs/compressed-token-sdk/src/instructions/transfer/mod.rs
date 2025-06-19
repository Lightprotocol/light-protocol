use light_compressed_token_types::account_infos::{
    TransferAccountInfos as TransferAccountInfosTypes, TransferAccountInfosIndex,
};
use solana_account_info::AccountInfo;

pub mod account_metas;
pub mod instruction;

pub type TransferAccountInfos<'a, 'b> =
    TransferAccountInfosTypes<'a, AccountInfo<'b>, TransferAccountInfosIndex>;
