//! SPL interface PDA derivation utilities.
//!
//! Re-exports from `light_token_interface` with convenience wrappers.

use light_token_interface::{
    discriminator::{ADD_TOKEN_POOL, CREATE_TOKEN_POOL},
    CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID,
};
// Re-export derivation functions from token-interface
pub use light_token_interface::{
    find_spl_interface_pda, find_spl_interface_pda_with_index, get_spl_interface_pda,
    has_restricted_extensions, is_valid_spl_interface_pda, NUM_MAX_POOL_ACCOUNTS,
};
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
/// Creates or adds an spl interface pda for an SPL mint.
/// Spl interface pdas store spl tokens that are wrapped in ctoken or compressed token accounts.
///
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::spl_interface::CreateSplInterfacePda;
/// # use light_token_sdk::constants::SPL_TOKEN_PROGRAM_ID;
/// # let fee_payer = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let token_program = SPL_TOKEN_PROGRAM_ID;
/// // Create initial pool (index 0)
/// let instruction = CreateSplInterfacePda::new(fee_payer, mint, token_program, false)
///     .instruction();
/// // Add additional pool (index 1)
/// let instruction = CreateSplInterfacePda::new_with_index(fee_payer, mint, token_program, 1, false)
///     .instruction();
/// ```
pub struct CreateSplInterfacePda {
    pub fee_payer: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub spl_interface_pda: Pubkey,
    pub existing_spl_interface_pda: Option<Pubkey>,
    pub index: u8,
}

impl CreateSplInterfacePda {
    /// Derives the spl interface pda for an SPL mint with index 0.
    pub fn new(fee_payer: Pubkey, mint: Pubkey, token_program: Pubkey, restricted: bool) -> Self {
        Self::new_with_index(fee_payer, mint, token_program, 0, restricted)
    }

    /// Derives the spl interface pda for an SPL mint with a specific index.
    /// For index 0, creates the initial pool. For index > 0, adds an additional pool.
    pub fn new_with_index(
        fee_payer: Pubkey,
        mint: Pubkey,
        token_program: Pubkey,
        index: u8,
        restricted: bool,
    ) -> Self {
        let (spl_interface_pda, _) = find_spl_interface_pda_with_index(&mint, index, restricted);
        let existing_spl_interface_pda = if index > 0 {
            let (existing_pda, _) =
                find_spl_interface_pda_with_index(&mint, index.saturating_sub(1), restricted);
            Some(existing_pda)
        } else {
            None
        };
        Self {
            fee_payer,
            mint,
            token_program,
            spl_interface_pda,
            existing_spl_interface_pda,
            index,
        }
    }

    pub fn instruction(self) -> Instruction {
        let cpi_authority = Pubkey::from(CPI_AUTHORITY);

        if self.index == 0 {
            // CreateTokenPool instruction
            Instruction {
                program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
                accounts: vec![
                    AccountMeta::new(self.fee_payer, true),
                    AccountMeta::new(self.spl_interface_pda, false),
                    AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                    AccountMeta::new(self.mint, false),
                    AccountMeta::new_readonly(self.token_program, false),
                    AccountMeta::new_readonly(cpi_authority, false),
                ],
                data: CREATE_TOKEN_POOL.to_vec(),
            }
        } else {
            // AddTokenPool instruction
            let mut data = ADD_TOKEN_POOL.to_vec();
            data.push(self.index);
            Instruction {
                program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
                accounts: vec![
                    AccountMeta::new(self.fee_payer, true),
                    AccountMeta::new(self.spl_interface_pda, false),
                    AccountMeta::new_readonly(self.existing_spl_interface_pda.unwrap(), false),
                    AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                    AccountMeta::new(self.mint, false),
                    AccountMeta::new_readonly(self.token_program, false),
                    AccountMeta::new_readonly(cpi_authority, false),
                ],
                data,
            }
        }
    }
}
