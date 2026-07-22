//! Read helpers with JS token-interface parity semantics.
//!
//! This module provides:
//! - `get_ata` / `get_ata_or_none` for unified light ATA reads (hot + primary cold)
//! - source-level account view helpers used by `Load`
//! - authority and cold-selection helpers mirroring JS token-interface behavior

use std::cmp::Ordering;

use borsh::BorshDeserialize;
use light_client::{
    indexer::{
        CompressedTokenAccount, GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer,
    },
    rpc::{Rpc, RpcError},
};
use light_token::{
    compat::{AccountState as CompressedAccountState, TokenData},
    constants::{LIGHT_TOKEN_PROGRAM_ID, SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID},
    instruction::get_associated_token_address,
};
use light_token_interface::state::{ExtensionStruct, Token};
use solana_pubkey::{pubkey, Pubkey};
use spl_token_2022::{
    solana_program::{program_option::COption, program_pack::Pack},
    state::{Account as SplTokenAccount, AccountState as SplAccountState},
};

const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SHA_FLAT_DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAccountSourceType {
    Spl,
    Token2022,
    SplCold,
    Token2022Cold,
    LightTokenHot,
    LightTokenCold,
}

impl TokenAccountSourceType {
    #[inline]
    pub fn is_cold(self) -> bool {
        matches!(
            self,
            Self::SplCold | Self::Token2022Cold | Self::LightTokenCold
        )
    }

