use borsh::BorshDeserialize;
use light_compressed_account::TreeType;
use light_token::compat::{AccountState, TokenData};
use light_token_interface::state::ExtensionStruct;
use solana_account::Account;
use solana_pubkey::Pubkey;

use super::super::{base58::decode_base58_to_fixed_array, IndexerError};

/// Re-export solana Account for interface types.
pub type SolanaAccountData = Account;

/// Merkle tree info for compressed accounts
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceTreeInfo {
    pub tree: Pubkey,
    pub queue: Pubkey,
    pub tree_type: TreeType,
    pub seq: Option<u64>,
    /// Slot when the account was created/compressed
    pub slot_created: u64,
}

/// Structured compressed account data (discriminator separated)
#[derive(Clone, Debug, PartialEq)]
pub struct ColdData {
    pub discriminator: [u8; 8],
    pub data: Vec<u8>,
}

/// Compressed account context — present when account is in compressed state
#[derive(Clone, Debug, PartialEq)]
pub enum ColdContext {
    Account {
        hash: [u8; 32],
        leaf_index: u64,
        tree_info: InterfaceTreeInfo,
        data: ColdData,
    },
    Token {
        hash: [u8; 32],
        leaf_index: u64,
        tree_info: InterfaceTreeInfo,
        data: ColdData,
    },
}

/// Decode tree info from photon_api format
fn decode_tree_info(
    tree_info: &photon_api::types::TreeInfo,
) -> Result<InterfaceTreeInfo, IndexerError> {
    let tree = Pubkey::new_from_array(decode_base58_to_fixed_array(&tree_info.tree)?);
    let queue = Pubkey::new_from_array(decode_base58_to_fixed_array(&tree_info.queue)?);
    let tree_type = match tree_info.tree_type {
        photon_api::types::TreeType::StateV1 => TreeType::StateV1,
        photon_api::types::TreeType::StateV2 => TreeType::StateV2,
    };
    Ok(InterfaceTreeInfo {
        tree,
        queue,
        tree_type,
        seq: tree_info.seq.as_ref().map(|s| **s),
        slot_created: *tree_info.slot_created,
    })
}

/// Decode cold data from photon_api format.
fn decode_cold_data(data: &photon_api::types::ColdData) -> Result<ColdData, IndexerError> {
    if data.discriminator.len() != 8 {
        return Err(IndexerError::decode_error(
            "discriminator",
            format!("expected 8 bytes, got {}", data.discriminator.len()),
        ));
    }
    let mut discriminator = [0u8; 8];
    for (i, &val) in data.discriminator.iter().enumerate() {
        discriminator[i] = val as u8;
    }
    Ok(ColdData {
        discriminator,
        data: base64::decode_config(&*data.data, base64::STANDARD_NO_PAD)
            .map_err(|e| IndexerError::decode_error("data", e))?,
    })
}

/// Helper to convert photon_api ColdContext to client ColdContext
fn convert_cold_context(
    cold: &photon_api::types::ColdContext,
) -> Result<ColdContext, IndexerError> {
    match cold {
        photon_api::types::ColdContext::Account {
            hash,
            leaf_index,
            tree_info,
            data,
        } => Ok(ColdContext::Account {
            hash: decode_base58_to_fixed_array(hash)?,
            leaf_index: **leaf_index,
            tree_info: decode_tree_info(tree_info)?,
            data: decode_cold_data(data)?,
        }),
        photon_api::types::ColdContext::Token {
            hash,
            leaf_index,
            tree_info,
            data,
        } => Ok(ColdContext::Token {
            hash: decode_base58_to_fixed_array(hash)?,
            leaf_index: **leaf_index,
            tree_info: decode_tree_info(tree_info)?,
            data: decode_cold_data(data)?,
        }),
    }
}

/// Unified account interface — works for both on-chain and compressed accounts
#[derive(Clone, Debug, PartialEq)]
pub struct AccountInterface {
    /// The on-chain Solana pubkey
    pub key: Pubkey,
    /// Standard Solana account fields
    pub account: SolanaAccountData,
    /// Compressed context — None if on-chain, Some if compressed
    pub cold: Option<ColdContext>,
}

