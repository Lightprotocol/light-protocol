use borsh::BorshDeserialize;
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_token::compat::{AccountState, TokenData};
use light_token_interface::state::ExtensionStruct;
use solana_pubkey::Pubkey;

use super::{
    super::{base58::decode_base58_to_fixed_array, IndexerError},
    account::CompressedAccount,
};

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CompressedTokenAccount {
    /// Token-specific data (mint, owner, amount, delegate, state, tlv)
    pub token: TokenData,
    /// General account information (address, hash, lamports, merkle context, etc.)
    pub account: CompressedAccount,
}

fn parse_token_data(td: &photon_api::types::TokenData) -> Result<TokenData, IndexerError> {
    Ok(TokenData {
        mint: Pubkey::new_from_array(decode_base58_to_fixed_array(&td.mint)?),
        owner: Pubkey::new_from_array(decode_base58_to_fixed_array(&td.owner)?),
        amount: *td.amount,
        delegate: td
            .delegate
            .as_ref()
            .map(|d| decode_base58_to_fixed_array(d).map(Pubkey::new_from_array))
            .transpose()?,
        state: match td.state {
            photon_api::types::AccountState::Initialized => AccountState::Initialized,
            photon_api::types::AccountState::Frozen => AccountState::Frozen,
        },
        tlv: td
            .tlv
            .as_ref()
            .map(|tlv| {
                let bytes = base64::decode_config(&**tlv, base64::STANDARD_NO_PAD)
                    .map_err(|e| IndexerError::decode_error("tlv", e))?;
                Vec::<ExtensionStruct>::deserialize(&mut bytes.as_slice())
                    .map_err(|e| IndexerError::decode_error("extensions", e))
            })
            .transpose()?,
    })
}

impl TryFrom<&photon_api::types::TokenAccount> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(token_account: &photon_api::types::TokenAccount) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(&token_account.account)?;
        let token = parse_token_data(&token_account.token_data)?;
        Ok(CompressedTokenAccount { token, account })
    }
}

impl TryFrom<&photon_api::types::TokenAccountV2> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(token_account: &photon_api::types::TokenAccountV2) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(&token_account.account)?;
        let token = parse_token_data(&token_account.token_data)?;
        Ok(CompressedTokenAccount { token, account })
    }
}

#[allow(clippy::from_over_into)]
impl Into<light_token::compat::TokenDataWithMerkleContext> for CompressedTokenAccount {
    fn into(self) -> light_token::compat::TokenDataWithMerkleContext {
        let compressed_account = CompressedAccountWithMerkleContext::from(self.account);

        light_token::compat::TokenDataWithMerkleContext {
            token_data: self.token,
            compressed_account,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<light_token::compat::TokenDataWithMerkleContext>>
    for super::super::response::Response<
        super::super::response::ItemsWithCursor<CompressedTokenAccount>,
    >
{
    fn into(self) -> Vec<light_token::compat::TokenDataWithMerkleContext> {
        self.value
            .items
            .into_iter()
            .map(
                |token_account| light_token::compat::TokenDataWithMerkleContext {
                    token_data: token_account.token,
                    compressed_account: CompressedAccountWithMerkleContext::from(
                        token_account.account.clone(),
                    ),
                },
            )
            .collect::<Vec<light_token::compat::TokenDataWithMerkleContext>>()
    }
}

impl TryFrom<light_token::compat::TokenDataWithMerkleContext> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(
        token_data_with_context: light_token::compat::TokenDataWithMerkleContext,
    ) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(token_data_with_context.compressed_account)?;

        Ok(CompressedTokenAccount {
            token: token_data_with_context.token_data,
            account,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TokenBalance {
    pub balance: u64,
    pub mint: Pubkey,
}

impl TryFrom<&photon_api::types::TokenBalance> for TokenBalance {
    type Error = IndexerError;

    fn try_from(token_balance: &photon_api::types::TokenBalance) -> Result<Self, Self::Error> {
        Ok(TokenBalance {
            balance: *token_balance.balance,
            mint: Pubkey::new_from_array(decode_base58_to_fixed_array(&token_balance.mint)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OwnerBalance {
    pub balance: u64,
    pub owner: Pubkey,
}

impl TryFrom<&photon_api::types::OwnerBalance> for OwnerBalance {
    type Error = IndexerError;

    fn try_from(owner_balance: &photon_api::types::OwnerBalance) -> Result<Self, Self::Error> {
        Ok(OwnerBalance {
            balance: *owner_balance.balance,
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(&owner_balance.owner)?),
        })
    }
}
