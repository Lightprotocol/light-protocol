//! Wrap instruction: SPL/T22 token account → light-token account.
//!
//! Uses Transfer2 with two compressions (compress from SPL + decompress to light-token).
//! Uses `decompressed_accounts_only` layout (CPI authority first, no light-system-program).
//!
//! Ported from TypeScript `wrap.ts`.

use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::KoraLightError,
    program_ids::{
        CPI_AUTHORITY_PDA, LIGHT_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, TRANSFER2_DISCRIMINATOR,
    },
    types::{CompressedTokenInstructionDataTransfer2, Compression, SplInterfaceInfo},
};

/// Default max top-up
const DEFAULT_MAX_TOP_UP: u16 = u16::MAX;

/// Build a wrap instruction: SPL → light-token.
///
/// Takes tokens from an SPL token account and deposits them into a light-token account.
/// Uses the `decompressed_accounts_only` Transfer2 layout.
///
/// # Arguments
/// * `source` - SPL token account (writable)
/// * `destination` - Light-token account (writable)
/// * `owner` - Token owner (signer)
/// * `mint` - Token mint
/// * `amount` - Amount to wrap
/// * `decimals` - Token decimals
/// * `payer` - Fee payer (signer)
/// * `spl_interface` - SPL pool info for the compress operation
#[allow(clippy::too_many_arguments)]
pub fn create_wrap_instruction(
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
    // Order: mint, owner, source, destination, pool, token_program
    let mint_index: u8 = 0;
    let owner_index: u8 = 1;
    let source_index: u8 = 2;
    let destination_index: u8 = 3;
    let pool_index: u8 = 4;
    let _token_program_index: u8 = 5;

    // Two compressions:
    // 1. Compress from SPL (source → pool)
    let compress = Compression::compress_spl(
        amount,
        mint_index,
        source_index,
        owner_index,
        pool_index,
        spl_interface.pool_index,
        spl_interface.bump,
        decimals,
    );

    // 2. Decompress to light-token (pool → destination)
    let decompress = Compression::decompress(amount, mint_index, destination_index);

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

    // decompressed_accounts_only layout: CPI authority first, then fee payer
    let mut accounts = vec![
        AccountMeta::new_readonly(CPI_AUTHORITY_PDA, false),
        AccountMeta::new(*payer, true),
    ];

    // Packed accounts
    accounts.extend([
        AccountMeta::new_readonly(*mint, false), // 0: mint
        AccountMeta::new_readonly(*owner, true), // 1: owner (signer)
        AccountMeta::new(*source, false),        // 2: source SPL account
        AccountMeta::new(*destination, false),   // 3: destination light-token
        AccountMeta::new(spl_interface.spl_interface_pda, false), // 4: pool
        AccountMeta::new_readonly(spl_interface.token_program, false), // 5: token program
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false), // light token program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system program
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
    fn test_wrap_instruction_builds() {
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
            create_wrap_instruction(&source, &destination, &owner, &mint, 1000, 6, &payer, &spl)
                .unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.data[0], TRANSFER2_DISCRIMINATOR);
        // CPI authority + payer + 8 packed
        assert_eq!(ix.accounts.len(), 10);
        // First account is CPI authority (decompressed_accounts_only mode)
        assert_eq!(ix.accounts[0].pubkey, CPI_AUTHORITY_PDA);
    }
}
