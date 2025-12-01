//! ATA Interface - unified balance view across SPL, T22, CToken hot/cold

use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::SPL_TOKEN_PROGRAM_ID;
use solana_pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

const SPL_ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSourceType {
    Spl,
    Token2022,
    CtokenHot,
    CtokenCold,
}

#[derive(Debug, Clone)]
pub struct TokenSource {
    pub source_type: TokenSourceType,
    pub address: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct AtaInterface {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub total_amount: u64,
    pub sources: Vec<TokenSource>,
    pub is_cold: bool,
}

impl AtaInterface {
    pub fn has_cold(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.source_type == TokenSourceType::CtokenCold)
    }

    pub fn has_spl(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.source_type == TokenSourceType::Spl)
    }

    pub fn has_t22(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.source_type == TokenSourceType::Token2022)
    }

    pub fn cold_balance(&self) -> u64 {
        self.sources
            .iter()
            .filter(|s| s.source_type == TokenSourceType::CtokenCold)
            .map(|s| s.amount)
            .sum()
    }

    pub fn hot_balance(&self) -> u64 {
        self.sources
            .iter()
            .filter(|s| s.source_type == TokenSourceType::CtokenHot)
            .map(|s| s.amount)
            .sum()
    }
}

fn get_spl_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

pub async fn get_ata_interface<R: Rpc + Indexer>(
    rpc: &mut R,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<AtaInterface, RpcError> {
    let mut sources = Vec::new();
    let mut total_amount: u64 = 0;

    let spl_token_program = Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID);

    // 1. Check SPL ATA
    let spl_ata = get_spl_ata(&owner, &mint, &spl_token_program);
    if let Some(spl_info) = rpc.get_account(spl_ata).await? {
        if let Ok(pod_account) = pod_from_bytes::<PodAccount>(&spl_info.data) {
            let balance: u64 = pod_account.amount.into();
            if balance > 0 {
                sources.push(TokenSource {
                    source_type: TokenSourceType::Spl,
                    address: spl_ata,
                    amount: balance,
                });
                total_amount += balance;
            }
        }
    }

    // 2. Check Token-2022 ATA
    let t22_ata = get_spl_ata(&owner, &mint, &TOKEN_2022_PROGRAM_ID);
    if let Some(t22_info) = rpc.get_account(t22_ata).await? {
        if let Ok(pod_account) = pod_from_bytes::<PodAccount>(&t22_info.data) {
            let balance: u64 = pod_account.amount.into();
            if balance > 0 {
                sources.push(TokenSource {
                    source_type: TokenSourceType::Token2022,
                    address: t22_ata,
                    amount: balance,
                });
                total_amount += balance;
            }
        }
    }

    // 3. Check compressed tokens (cold)
    let options = GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(Some(mint));
    let compressed_response = rpc
        .get_compressed_token_accounts_by_owner(&owner, Some(options), None)
        .await
        .map_err(|e| RpcError::CustomError(e.to_string()))?;

    let compressed_accounts = compressed_response.value.items;
    if !compressed_accounts.is_empty() {
        let cold_balance: u64 = compressed_accounts.iter().map(|acc| acc.token.amount).sum();
        if cold_balance > 0 {
            sources.push(TokenSource {
                source_type: TokenSourceType::CtokenCold,
                address: owner,
                amount: cold_balance,
            });
            total_amount += cold_balance;
        }
    }

    let is_cold = sources
        .iter()
        .any(|s| s.source_type == TokenSourceType::CtokenCold);

    Ok(AtaInterface {
        owner,
        mint,
        total_amount,
        sources,
        is_cold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn make_token_source(source_type: TokenSourceType, seed: u8, amount: u64) -> TokenSource {
        TokenSource {
            source_type,
            address: make_pubkey(seed),
            amount,
        }
    }

    fn make_ata_interface(sources: Vec<TokenSource>) -> AtaInterface {
        let total_amount = sources.iter().map(|s| s.amount).sum();
        let is_cold = sources
            .iter()
            .any(|s| s.source_type == TokenSourceType::CtokenCold);
        AtaInterface {
            owner: make_pubkey(1),
            mint: make_pubkey(2),
            total_amount,
            sources,
            is_cold,
        }
    }

    #[test]
    fn test_token_source_type_equality() {
        assert_eq!(TokenSourceType::Spl, TokenSourceType::Spl);
        assert_eq!(TokenSourceType::Token2022, TokenSourceType::Token2022);
        assert_eq!(TokenSourceType::CtokenHot, TokenSourceType::CtokenHot);
        assert_eq!(TokenSourceType::CtokenCold, TokenSourceType::CtokenCold);
        assert_ne!(TokenSourceType::Spl, TokenSourceType::Token2022);
        assert_ne!(TokenSourceType::CtokenHot, TokenSourceType::CtokenCold);
    }

    #[test]
    fn test_token_source_type_copy() {
        let t = TokenSourceType::Spl;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn test_token_source_creation() {
        let source = make_token_source(TokenSourceType::Spl, 1, 1000);
        assert_eq!(source.source_type, TokenSourceType::Spl);
        assert_eq!(source.amount, 1000);
    }

    #[test]
    fn test_ata_interface_empty_sources() {
        let ata = make_ata_interface(vec![]);
        assert!(!ata.has_cold());
        assert!(!ata.has_spl());
        assert!(!ata.has_t22());
        assert_eq!(ata.cold_balance(), 0);
        assert_eq!(ata.hot_balance(), 0);
        assert_eq!(ata.total_amount, 0);
        assert!(!ata.is_cold);
    }

    #[test]
    fn test_ata_interface_only_spl() {
        let ata = make_ata_interface(vec![make_token_source(TokenSourceType::Spl, 1, 500)]);
        assert!(!ata.has_cold());
        assert!(ata.has_spl());
        assert!(!ata.has_t22());
        assert_eq!(ata.cold_balance(), 0);
        assert_eq!(ata.hot_balance(), 0);
        assert_eq!(ata.total_amount, 500);
        assert!(!ata.is_cold);
    }

    #[test]
    fn test_ata_interface_only_t22() {
        let ata = make_ata_interface(vec![make_token_source(TokenSourceType::Token2022, 1, 750)]);
        assert!(!ata.has_cold());
        assert!(!ata.has_spl());
        assert!(ata.has_t22());
        assert_eq!(ata.cold_balance(), 0);
        assert_eq!(ata.hot_balance(), 0);
        assert_eq!(ata.total_amount, 750);
        assert!(!ata.is_cold);
    }

    #[test]
    fn test_ata_interface_only_cold() {
        let ata = make_ata_interface(vec![make_token_source(
            TokenSourceType::CtokenCold,
            1,
            1000,
        )]);
        assert!(ata.has_cold());
        assert!(!ata.has_spl());
        assert!(!ata.has_t22());
        assert_eq!(ata.cold_balance(), 1000);
        assert_eq!(ata.hot_balance(), 0);
        assert_eq!(ata.total_amount, 1000);
        assert!(ata.is_cold);
    }

    #[test]
    fn test_ata_interface_only_hot() {
        let ata = make_ata_interface(vec![make_token_source(TokenSourceType::CtokenHot, 1, 2000)]);
        assert!(!ata.has_cold());
        assert!(!ata.has_spl());
        assert!(!ata.has_t22());
        assert_eq!(ata.cold_balance(), 0);
        assert_eq!(ata.hot_balance(), 2000);
        assert_eq!(ata.total_amount, 2000);
        assert!(!ata.is_cold);
    }

    #[test]
    fn test_ata_interface_mixed_sources() {
        let ata = make_ata_interface(vec![
            make_token_source(TokenSourceType::Spl, 1, 100),
            make_token_source(TokenSourceType::Token2022, 2, 200),
            make_token_source(TokenSourceType::CtokenHot, 3, 300),
            make_token_source(TokenSourceType::CtokenCold, 4, 400),
        ]);
        assert!(ata.has_cold());
        assert!(ata.has_spl());
        assert!(ata.has_t22());
        assert_eq!(ata.cold_balance(), 400);
        assert_eq!(ata.hot_balance(), 300);
        assert_eq!(ata.total_amount, 1000);
        assert!(ata.is_cold);
    }

    #[test]
    fn test_ata_interface_multiple_cold_sources() {
        let ata = make_ata_interface(vec![
            make_token_source(TokenSourceType::CtokenCold, 1, 100),
            make_token_source(TokenSourceType::CtokenCold, 2, 200),
            make_token_source(TokenSourceType::CtokenCold, 3, 300),
        ]);
        assert!(ata.has_cold());
        assert_eq!(ata.cold_balance(), 600);
        assert_eq!(ata.total_amount, 600);
    }

    #[test]
    fn test_ata_interface_multiple_hot_sources() {
        let ata = make_ata_interface(vec![
            make_token_source(TokenSourceType::CtokenHot, 1, 50),
            make_token_source(TokenSourceType::CtokenHot, 2, 150),
        ]);
        assert!(!ata.has_cold());
        assert_eq!(ata.hot_balance(), 200);
        assert_eq!(ata.total_amount, 200);
    }

    #[test]
    fn test_get_spl_ata_deterministic() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(2);
        let program = make_pubkey(3);

        let ata1 = get_spl_ata(&owner, &mint, &program);
        let ata2 = get_spl_ata(&owner, &mint, &program);
        assert_eq!(ata1, ata2);

        // Different inputs should produce different results
        let other_owner = make_pubkey(10);
        let ata3 = get_spl_ata(&other_owner, &mint, &program);
        assert_ne!(ata1, ata3);
    }
}
