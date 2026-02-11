use light_compressed_account::TreeType;
use light_token::compat::TokenData;
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
    pub data_hash: [u8; 32],
}

/// Compressed account context — present when account is in compressed state.
#[derive(Clone, Debug, PartialEq)]
pub struct ColdContext {
    pub hash: [u8; 32],
    pub leaf_index: u64,
    pub tree_info: InterfaceTreeInfo,
    pub data: ColdData,
    pub address: Option<[u8; 32]>,
    pub prove_by_index: bool,
}

/// Decode tree info from photon_api AccountV2 format
fn decode_tree_info_v2(
    merkle_ctx: &photon_api::types::MerkleContextV2,
    seq: Option<u64>,
    slot_created: u64,
) -> Result<InterfaceTreeInfo, IndexerError> {
    let tree = Pubkey::new_from_array(decode_base58_to_fixed_array(&merkle_ctx.tree)?);
    let queue = Pubkey::new_from_array(decode_base58_to_fixed_array(&merkle_ctx.queue)?);
    let tree_type = TreeType::from(merkle_ctx.tree_type as u64);
    Ok(InterfaceTreeInfo {
        tree,
        queue,
        tree_type,
        seq,
        slot_created,
    })
}

/// Decode cold data from photon_api AccountData format.
fn decode_account_data(data: &photon_api::types::AccountData) -> Result<ColdData, IndexerError> {
    let disc_val = *data.discriminator;
    let discriminator = disc_val.to_le_bytes();
    Ok(ColdData {
        discriminator,
        data: base64::decode_config(&*data.data, base64::STANDARD_NO_PAD)
            .map_err(|e| IndexerError::decode_error("data", e))?,
        data_hash: decode_base58_to_fixed_array(&data.data_hash)?,
    })
}

/// Convert a photon_api AccountV2 to a client ColdContext.
fn convert_account_v2(av2: &photon_api::types::AccountV2) -> Result<ColdContext, IndexerError> {
    let tree_info = decode_tree_info_v2(
        &av2.merkle_context,
        av2.seq.as_ref().map(|s| **s),
        *av2.slot_created,
    )?;

    let data = match &av2.data {
        Some(d) => decode_account_data(d)?,
        None => ColdData {
            discriminator: [0u8; 8],
            data: Vec::new(),
            data_hash: [0u8; 32],
        },
    };

    let address = av2
        .address
        .as_ref()
        .map(|a| decode_base58_to_fixed_array(a))
        .transpose()?;

    Ok(ColdContext {
        hash: decode_base58_to_fixed_array(&av2.hash)?,
        leaf_index: *av2.leaf_index,
        tree_info,
        data,
        address,
        prove_by_index: av2.prove_by_index,
    })
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
    /// Returns true if this account is on-chain (hot).
    pub fn is_hot(&self) -> bool {
        !self.is_cold()
    }

    /// Returns true if this account is compressed (cold).
    ///
    /// An account is truly cold only when compressed data exists AND the
    /// on-chain account is closed (lamports == 0). Decompressed accounts
    /// keep a compressed placeholder (DECOMPRESSED_PDA_DISCRIMINATOR) but
    /// are still on-chain with lamports > 0 — those are hot.
    pub fn is_cold(&self) -> bool {
        self.cold.is_some() && self.account.lamports == 0
    }
}

/// Helper to convert photon_api AccountInterface to client AccountInterface
fn convert_account_interface(
    ai: &photon_api::types::AccountInterface,
) -> Result<AccountInterface, IndexerError> {
    // Take the first compressed account entry if present
    let cold = ai
        .cold
        .as_ref()
        .and_then(|entries| entries.first())
        .map(convert_account_v2)
        .transpose()?;

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

impl TryFrom<&photon_api::types::AccountInterface> for AccountInterface {
    type Error = IndexerError;

    fn try_from(ai: &photon_api::types::AccountInterface) -> Result<Self, Self::Error> {
        convert_account_interface(ai)
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
