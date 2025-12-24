//! SPL interface PDA derivation utilities.
//!
//! Re-exports from `light_ctoken_interface` with convenience wrappers.

use light_ctoken_interface::CTOKEN_PROGRAM_ID;
// Re-export derivation functions from ctoken-interface
pub use light_ctoken_interface::{
    find_spl_interface_pda, find_spl_interface_pda_with_index, get_spl_interface_pda,
    has_restricted_extensions, is_valid_spl_interface_pda, NUM_MAX_POOL_ACCOUNTS,
};
use light_ctoken_types::constants::{CPI_AUTHORITY_PDA, CREATE_TOKEN_POOL};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct SplInterfacePda {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}

/// Derive spl interface pda information for a given mint
pub fn derive_spl_interface_pda(mint: &Pubkey, index: u8, restricted: bool) -> SplInterfacePda {
    let (pubkey, bump) = find_spl_interface_pda_with_index(mint, index, restricted);
    SplInterfacePda {
        pubkey,
        bump,
        index,
    }
}

/// # Create SPL interface PDA (token pool) instruction builder
///
/// Creates the spl interface pda for an SPL mint with index 0.
/// Spl interface pdas store spl tokens that are wrapped in ctoken or compressed token accounts.
///
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::spl_interface::CreateSplInterfacePda;
/// # use light_ctoken_sdk::constants::SPL_TOKEN_PROGRAM_ID;
/// # let fee_payer = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let token_program = SPL_TOKEN_PROGRAM_ID;
/// let instruction = CreateSplInterfacePda::new(fee_payer, mint, token_program, false)
///     .instruction();
/// ```
pub struct CreateSplInterfacePda {
    pub fee_payer: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub spl_interface_pda: Pubkey,
}

impl CreateSplInterfacePda {
    /// Derives the spl interface pda for an SPL mint with index 0.
    pub fn new(fee_payer: Pubkey, mint: Pubkey, token_program: Pubkey, restricted: bool) -> Self {
        let (spl_interface_pda, _) = find_spl_interface_pda(&mint, restricted);
        Self {
            fee_payer,
            mint,
            token_program,
            spl_interface_pda,
        }
    }

    pub fn instruction(self) -> Instruction {
        Instruction {
            program_id: Pubkey::from(CTOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.fee_payer, true),
                AccountMeta::new(self.spl_interface_pda, false),
                AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                AccountMeta::new(self.mint, false),
                AccountMeta::new_readonly(self.token_program, false),
                AccountMeta::new_readonly(Pubkey::from(CPI_AUTHORITY_PDA), false),
            ],
            data: CREATE_TOKEN_POOL.to_vec(),
        }
    }
}
