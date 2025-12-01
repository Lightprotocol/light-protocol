//! Load ATA - unifies wrap SPL/T22 + decompress into single flow

use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::{
    ctoken::TransferSplToCtoken, token_pool::find_token_pool_pda_with_index, SPL_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};

const SPL_ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

fn get_spl_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

/// Returns `Vec<Instruction>` (empty if nothing to load)
pub async fn load_ata_instructions<R: Rpc + Indexer>(
    rpc: &mut R,
    payer: Pubkey,
    ctoken_ata: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    let mut instructions = Vec::new();

    // 1. Check SPL ATA balance
    let spl_token_program = Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID);
    let spl_ata = get_spl_ata(&owner, &mint, &spl_token_program);

    if let Some(spl_account_info) = rpc.get_account(spl_ata).await? {
        if let Ok(pod_account) = pod_from_bytes::<PodAccount>(&spl_account_info.data) {
            let balance: u64 = pod_account.amount.into();
            if balance > 0 {
                let (token_pool_pda, token_pool_pda_bump) =
                    find_token_pool_pda_with_index(&mint, 0);
                let wrap_ix = TransferSplToCtoken {
                    amount: balance,
                    token_pool_pda_bump,
                    source_spl_token_account: spl_ata,
                    destination_ctoken_account: ctoken_ata,
                    authority: owner,
                    mint,
                    payer,
                    token_pool_pda,
                    spl_token_program: Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID),
                }
                .instruction()
                .map_err(|e| RpcError::CustomError(e.to_string()))?;
                instructions.push(wrap_ix);
            }
        }
    }

    // 2. Check compressed token accounts
    let options = GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(Some(mint));
    let compressed_response = rpc
        .get_compressed_token_accounts_by_owner(&owner, Some(options), None)
        .await
        .map_err(|e| RpcError::CustomError(e.to_string()))?;

    let compressed_accounts = compressed_response.value.items;
    if !compressed_accounts.is_empty() {
        let compressed_balance: u64 = compressed_accounts.iter().map(|acc| acc.token.amount).sum();
        if compressed_balance > 0 {
            let decompress_ix = create_generic_transfer2_instruction(
                rpc,
                vec![Transfer2InstructionType::Decompress(DecompressInput {
                    compressed_token_account: compressed_accounts.clone(),
                    decompress_amount: compressed_balance,
                    solana_token_account: ctoken_ata,
                    amount: compressed_balance,
                    pool_index: None,
                })],
                payer,
                false,
            )
            .await
            .map_err(|e| RpcError::CustomError(e.to_string()))?;
            instructions.push(decompress_ix);
        }
    }

    Ok(instructions)
}

/// Returns `Option<Signature>` (None if nothing to load)
pub async fn load_ata<R: Rpc + Indexer>(
    rpc: &mut R,
    payer: &Keypair,
    ctoken_ata: Pubkey,
    owner: &Keypair,
    mint: Pubkey,
) -> Result<Option<Signature>, RpcError> {
    let instructions =
        load_ata_instructions(rpc, payer.pubkey(), ctoken_ata, owner.pubkey(), mint).await?;

    if instructions.is_empty() {
        return Ok(None);
    }

    let mut signers = vec![payer];
    if owner.pubkey() != payer.pubkey() {
        signers.push(owner);
    }

    let signature = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
        .await?;

    Ok(Some(signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    #[test]
    fn test_get_spl_ata_deterministic() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(2);
        let program = make_pubkey(3);

        let ata1 = get_spl_ata(&owner, &mint, &program);
        let ata2 = get_spl_ata(&owner, &mint, &program);
        assert_eq!(ata1, ata2);
    }

    #[test]
    fn test_get_spl_ata_different_owners() {
        let owner1 = make_pubkey(1);
        let owner2 = make_pubkey(2);
        let mint = make_pubkey(10);
        let program = make_pubkey(20);

        let ata1 = get_spl_ata(&owner1, &mint, &program);
        let ata2 = get_spl_ata(&owner2, &mint, &program);
        assert_ne!(ata1, ata2);
    }

    #[test]
    fn test_get_spl_ata_different_mints() {
        let owner = make_pubkey(1);
        let mint1 = make_pubkey(10);
        let mint2 = make_pubkey(11);
        let program = make_pubkey(20);

        let ata1 = get_spl_ata(&owner, &mint1, &program);
        let ata2 = get_spl_ata(&owner, &mint2, &program);
        assert_ne!(ata1, ata2);
    }

    #[test]
    fn test_get_spl_ata_different_programs() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(10);
        let program1 = make_pubkey(20);
        let program2 = make_pubkey(21);

        let ata1 = get_spl_ata(&owner, &mint, &program1);
        let ata2 = get_spl_ata(&owner, &mint, &program2);
        assert_ne!(ata1, ata2);
    }

    #[test]
    fn test_get_spl_ata_uses_associated_token_program() {
        let owner = make_pubkey(1);
        let mint = make_pubkey(2);
        let program = make_pubkey(3);

        // The ATA should be derived using SPL_ASSOCIATED_TOKEN_PROGRAM_ID
        let ata = get_spl_ata(&owner, &mint, &program);

        // Verify it's a valid program-derived address
        let expected = Pubkey::find_program_address(
            &[owner.as_ref(), program.as_ref(), mint.as_ref()],
            &SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
        )
        .0;
        assert_eq!(ata, expected);
    }
}
