// Example showing how to implement the Pack trait for custom types

use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use light_compressible_client::{Pack, PackedAccounts};
use solana_pubkey::Pubkey;

// Original data structure with Pubkeys
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserRecord {
    pub owner: Pubkey,    // 32 bytes
    pub delegate: Pubkey, // 32 bytes
    pub name: String,     // Variable
    pub score: u64,       // 8 bytes
}

// Packed version with u8 indices instead of Pubkeys
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedUserRecord {
    pub owner: u8,    // 1 byte (index into remaining_accounts)
    pub delegate: u8, // 1 byte (index into remaining_accounts)
    pub name: String, // Stays as-is
    pub score: u64,   // Stays as-is
}

// Implement Pack trait for UserRecord
impl Pack for UserRecord {
    type Packed = PackedUserRecord;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        PackedUserRecord {
            owner: remaining_accounts.insert_or_get(self.owner),
            delegate: remaining_accounts.insert_or_get(self.delegate),
            name: self.name.clone(),
            score: self.score,
        }
    }
}

// Example with variant wrapper (for token accounts)
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum AccountVariant {
    Standard = 0,
    Premium = 1,
}

// Wrapper that combines variant with data
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AccountWithVariant {
    pub variant: AccountVariant,
    pub data: UserRecord,
}

// Packed version
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedAccountWithVariant {
    pub variant: AccountVariant, // Variant stays unchanged
    pub data: PackedUserRecord,  // Data gets packed
}

// Pack implementation for the wrapper
impl Pack for AccountWithVariant {
    type Packed = PackedAccountWithVariant;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        PackedAccountWithVariant {
            variant: self.variant,                    // Variant is copied as-is
            data: self.data.pack(remaining_accounts), // Data is packed
        }
    }
}

// For simple types without Pubkeys, you can use identity packing
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SimpleData {
    pub counter: u64,
    pub active: bool,
}

// Identity pack - returns self
impl Pack for SimpleData {
    type Packed = Self;

    fn pack(&self, _remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

fn main() {
    // Example usage
    let mut remaining_accounts = PackedAccounts::default();

    let user_record = UserRecord {
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        name: "Alice".to_string(),
        score: 100,
    };

    // Pack the user record
    let packed = user_record.pack(&mut remaining_accounts);
    println!("Packed: {:?}", packed);

    // The remaining_accounts now contains the Pubkeys
    let (account_metas, _, _) = remaining_accounts.to_account_metas();
    println!("Account metas: {} accounts", account_metas.len());
}
