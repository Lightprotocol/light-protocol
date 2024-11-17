use crate::photon_rpc::types::{Base58Conversions, Hash};

#[derive(Debug)]
pub struct CompressedAccount {
    pub hash: Hash,
    pub data: String,
    pub owner: String,
    pub lamports: u64,
    pub executable: bool,
    pub rent_epoch: u64,
}

#[derive(Debug)]
pub struct TokenAccountBalance {
    pub amount: String,
}

#[derive(Debug)]
pub struct AccountBalance {
    pub lamports: u64,
}

#[derive(Debug)]
pub struct CompressedAccountResponse {
    pub context: ResponseContext,
    pub value: CompressedAccount,
}

#[derive(Debug)]
pub struct CompressedAccountsResponse {
    pub context: ResponseContext,
    pub value: Vec<CompressedAccount>,
}

#[derive(Debug)]
pub struct TokenAccountBalanceResponse {
    pub context: ResponseContext,
    pub value: TokenAccountBalance,
}

#[derive(Debug)]
pub struct AccountBalanceResponse {
    pub context: ResponseContext,
    pub value: AccountBalance,
}

#[derive(Debug)]
pub struct ResponseContext {
    pub slot: u64,
}

impl From<photon_api::models::Context> for ResponseContext {
    fn from(ctx: photon_api::models::Context) -> Self {
        ResponseContext {
            slot: ctx.slot as u64,
        }
    }
}

impl From<photon_api::models::GetCompressedAccountPost200ResponseResult>
    for CompressedAccountResponse
{
    fn from(result: photon_api::models::GetCompressedAccountPost200ResponseResult) -> Self {
        CompressedAccountResponse {
            context: ResponseContext::from(*result.context),
            value: CompressedAccount {
                hash: Hash::from_base58(&result.value.as_ref().unwrap().hash).unwrap(),
                data: result
                    .value
                    .as_ref()
                    .unwrap()
                    .data
                    .as_ref()
                    .unwrap()
                    .data
                    .clone(),
                owner: result.value.as_ref().unwrap().owner.clone(),
                lamports: result.value.as_ref().unwrap().lamports as u64,
                executable: false,
                rent_epoch: result.value.as_ref().unwrap().slot_created as u64,
            },
        }
    }
}

impl From<photon_api::models::GetCompressedTokenAccountBalancePost200ResponseResult>
    for TokenAccountBalanceResponse
{
    fn from(
        result: photon_api::models::GetCompressedTokenAccountBalancePost200ResponseResult,
    ) -> Self {
        TokenAccountBalanceResponse {
            context: ResponseContext::from(*result.context),
            value: TokenAccountBalance {
                amount: result.value.amount.to_string(),
            },
        }
    }
}

impl From<photon_api::models::GetCompressedAccountBalancePost200ResponseResult>
    for AccountBalanceResponse
{
    fn from(result: photon_api::models::GetCompressedAccountBalancePost200ResponseResult) -> Self {
        AccountBalanceResponse {
            context: ResponseContext::from(*result.context),
            value: AccountBalance {
                lamports: result.value as u64,
            },
        }
    }
}

impl From<photon_api::models::GetMultipleCompressedAccountsPost200ResponseResult>
    for CompressedAccountsResponse
{
    fn from(
        result: photon_api::models::GetMultipleCompressedAccountsPost200ResponseResult,
    ) -> Self {
        CompressedAccountsResponse {
            context: ResponseContext::from(*result.context),
            value: result
                .value
                .items
                .iter()
                .map(|acc| CompressedAccount {
                    hash: Hash::from_base58(&acc.hash).unwrap(),
                    data: acc.data.as_ref().unwrap().data.clone(),
                    owner: acc.owner.clone(),
                    lamports: acc.lamports as u64,
                    executable: false,
                    rent_epoch: acc.slot_created as u64,
                })
                .collect(),
        }
    }
}
