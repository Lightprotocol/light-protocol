pub mod account_metas;
pub mod cpi_accounts;
pub mod instruction;

pub use account_metas::{
    get_mint_action_instruction_account_metas, get_mint_action_instruction_account_metas_cpi_write,
    MintActionMetaConfig, MintActionMetaConfigCpiWrite,
};
pub use cpi_accounts::MintActionCpiAccounts;

use crate::{AnchorDeserialize, AnchorSerialize};
use solana_pubkey::Pubkey;

// Backwards compatibility types for token-client
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct MintToRecipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

/// High-level action types for the mint action instruction (backwards compatibility)
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub enum MintActionType {
    MintTo {
        recipients: Vec<MintToRecipient>,
        token_account_version: u8,
    },
    UpdateMintAuthority {
        new_authority: Option<Pubkey>,
    },
    UpdateFreezeAuthority {
        new_authority: Option<Pubkey>,
    },
    MintToCToken {
        account: Pubkey,
        amount: u64,
    },
    UpdateMetadataField {
        extension_index: u8,
        field_type: u8,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    UpdateMetadataAuthority {
        extension_index: u8,
        new_authority: Pubkey,
    },
    RemoveMetadataKey {
        extension_index: u8,
        key: Vec<u8>,
        idempotent: u8,
    },
}
use light_account_checks::AccountInfoTrait;
use light_sdk::cpi::CpiSigner;

/// Account structure for mint action CPI write operations - follows the same pattern as CpiContextWriteAccounts
#[derive(Clone, Debug)]
pub struct MintActionCpiWriteAccounts<'a, T: AccountInfoTrait + Clone> {
    pub light_system_program: &'a T,
    pub mint_signer: Option<&'a T>, // Optional - only when creating mint and when creating SPL mint
    pub authority: &'a T,
    pub fee_payer: &'a T,
    pub cpi_authority_pda: &'a T,
    pub cpi_context: &'a T,
    pub cpi_signer: CpiSigner,
    pub recipient_token_accounts: Vec<&'a T>, // For mint_to_ctoken actions
}

impl<T: AccountInfoTrait + Clone> MintActionCpiWriteAccounts<'_, T> {
    pub fn bump(&self) -> u8 {
        self.cpi_signer.bump
    }

    pub fn invoking_program(&self) -> [u8; 32] {
        self.cpi_signer.program_id
    }

    pub fn to_account_infos(&self) -> Vec<T> {
        // The order must match mint_action on-chain program expectations:
        // [light_system_program, mint_signer, authority, fee_payer, cpi_authority_pda, cpi_context, ...recipient_token_accounts]
        let mut accounts = Vec::new();

        accounts.push(self.light_system_program.clone());

        if let Some(mint_signer) = &self.mint_signer {
            accounts.push((*mint_signer).clone());
        }

        accounts.push(self.authority.clone());
        accounts.push(self.fee_payer.clone());
        accounts.push(self.cpi_authority_pda.clone());
        accounts.push(self.cpi_context.clone());

        // Add recipient token accounts as remaining accounts
        for token_account in &self.recipient_token_accounts {
            accounts.push((*token_account).clone());
        }

        accounts
    }

    pub fn to_account_info_refs(&self) -> Vec<&T> {
        let mut refs = vec![self.fee_payer, self.cpi_context];
        if let Some(mint_signer) = &self.mint_signer {
            refs.push(mint_signer);
        }
        refs
    }
}
