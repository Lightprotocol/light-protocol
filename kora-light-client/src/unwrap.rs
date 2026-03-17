//! Unwrap instruction: light-token account → SPL/T22 token account.
//!
//! Uses Transfer2 with two compressions (compress from light-token + decompress to SPL).
//! Uses `decompressed_accounts_only` layout.
//!
//! Ported from TypeScript `unwrap.ts`.

use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::KoraLightError,
    program_ids::{
        CPI_AUTHORITY_PDA, DEFAULT_MAX_TOP_UP, LIGHT_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID,
        TRANSFER2_DISCRIMINATOR,
    },
    types::{CompressedTokenInstructionDataTransfer2, Compression, SplInterfaceInfo},
};

/// Unwrap light-token to SPL/T22 token account.
///
/// Builds a Transfer2 instruction with two compression operations:
/// compress from light-token source, then decompress to SPL destination.
/// Uses the `decompressed_accounts_only` layout. Reverse of [`Wrap`](crate::Wrap).
///
/// # Example
/// ```rust,ignore
/// use kora_light_client::Unwrap;
///
/// let ix = Unwrap {
///     source: light_token_ata,
///     destination: spl_ata,
///     owner,
///     mint,
///     amount: 1_000,
///     decimals: 6,
///     payer,
///     spl_interface: &spl_info,
/// }.instruction()?;
/// ```
#[derive(Debug, Clone)]
pub struct Unwrap<'a> {
    /// Source light-token account (writable).
    pub source: Pubkey,
    /// Destination SPL token account (writable).
    pub destination: Pubkey,
    /// Token owner (signer).
    pub owner: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// Amount to unwrap.
    pub amount: u64,
    /// Token decimals.
    pub decimals: u8,
    /// Fee payer (signer).
    pub payer: Pubkey,
    /// SPL pool info for the decompress operation.
    pub spl_interface: &'a SplInterfaceInfo,
}

impl<'a> Unwrap<'a> {
    /// Build the unwrap instruction.
    pub fn instruction(&self) -> Result<Instruction, KoraLightError> {
        create_unwrap_instruction(
            &self.source,
            &self.destination,
            &self.owner,
            &self.mint,
            self.amount,
            self.decimals,
            &self.payer,
            self.spl_interface,
        )
    }
}

/// Build an unwrap instruction: light-token → SPL.
#[allow(clippy::too_many_arguments)]
pub fn create_unwrap_instruction(
    source: &Pubkey,
    destination: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    payer: &Pubkey,
    spl_interface: &SplInterfaceInfo,
) -> Result<Instruction, KoraLightError> {
    // Packed accounts for decompressed_accounts_only mode
    let mint_index: u8 = 0;
    let owner_index: u8 = 1;
    let source_index: u8 = 2;
    let destination_index: u8 = 3;
    let pool_index: u8 = 4;
    let _token_program_index: u8 = 5;

    // Two compressions (reverse of wrap):
    // 1. Compress from light-token (source → pool equivalent)
    let compress = Compression::compress(amount, mint_index, source_index, owner_index);

    // 2. Decompress to SPL (pool → destination)
    let decompress = Compression::decompress_spl(
        amount,
        mint_index,
        destination_index,
        pool_index,
        spl_interface.pool_index,
        spl_interface.bump,
        decimals,
    );

    let transfer2_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: DEFAULT_MAX_TOP_UP,
        cpi_context: None,
        compressions: Some(vec![compress, decompress]),
        proof: None,
        in_token_data: Vec::new(),
        out_token_data: Vec::new(),
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
    };

    let mut data = Vec::new();
    data.push(TRANSFER2_DISCRIMINATOR);
    transfer2_data.serialize(&mut data)?;

    // decompressed_accounts_only layout
    let mut accounts = vec![
        AccountMeta::new_readonly(CPI_AUTHORITY_PDA, false),
        AccountMeta::new(*payer, true),
    ];

    accounts.extend([
        AccountMeta::new_readonly(*mint, false), // 0: mint
        AccountMeta::new_readonly(*owner, true), // 1: owner (signer)
        AccountMeta::new(*source, false),        // 2: source light-token
        AccountMeta::new(*destination, false),   // 3: destination SPL
        AccountMeta::new(spl_interface.spl_interface_pda, false), // 4: pool
        AccountMeta::new_readonly(spl_interface.token_program, false), // 5: token program
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ]);

    Ok(Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unwrap_instruction_builds() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let pool_pda = Pubkey::new_unique();
        let token_program = Pubkey::new_unique();

        let spl = SplInterfaceInfo {
            spl_interface_pda: pool_pda,
            bump: 255,
            pool_index: 0,
            token_program,
        };

        let ix =
            create_unwrap_instruction(&source, &destination, &owner, &mint, 1000, 6, &payer, &spl)
                .unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.data[0], TRANSFER2_DISCRIMINATOR);
        assert_eq!(ix.accounts.len(), 10);
        assert_eq!(ix.accounts[0].pubkey, CPI_AUTHORITY_PDA);
    }

    #[test]
    fn test_unwrap_is_reverse_of_wrap() {
        // Verify unwrap produces different compressions than wrap
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let payer = Pubkey::new_unique();

        let spl = SplInterfaceInfo {
            spl_interface_pda: Pubkey::new_unique(),
            bump: 255,
            pool_index: 0,
            token_program: Pubkey::new_unique(),
        };

        let wrap_ix = crate::wrap::create_wrap_instruction(
            &source,
            &destination,
            &owner,
            &mint,
            1000,
            6,
            &payer,
            &spl,
        )
        .unwrap();

        let unwrap_ix =
            create_unwrap_instruction(&source, &destination, &owner, &mint, 1000, 6, &payer, &spl)
                .unwrap();

        // Both should have same program and account count but different data
        assert_eq!(wrap_ix.program_id, unwrap_ix.program_id);
        assert_eq!(wrap_ix.accounts.len(), unwrap_ix.accounts.len());
        // Data should differ (different compression modes)
        assert_ne!(wrap_ix.data, unwrap_ix.data);
    }
}
