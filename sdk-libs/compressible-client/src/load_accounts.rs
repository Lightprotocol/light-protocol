//! Load (decompress) accounts API.
use light_client::indexer::{Indexer, IndexerError, ValidityProofWithContext};
use light_compressed_account::{
    compressed_account::PackedMerkleContext, instruction_data::compressed_proof::ValidityProof,
};
use light_sdk::{compressible::Pack, instruction::PackedAccounts};
use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        mint_action::{MintInstructionData, MintWithContext},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{ExtensionStruct, TokenDataVersion},
};
use light_token_sdk::{
    compat::AccountState,
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    token::{
        derive_token_ata, CreateAssociatedTokenAccount, DecompressMint, LIGHT_TOKEN_PROGRAM_ID,
    },
};
use smallvec::SmallVec;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{
    account_interface::{TokenAccountInterface, TokenLoadContext},
    compressible_instruction::{self, DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR},
    decompress_mint::{
        DecompressMintError, MintInterface, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
    },
    RentFreeDecompressAccount,
};

/// Error type for load accounts operations.
#[derive(Debug, Error)]
pub enum LoadAccountsError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Build instruction failed: {0}")]
    BuildInstruction(String),

    #[error("Token SDK error: {0}")]
    TokenSdk(#[from] light_token_sdk::error::TokenSdkError),

    #[error("Mint error: {0}")]
    Mint(#[from] DecompressMintError),

    #[error("Cold PDA at index {index} (pubkey {pubkey}) is missing decompression_context")]
    MissingPdaDecompressionContext { index: usize, pubkey: Pubkey },

    #[error("Cold ATA at index {index} (pubkey {pubkey}) is missing load_context")]
    MissingAtaLoadContext { index: usize, pubkey: Pubkey },

    #[error("Cold mint at index {index} (cmint {cmint}) is missing compressed hash")]
    MissingMintHash { index: usize, cmint: Pubkey },
}

/// Build load instructions for cold accounts.
/// Exists fast if all accounts are hot.
/// Else, fetches proofs, returns instructions.
#[allow(clippy::too_many_arguments)]
pub async fn create_load_accounts_instructions<V, I>(
    program_owned_accounts: &[RentFreeDecompressAccount<V>],
    associated_token_accounts: &[TokenAccountInterface],
    mint_accounts: &[MintInterface],
    program_id: Pubkey,
    fee_payer: Pubkey,
    compression_config: Pubkey,
    rent_sponsor: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, LoadAccountsError>
where
    V: Pack + Clone + std::fmt::Debug,
    I: Indexer,
{
    // Fast exit if all hot.
    let cold_pdas: SmallVec<[&RentFreeDecompressAccount<V>; 8]> = program_owned_accounts
        .iter()
        .filter(|a| a.account_interface.is_cold)
        .collect();
    let cold_atas: SmallVec<[&TokenAccountInterface; 8]> = associated_token_accounts
        .iter()
        .filter(|a| a.is_cold)
        .collect();
    let cold_mints: SmallVec<[&MintInterface; 8]> =
        mint_accounts.iter().filter(|m| m.is_cold()).collect();

    if cold_pdas.is_empty() && cold_atas.is_empty() && cold_mints.is_empty() {
        return Ok(vec![]);
    }

    // get hashes - fail fast if any cold account is missing required context
    let pda_hashes: Vec<[u8; 32]> = cold_pdas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            a.account_interface
                .decompression_context
                .as_ref()
                .map(|c| c.compressed_account.hash)
                .ok_or(LoadAccountsError::MissingPdaDecompressionContext {
                    index: i,
                    pubkey: a.account_interface.pubkey,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let ata_hashes: Vec<[u8; 32]> = cold_atas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            a.hash().ok_or(LoadAccountsError::MissingAtaLoadContext {
                index: i,
                pubkey: a.pubkey,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mint_hashes: Vec<[u8; 32]> = cold_mints
        .iter()
        .enumerate()
        .map(|(i, m)| {
            m.hash().ok_or(LoadAccountsError::MissingMintHash {
                index: i,
                cmint: m.cmint,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Fetch proofs concurrently.
    // TODO: single batched proof RPC endpoint.
    let (pda_proof, ata_proof, mint_proofs) = futures::join!(
        fetch_proof_if_needed(&pda_hashes, indexer),
        fetch_proof_if_needed(&ata_hashes, indexer),
        fetch_mint_proofs(&mint_hashes, indexer),
    );

    // cap
    let cap = (!cold_pdas.is_empty()) as usize
        + if !cold_atas.is_empty() {
            cold_atas.len() + 1
        } else {
            0
        }
        + cold_mints.len();
    let mut out = Vec::with_capacity(cap);

    // Build PDA + Token instructions
    if !cold_pdas.is_empty() {
        let proof = pda_proof?
            .ok_or_else(|| LoadAccountsError::BuildInstruction("PDA proof fetch failed".into()))?;
        let ix = create_decompress_idempotent_instructions(
            &cold_pdas,
            proof,
            program_id,
            fee_payer,
            compression_config,
            rent_sponsor,
        )?;
        out.push(ix);
    }

    // Build associated token account instructions
    if !cold_atas.is_empty() {
        let proof = ata_proof?
            .ok_or_else(|| LoadAccountsError::BuildInstruction("ATA proof fetch failed".into()))?;
        let ixs = create_decompress_ata_instructions(&cold_atas, proof, fee_payer)?;
        out.extend(ixs);
    }

    // Build Mint instructions. One mint allowed per ixn.
    let mint_proofs = mint_proofs?;
    for (mint, proof) in cold_mints.iter().zip(mint_proofs.into_iter()) {
        let ix = create_decompress_mint_instructions(mint, proof, fee_payer, None, None)?;
        out.push(ix);
    }

    Ok(out)
}

async fn fetch_proof_if_needed<I: Indexer>(
    hashes: &[[u8; 32]],
    indexer: &I,
) -> Result<Option<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(None);
    }
    let result = indexer
        .get_validity_proof(hashes.to_vec(), vec![], None)
        .await?;
    Ok(Some(result.value))
}

async fn fetch_mint_proofs<I: Indexer>(
    hashes: &[[u8; 32]],
    indexer: &I,
) -> Result<Vec<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    // Each mint needs its own proof
    let mut proofs = Vec::with_capacity(hashes.len());
    for hash in hashes {
        let result = indexer
            .get_validity_proof(vec![*hash], vec![], None)
            .await?;
        proofs.push(result.value);
    }
    Ok(proofs)
}

/// Build decompress instruction for PDA + Token accounts.
/// Assumes all inputs are cold (caller filtered).
pub fn create_decompress_idempotent_instructions<V>(
    accounts: &[&RentFreeDecompressAccount<V>],
    proof: ValidityProofWithContext,
    program_id: Pubkey,
    fee_payer: Pubkey,
    compression_config: Pubkey,
    rent_sponsor: Pubkey,
) -> Result<Instruction, LoadAccountsError>
where
    V: Pack + Clone + std::fmt::Debug,
{
    // Check for tokens by program id
    let has_tokens = accounts.iter().any(|a| {
        a.account_interface
            .decompression_context
            .as_ref()
            .map(|c| c.compressed_account.owner == LIGHT_TOKEN_PROGRAM_ID)
            .unwrap_or(false)
    });

    let metas = if has_tokens {
        compressible_instruction::decompress::accounts(fee_payer, compression_config, rent_sponsor)
    } else {
        compressible_instruction::decompress::accounts_pda_only(
            fee_payer,
            compression_config,
            rent_sponsor,
        )
    };

    // Extract pubkeys and (CompressedAccount, variant) pairs
    let decompressed_account_addresses: Vec<Pubkey> = accounts
        .iter()
        .map(|a| a.account_interface.pubkey)
        .collect();

    let compressed_accounts: Vec<_> = accounts
        .iter()
        .map(|a| {
            let compressed_account = a
                .account_interface
                .decompression_context
                .as_ref()
                .expect("Cold account must have decompression context")
                .compressed_account
                .clone();
            (compressed_account, a.variant.clone())
        })
        .collect();

    compressible_instruction::build_decompress_idempotent_raw(
        &program_id,
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &decompressed_account_addresses,
        &compressed_accounts,
        &metas,
        proof,
    )
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}

/// Build decompress instructions for ATA accounts.
/// Returns N create_ata + 1 decompress instruction.
/// Assumes all inputs are cold (caller filtered).
pub fn create_decompress_ata_instructions(
    accounts: &[&TokenAccountInterface],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Vec<Instruction>, LoadAccountsError> {
    let contexts: SmallVec<[&TokenLoadContext; 8]> = accounts
        .iter()
        .filter_map(|a| a.load_context.as_ref())
        .collect();

    let mut out = Vec::with_capacity(contexts.len() + 1);

    // Build create_ata instructions (idempotent)
    for ctx in &contexts {
        let ix = CreateAssociatedTokenAccount::new(fee_payer, ctx.wallet_owner, ctx.mint)
            .idempotent()
            .instruction()
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        out.push(ix);
    }

    // Build single Transfer2 decompress instruction
    let decompress_ix = build_transfer2_decompress(&contexts, proof, fee_payer)?;
    out.push(decompress_ix);

    Ok(out)
}

/// Build Transfer2 decompress instruction from contexts.
fn build_transfer2_decompress(
    contexts: &[&TokenLoadContext],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Instruction, LoadAccountsError> {
    let mut packed_accounts = PackedAccounts::default();

    // Pack tree infos
    let packed_tree_infos = proof.pack_tree_infos(&mut packed_accounts);
    let tree_infos = packed_tree_infos
        .state_trees
        .as_ref()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("No state trees in proof".into()))?;

    let mut token_accounts = Vec::with_capacity(contexts.len());
    let mut in_tlv_data: Vec<Vec<ExtensionInstructionData>> = Vec::with_capacity(contexts.len());
    let mut has_any_tlv = false;

    for (i, ctx) in contexts.iter().enumerate() {
        let token = &ctx.compressed.token;
        let tree_info = &tree_infos.packed_tree_infos[i];

        // Pack accounts
        let owner_index = packed_accounts.insert_or_get_config(ctx.wallet_owner, true, false);
        let ata_index =
            packed_accounts.insert_or_get(derive_token_ata(&ctx.wallet_owner, &ctx.mint).0);
        let mint_index = packed_accounts.insert_or_get(token.mint);
        let delegate_index = token
            .delegate
            .map(|d| packed_accounts.insert_or_get(d))
            .unwrap_or(0);

        let source = MultiInputTokenDataWithContext {
            owner: ata_index,
            amount: token.amount,
            has_delegate: token.delegate.is_some(),
            delegate: delegate_index,
            mint: mint_index,
            version: TokenDataVersion::ShaFlat as u8,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: tree_info.queue_pubkey_index,
                prove_by_index: tree_info.prove_by_index,
                leaf_index: tree_info.leaf_index,
            },
            root_index: tree_info.root_index,
        };

        let mut ctoken = CTokenAccount2::new(vec![source])
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        ctoken
            .decompress(token.amount, ata_index)
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        token_accounts.push(ctoken);

        // Build TLV (CompressedOnly extension)
        let is_frozen = token.state == AccountState::Frozen;
        let tlv: Vec<ExtensionInstructionData> = token
            .tlv
            .as_ref()
            .map(|exts| {
                exts.iter()
                    .filter_map(|ext| match ext {
                        ExtensionStruct::CompressedOnly(co) => {
                            Some(ExtensionInstructionData::CompressedOnly(
                                CompressedOnlyExtensionInstructionData {
                                    delegated_amount: co.delegated_amount,
                                    withheld_transfer_fee: co.withheld_transfer_fee,
                                    is_frozen,
                                    compression_index: 0,
                                    is_ata: true,
                                    bump: ctx.bump,
                                    owner_index,
                                },
                            ))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !tlv.is_empty() {
            has_any_tlv = true;
        }
        in_tlv_data.push(tlv);
    }

    let (packed_metas, _, _) = packed_accounts.to_account_metas();

    create_transfer2_instruction(Transfer2Inputs {
        meta_config: Transfer2AccountsMetaConfig::new(fee_payer, packed_metas),
        token_accounts,
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        validity_proof: proof.proof,
        in_tlv: if has_any_tlv { Some(in_tlv_data) } else { None },
        ..Default::default()
    })
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}

/// Build decompress instruction for a single mint.
pub fn create_decompress_mint_instructions(
    mint: &MintInterface,
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
    rent_payment: Option<u8>,
    write_top_up: Option<u32>,
) -> Result<Instruction, LoadAccountsError> {
    // assume mint is cold
    let (_, mint_data) = mint
        .compressed()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("Expected cold mint".into()))?;

    // get tree info
    let account_info = &proof.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(input_queue);

    // ixdata
    let mint_instruction_data = MintInstructionData::try_from(mint_data.clone())
        .map_err(|_| LoadAccountsError::BuildInstruction("Invalid mint data".into()))?;

    DecompressMint {
        payer: fee_payer,
        authority: fee_payer,
        state_tree,
        input_queue,
        output_queue,
        compressed_mint_with_context: MintWithContext {
            leaf_index: account_info.leaf_index as u32,
            prove_by_index: account_info.root_index.proof_by_index(),
            root_index: account_info.root_index.root_index().unwrap_or_default(),
            address: mint.compressed_address,
            mint: Some(mint_instruction_data),
        },
        proof: ValidityProof(proof.proof.into()),
        rent_payment: rent_payment.unwrap_or(DEFAULT_RENT_PAYMENT),
        write_top_up: write_top_up.unwrap_or(DEFAULT_WRITE_TOP_UP),
    }
    .instruction()
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}