impl AccountInterface {
    /// Returns true if this account is on-chain (hot)
    pub fn is_hot(&self) -> bool {
        self.cold.is_none()
    }

    /// Returns true if this account is compressed (cold)
    pub fn is_cold(&self) -> bool {
        self.cold.is_some()
    }
}

/// Helper to convert photon_api AccountInterface to client AccountInterface
fn convert_account_interface(
    ai: &photon_api::types::AccountInterface,
) -> Result<AccountInterface, IndexerError> {
    let cold = ai.cold.as_ref().map(convert_cold_context).transpose()?;

    let data = base64::decode_config(&*ai.account.data, base64::STANDARD_NO_PAD)
        .map_err(|e| IndexerError::decode_error("account.data", e))?;

    Ok(AccountInterface {
        key: Pubkey::new_from_array(decode_base58_to_fixed_array(&ai.key)?),
        account: Account {
            lamports: *ai.account.lamports,
            data,
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(&ai.account.owner)?),
            executable: ai.account.executable,
            rent_epoch: *ai.account.rent_epoch,
        },
        cold,
    })
}

/// Helper to convert flattened interface fields (from TokenAccountInterface or InterfaceResult variants)
/// into a client AccountInterface
fn convert_flattened_account_interface(
    key: &photon_api::types::SerializablePubkey,
    account: &photon_api::types::SolanaAccountData,
    cold: &Option<photon_api::types::ColdContext>,
) -> Result<AccountInterface, IndexerError> {
    let cold = cold.as_ref().map(convert_cold_context).transpose()?;

    let data = base64::decode_config(&*account.data, base64::STANDARD_NO_PAD)
        .map_err(|e| IndexerError::decode_error("account.data", e))?;

    Ok(AccountInterface {
        key: Pubkey::new_from_array(decode_base58_to_fixed_array(key)?),
        account: Account {
            lamports: *account.lamports,
            data,
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(&account.owner)?),
            executable: account.executable,
            rent_epoch: *account.rent_epoch,
        },
        cold,
    })
}

impl TryFrom<&photon_api::types::AccountInterface> for AccountInterface {
    type Error = IndexerError;

    fn try_from(ai: &photon_api::types::AccountInterface) -> Result<Self, Self::Error> {
        convert_account_interface(ai)
    }
}

impl TryFrom<&photon_api::types::InterfaceResult> for AccountInterface {
    type Error = IndexerError;

    fn try_from(ir: &photon_api::types::InterfaceResult) -> Result<Self, Self::Error> {
        match ir {
            photon_api::types::InterfaceResult::Variant0 {
                key, account, cold, ..
            } => convert_flattened_account_interface(key, account, cold),
            photon_api::types::InterfaceResult::Variant1 {
                key, account, cold, ..
            } => convert_flattened_account_interface(key, account, cold),
        }
    }
}

/// Token account interface with parsed token data
#[derive(Clone, Debug, PartialEq)]
pub struct TokenAccountInterface {
    /// Base account interface data
    pub account: AccountInterface,
    /// Parsed token data (same as CompressedTokenAccount.token)
    pub token: TokenData,
}

/// Parse token data from photon_api TokenData
fn parse_interface_token_data(
    td: &photon_api::types::TokenData,
) -> Result<TokenData, IndexerError> {
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

impl TryFrom<&photon_api::types::TokenAccountInterface> for TokenAccountInterface {
    type Error = IndexerError;

    fn try_from(tai: &photon_api::types::TokenAccountInterface) -> Result<Self, Self::Error> {
        // TokenAccountInterface has flattened AccountInterface fields + token_data
        let account = convert_flattened_account_interface(&tai.key, &tai.account, &tai.cold)?;
        let token = parse_interface_token_data(&tai.token_data)?;
        Ok(TokenAccountInterface { account, token })
    }
}
