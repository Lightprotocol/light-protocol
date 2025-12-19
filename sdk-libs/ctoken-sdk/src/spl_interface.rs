//! SPL interface PDA derivation utilities.

use light_ctoken_types::constants::{CPI_AUTHORITY_PDA, CREATE_TOKEN_POOL, POOL_SEED};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{token::CTOKEN_PROGRAM_ID, AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct SplInterfacePda {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}

/// Derive the spl interface pda for a given mint
pub fn get_spl_interface_pda(mint: &Pubkey) -> Pubkey {
    get_spl_interface_pda_with_index(mint, 0)
}

/// Find the spl interface pda for a given mint and index
pub fn find_spl_interface_pda_with_index(mint: &Pubkey, spl_interface_index: u8) -> (Pubkey, u8) {
    let seeds = &[POOL_SEED, mint.as_ref(), &[spl_interface_index]];
    let seeds = if spl_interface_index == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    Pubkey::find_program_address(seeds, &CTOKEN_PROGRAM_ID)
}

/// Get the spl interface pda for a given mint and index
pub fn get_spl_interface_pda_with_index(mint: &Pubkey, spl_interface_index: u8) -> Pubkey {
    find_spl_interface_pda_with_index(mint, spl_interface_index).0
}

/// Derive spl interface pda information for a given mint
pub fn derive_spl_interface_pda(mint: &solana_pubkey::Pubkey, index: u8) -> SplInterfacePda {
    let (pubkey, bump) = find_spl_interface_pda_with_index(mint, index);
    SplInterfacePda {
        pubkey,
        bump,
        index,
    }
}

/// # Create SPL interface PDA (token pool) instruction builder
///
/// Creates the spl interface pda for an SPL mint with index 0.
/// Spl interface pdas store spl tokens that are wrapped in light token or compressed token accounts.
///
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::spl_interface::CreateSplInterfacePda;
/// # use light_token_sdk::constants::SPL_TOKEN_PROGRAM_ID;
/// # let fee_payer = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let token_program = SPL_TOKEN_PROGRAM_ID;
/// let instruction = CreateSplInterfacePda::new(fee_payer, mint, token_program)
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
    pub fn new(fee_payer: Pubkey, mint: Pubkey, token_program: Pubkey) -> Self {
        let (spl_interface_pda, _) = find_spl_interface_pda_with_index(&mint, 0);
        Self {
            fee_payer,
            mint,
            token_program,
            spl_interface_pda,
        }
    }

    pub fn instruction(self) -> Instruction {
        Instruction {
            program_id: CTOKEN_PROGRAM_ID,
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