    #[inline]
    fn priority(self) -> u8 {
        match self {
            Self::LightTokenHot => 0,
            Self::LightTokenCold => 1,
            Self::Spl => 2,
            Self::Token2022 => 3,
            Self::SplCold => 4,
            Self::Token2022Cold => 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenAccountSource {
    pub source_type: TokenAccountSourceType,
    pub address: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub delegated_amount: u64,
    pub is_initialized: bool,
    pub is_frozen: bool,
    /// Present for cold sources.
    pub compressed: Option<CompressedTokenAccount>,
}

impl TokenAccountSource {
    #[inline]
    pub fn is_cold(&self) -> bool {
        self.source_type.is_cold()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenInterfaceParsedAta {
    pub address: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub delegated_amount: u64,
    pub is_initialized: bool,
    pub is_frozen: bool,
}

#[derive(Debug, Clone)]
pub struct TokenInterfaceAccount {
    pub address: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub hot_amount: u64,
    pub compressed_amount: u64,
    pub has_hot_account: bool,
    pub requires_load: bool,
    pub parsed: TokenInterfaceParsedAta,
    pub compressed_account: Option<CompressedTokenAccount>,
    pub ignored_compressed_accounts: Vec<CompressedTokenAccount>,
    pub ignored_compressed_amount: u64,
}

#[derive(Debug, Clone)]
pub struct LoadAccountView {
    pub address: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub delegated_amount: u64,
    pub any_frozen: bool,
    pub sources: Vec<TokenAccountSource>,
}

#[derive(Debug, Clone, Copy)]
struct ParsedSourceData {
    amount: u64,
    delegate: Option<Pubkey>,
    delegated_amount: u64,
    is_initialized: bool,
    is_frozen: bool,
}

fn clamp_delegated_amount(amount: u64, delegated_amount: u64) -> u64 {
    amount.min(delegated_amount)
}

fn delegated_contribution(source: &TokenAccountSource) -> u64 {
    clamp_delegated_amount(source.amount, source.delegated_amount)
}

fn coption_to_option(coption: COption<Pubkey>) -> Option<Pubkey> {
    match coption {
        COption::Some(pk) => Some(pk),
        COption::None => None,
    }
}

fn get_compressed_only_delegated_amount(token_data: &TokenData) -> Option<u64> {
    token_data.tlv.as_ref().and_then(|extensions| {
        extensions.iter().find_map(|extension| match extension {
            ExtensionStruct::CompressedOnly(compressed_only) => {
                Some(compressed_only.delegated_amount)
            }
            _ => None,
        })
    })
}

fn parse_light_hot_source_data(data: &[u8]) -> Option<ParsedSourceData> {
    let token = Token::deserialize(&mut &data[..]).ok()?;
    Some(ParsedSourceData {
        amount: token.amount,
        delegate: token
            .delegate
            .map(|delegate| Pubkey::new_from_array(delegate.to_bytes())),
        delegated_amount: token.delegated_amount,
        is_initialized: token.state != light_token_interface::state::AccountState::Uninitialized,
        is_frozen: token.state == light_token_interface::state::AccountState::Frozen,
    })
}

fn parse_spl_source_data(data: &[u8], owner: &Pubkey, mint: &Pubkey) -> Option<ParsedSourceData> {
    if data.len() < SplTokenAccount::LEN {
        return None;
    }

    let account = SplTokenAccount::unpack(&data[..SplTokenAccount::LEN]).ok()?;
    if &account.owner != owner || &account.mint != mint {
        return None;
    }

    let delegate = coption_to_option(account.delegate);
    let delegated_amount = if delegate.is_some() {
        account.delegated_amount
    } else {
        0
    };

    Some(ParsedSourceData {
        amount: account.amount,
        delegate,
        delegated_amount,
        is_initialized: account.state != SplAccountState::Uninitialized,
        is_frozen: account.state == SplAccountState::Frozen,
    })
}

fn parse_cold_source_data(token_data: &TokenData) -> ParsedSourceData {
    let delegated_amount = get_compressed_only_delegated_amount(token_data).unwrap_or_else(|| {
        if token_data.delegate.is_some() {
            token_data.amount
        } else {
            0
        }
    });

    ParsedSourceData {
        amount: token_data.amount,
        delegate: token_data.delegate,
        delegated_amount,
        is_initialized: true,
        is_frozen: token_data.state == CompressedAccountState::Frozen,
    }
}

fn compute_canonical_delegate(sources: &[TokenAccountSource]) -> (Option<Pubkey>, u64) {
    let hot_delegate_source = sources
        .iter()
        .find(|source| !source.is_cold() && source.delegate.is_some());

    if let Some(source) = hot_delegate_source {
        let delegate = source.delegate;
        let delegated_amount = sources
            .iter()
            .filter(|candidate| candidate.delegate == delegate)
            .fold(0u64, |sum, candidate| {
                sum.saturating_add(delegated_contribution(candidate))
            });
        return (delegate, delegated_amount);
    }

    let cold_delegate_source = sources
        .iter()
        .find(|source| source.is_cold() && source.delegate.is_some());

    if let Some(source) = cold_delegate_source {
        let delegate = source.delegate;
        let delegated_amount = sources
            .iter()
            .filter(|candidate| candidate.is_cold() && candidate.delegate == delegate)
            .fold(0u64, |sum, candidate| {
                sum.saturating_add(delegated_contribution(candidate))
            });
        return (delegate, delegated_amount);
    }

    (None, 0)
}

fn sort_sources_by_priority(sources: &mut [TokenAccountSource]) {
    sources.sort_by_key(|source| source.source_type.priority());
}

fn build_load_account_view(
    address: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    mut sources: Vec<TokenAccountSource>,
) -> LoadAccountView {
    sort_sources_by_priority(&mut sources);

    let amount = sources
        .iter()
        .fold(0u64, |sum, source| sum.saturating_add(source.amount));
    let any_frozen = sources.iter().any(|source| source.is_frozen);
    let (delegate, delegated_amount) = compute_canonical_delegate(&sources);

    LoadAccountView {
        address,
        owner,
        mint,
        amount,
        delegate,
        delegated_amount: clamp_delegated_amount(amount, delegated_amount),
        any_frozen,
        sources,
    }
}

fn build_parsed_ata(
    address: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    hot: Option<ParsedSourceData>,
    cold: Option<ParsedSourceData>,
) -> TokenInterfaceParsedAta {
    let hot_amount = hot.map(|value| value.amount).unwrap_or(0);
    let compressed_amount = cold.map(|value| value.amount).unwrap_or(0);
    let amount = hot_amount.saturating_add(compressed_amount);

    let mut delegate = None;
    let mut delegated_amount = 0u64;

    if let Some(hot_source) = hot {
        if hot_source.delegate.is_some() {
            delegate = hot_source.delegate;
            delegated_amount = hot_source.delegated_amount;
            if let Some(cold_source) = cold {
                if cold_source.delegate == delegate {
                    delegated_amount = delegated_amount.saturating_add(clamp_delegated_amount(
                        cold_source.amount,
                        cold_source.delegated_amount,
                    ));
                }
            }
        }
    } else if let Some(cold_source) = cold {
        if cold_source.delegate.is_some() {
            delegate = cold_source.delegate;
            delegated_amount =
                clamp_delegated_amount(cold_source.amount, cold_source.delegated_amount);
        }
    }

    TokenInterfaceParsedAta {
        address,
        owner,
        mint,
        amount,
        delegate,
        delegated_amount: clamp_delegated_amount(amount, delegated_amount),
        is_initialized: hot.map(|value| value.is_initialized).unwrap_or(false) || cold.is_some(),
        is_frozen: hot.map(|value| value.is_frozen).unwrap_or(false)
            || cold.map(|value| value.is_frozen).unwrap_or(false),
    }
}

fn sorted_primary_cold_candidates(
    accounts: &[CompressedTokenAccount],
) -> Vec<CompressedTokenAccount> {
    let mut candidates = accounts
        .iter()
        .filter(|account| {
            account.account.owner == LIGHT_TOKEN_PROGRAM_ID
                && account
                    .account
                    .data
                    .as_ref()
                    .is_some_and(|data| !data.data.is_empty())
                && account.token.amount > 0
        })
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        let amount_cmp = right.token.amount.cmp(&left.token.amount);
        if amount_cmp != Ordering::Equal {
            return amount_cmp;
        }
        right.account.leaf_index.cmp(&left.account.leaf_index)
    });

    candidates
}

fn derive_spl_associated_token_address(
    owner: &Pubkey,
    mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

pub fn is_authority_for_account(view: &LoadAccountView, authority: &Pubkey) -> bool {
    *authority == view.owner || view.delegate.is_some_and(|delegate| delegate == *authority)
}

pub fn filter_account_for_authority(view: &LoadAccountView, authority: &Pubkey) -> LoadAccountView {
    if *authority == view.owner {
        return view.clone();
    }

    if view.delegate != Some(*authority) {
        return LoadAccountView {
            address: view.address,
            owner: view.owner,
            mint: view.mint,
            amount: 0,
            delegate: view.delegate,
            delegated_amount: 0,
            any_frozen: view.any_frozen,
            sources: Vec::new(),
        };
    }

    let filtered_sources = view
        .sources
        .iter()
        .filter(|source| source.delegate == Some(*authority))
        .cloned()
        .collect::<Vec<_>>();

    if filtered_sources.is_empty() {
        return LoadAccountView {
            address: view.address,
            owner: view.owner,
            mint: view.mint,
            amount: 0,
            delegate: view.delegate,
            delegated_amount: 0,
            any_frozen: view.any_frozen,
            sources: Vec::new(),
        };
    }

    build_load_account_view(view.address, view.owner, view.mint, filtered_sources)
}

pub fn select_primary_cold_account_for_load(
    sources: &[TokenAccountSource],
) -> Option<CompressedTokenAccount> {
    let mut candidates = sources
        .iter()
        .filter_map(|source| {
            if source.is_cold() && source.amount > 0 {
                source.compressed.clone()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        let amount_cmp = right.token.amount.cmp(&left.token.amount);
        if amount_cmp != Ordering::Equal {
            return amount_cmp;
        }
        right.account.leaf_index.cmp(&left.account.leaf_index)
    });

    candidates.into_iter().next()
}

pub async fn get_ata_or_none<R: Rpc + Indexer>(
    rpc: &R,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Option<TokenInterfaceAccount>, RpcError> {
    let light_ata = get_associated_token_address(&owner, &mint);
    let hot_account = rpc.get_account(light_ata).await?;
    let compressed_response = rpc
        .get_compressed_token_accounts_by_owner(
            &owner,
            Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(
                Some(mint),
            )),
            None,
        )
        .await
        .map_err(|error| {
            RpcError::CustomError(format!("Failed to fetch compressed accounts: {error}"))
        })?;

    let hot_parsed = hot_account.as_ref().and_then(|account| {
        if account.owner == LIGHT_TOKEN_PROGRAM_ID {
            parse_light_hot_source_data(&account.data)
        } else {
            None
        }
    });

    let sorted_candidates = sorted_primary_cold_candidates(&compressed_response.value.items);
    let selected_cold = sorted_candidates.first().cloned();
    let ignored_cold = sorted_candidates
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>();
    let ignored_compressed_amount = ignored_cold.iter().fold(0u64, |sum, account| {
        sum.saturating_add(account.token.amount)
    });
    let cold_parsed = selected_cold
        .as_ref()
        .map(|account| parse_cold_source_data(&account.token));

    if hot_parsed.is_none() && cold_parsed.is_none() {
        return Ok(None);
    }

    let parsed = build_parsed_ata(light_ata, owner, mint, hot_parsed, cold_parsed);

    Ok(Some(TokenInterfaceAccount {
        address: light_ata,
        owner,
        mint,
        amount: parsed.amount,
        hot_amount: hot_parsed.map(|value| value.amount).unwrap_or(0),
        compressed_amount: cold_parsed.map(|value| value.amount).unwrap_or(0),
        has_hot_account: hot_parsed.is_some(),
        requires_load: selected_cold.is_some(),
        parsed,
        compressed_account: selected_cold,
        ignored_compressed_accounts: ignored_cold,
        ignored_compressed_amount,
    }))
}

pub async fn get_ata<R: Rpc + Indexer>(
    rpc: &R,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<TokenInterfaceAccount, RpcError> {
    get_ata_or_none(rpc, owner, mint)
        .await?
        .ok_or_else(|| RpcError::CustomError("Associated token account not found".to_string()))
}

pub async fn get_ata_view_for_load_or_none<R: Rpc + Indexer>(
    rpc: &R,
    owner: Pubkey,
    mint: Pubkey,
    wrap: bool,
) -> Result<Option<LoadAccountView>, RpcError> {
    let light_ata = get_associated_token_address(&owner, &mint);
    let mut sources = Vec::<TokenAccountSource>::new();

    let light_hot_account = rpc.get_account(light_ata).await?;
    if let Some(account) = light_hot_account {
        if account.owner == LIGHT_TOKEN_PROGRAM_ID {
            if let Some(parsed) = parse_light_hot_source_data(&account.data) {
                sources.push(TokenAccountSource {
                    source_type: TokenAccountSourceType::LightTokenHot,
                    address: light_ata,
                    amount: parsed.amount,
                    delegate: parsed.delegate,
                    delegated_amount: parsed.delegated_amount,
                    is_initialized: parsed.is_initialized,
                    is_frozen: parsed.is_frozen,
                    compressed: None,
                });
            }
        }
    }

    if wrap {
        let spl_ata = derive_spl_associated_token_address(&owner, &mint, &SPL_TOKEN_PROGRAM_ID);
        if let Some(account) = rpc.get_account(spl_ata).await? {
            if account.owner == SPL_TOKEN_PROGRAM_ID {
                if let Some(parsed) = parse_spl_source_data(&account.data, &owner, &mint) {
                    sources.push(TokenAccountSource {
                        source_type: TokenAccountSourceType::Spl,
                        address: spl_ata,
                        amount: parsed.amount,
                        delegate: parsed.delegate,
                        delegated_amount: parsed.delegated_amount,
                        is_initialized: parsed.is_initialized,
                        is_frozen: parsed.is_frozen,
                        compressed: None,
                    });
                }
            }
        }

        let t22_ata =
            derive_spl_associated_token_address(&owner, &mint, &SPL_TOKEN_2022_PROGRAM_ID);
        if let Some(account) = rpc.get_account(t22_ata).await? {
            if account.owner == SPL_TOKEN_2022_PROGRAM_ID {
                if let Some(parsed) = parse_spl_source_data(&account.data, &owner, &mint) {
                    sources.push(TokenAccountSource {
                        source_type: TokenAccountSourceType::Token2022,
                        address: t22_ata,
                        amount: parsed.amount,
                        delegate: parsed.delegate,
                        delegated_amount: parsed.delegated_amount,
                        is_initialized: parsed.is_initialized,
                        is_frozen: parsed.is_frozen,
                        compressed: None,
                    });
                }
            }
        }
    }

    let compressed_response = rpc
        .get_compressed_token_accounts_by_owner(
            &owner,
            Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(
                Some(mint),
            )),
            None,
        )
        .await
        .map_err(|error| {
            RpcError::CustomError(format!("Failed to fetch compressed accounts: {error}"))
        })?;

    for account in compressed_response.value.items {
        if account.account.owner != LIGHT_TOKEN_PROGRAM_ID {
            continue;
        }
        if account
            .account
            .data
            .as_ref()
            .map(|data| data.data.is_empty())
            .unwrap_or(true)
        {
            continue;
        }
        let parsed = parse_cold_source_data(&account.token);
        sources.push(TokenAccountSource {
            source_type: TokenAccountSourceType::LightTokenCold,
            address: light_ata,
            amount: parsed.amount,
            delegate: parsed.delegate,
            delegated_amount: parsed.delegated_amount,
            is_initialized: parsed.is_initialized,
            is_frozen: parsed.is_frozen,
            compressed: Some(account),
        });
    }

    if sources.is_empty() {
        return Ok(None);
    }

    Ok(Some(build_load_account_view(
        light_ata, owner, mint, sources,
    )))
}

pub fn default_token_data_discriminator(compressed_account: &CompressedTokenAccount) -> [u8; 8] {
    compressed_account
        .account
        .data
        .as_ref()
        .map(|data| data.discriminator)
        .unwrap_or(SHA_FLAT_DISCRIMINATOR)
}

#[cfg(test)]
mod tests {
    use light_client::indexer::{CompressedAccount, CompressedTokenAccount, TreeInfo};
    use light_compressed_account::{compressed_account::CompressedAccountData, TreeType};

    use super::*;

    fn make_source(
        source_type: TokenAccountSourceType,
        amount: u64,
        delegate: Option<Pubkey>,
        delegated_amount: u64,
        is_frozen: bool,
    ) -> TokenAccountSource {
        TokenAccountSource {
            source_type,
            address: Pubkey::new_unique(),
            amount,
            delegate,
            delegated_amount,
            is_initialized: true,
            is_frozen,
            compressed: None,
        }
    }

    fn make_cold_source(amount: u64, leaf_index: u32) -> TokenAccountSource {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let compressed = CompressedTokenAccount {
            token: TokenData {
                mint,
                owner,
                amount,
                delegate: None,
                state: CompressedAccountState::Initialized,
                tlv: None,
            },
            account: CompressedAccount {
                address: None,
                data: Some(CompressedAccountData {
                    discriminator: SHA_FLAT_DISCRIMINATOR,
                    data: vec![1, 2, 3],
                    data_hash: [0u8; 32],
                }),
                hash: [0u8; 32],
                lamports: 0,
                leaf_index,
                owner: LIGHT_TOKEN_PROGRAM_ID,
                prove_by_index: false,
                seq: None,
                slot_created: 0,
                tree_info: TreeInfo {
                    cpi_context: None,
                    next_tree_info: None,
                    queue: Pubkey::new_unique(),
                    tree: Pubkey::new_unique(),
                    tree_type: TreeType::StateV2,
                },
            },
        };

        TokenAccountSource {
            source_type: TokenAccountSourceType::LightTokenCold,
            address: Pubkey::new_unique(),
            amount,
            delegate: None,
            delegated_amount: 0,
            is_initialized: true,
            is_frozen: false,
            compressed: Some(compressed),
        }
    }

    #[test]
    fn js_parity_select_primary_cold_prefers_amount_then_leaf() {
        let sources = vec![
            make_cold_source(50, 10),
            make_cold_source(75, 2),
            make_cold_source(75, 9),
            make_cold_source(10, 99),
        ];

        let selected = select_primary_cold_account_for_load(&sources).expect("must select");
        assert_eq!(selected.token.amount, 75);
        assert_eq!(selected.account.leaf_index, 9);
    }

    #[test]
    fn js_parity_delegate_prefers_hot_delegate_and_sums_matching_sources() {
        let delegate = Pubkey::new_unique();
        let sources = vec![
            make_source(
                TokenAccountSourceType::LightTokenHot,
                100,
                Some(delegate),
                80,
                false,
            ),
            make_source(
                TokenAccountSourceType::LightTokenCold,
                60,
                Some(delegate),
                50,
                false,
            ),
            make_source(
                TokenAccountSourceType::LightTokenCold,
                40,
                Some(Pubkey::new_unique()),
                40,
                false,
            ),
        ];

        let view = build_load_account_view(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            sources,
        );

        assert_eq!(view.delegate, Some(delegate));
        assert_eq!(view.delegated_amount, 130);
    }

    #[test]
    fn js_parity_delegate_uses_first_cold_when_no_hot_delegate() {
        let first_delegate = Pubkey::new_unique();
        let sources = vec![
            make_source(
                TokenAccountSourceType::LightTokenCold,
                70,
                Some(first_delegate),
                70,
                false,
            ),
            make_source(
                TokenAccountSourceType::LightTokenCold,
                30,
                Some(first_delegate),
                10,
                false,
            ),
            make_source(TokenAccountSourceType::Spl, 20, None, 0, false),
        ];

        let view = build_load_account_view(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            sources,
        );

        assert_eq!(view.delegate, Some(first_delegate));
        assert_eq!(view.delegated_amount, 80);
    }

    #[test]
    fn js_parity_filter_for_authority_keeps_delegate_sources_only() {
        let owner = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let other_delegate = Pubkey::new_unique();
        let sources = vec![
            make_source(
                TokenAccountSourceType::LightTokenHot,
                100,
                Some(delegate),
                100,
                false,
            ),
            make_source(
                TokenAccountSourceType::LightTokenCold,
                40,
                Some(delegate),
                40,
                false,
            ),
            make_source(
                TokenAccountSourceType::LightTokenCold,
                30,
                Some(other_delegate),
                30,
                false,
            ),
        ];

        let view =
            build_load_account_view(Pubkey::new_unique(), owner, Pubkey::new_unique(), sources);
        let filtered = filter_account_for_authority(&view, &delegate);

        assert_eq!(filtered.sources.len(), 2);
        assert!(filtered
            .sources
            .iter()
            .all(|source| source.delegate == Some(delegate)));
        assert_eq!(filtered.amount, 140);
    }
}
