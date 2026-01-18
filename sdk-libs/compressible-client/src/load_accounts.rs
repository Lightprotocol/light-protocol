//! Load (decompress) accounts API.
use light_client::indexer::{
    CompressedTokenAccount, Indexer, IndexerError, ValidityProofWithContext,
};
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
    compressible_instruction::{self, DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR},
    decompress_mint::{DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP},
    AccountInterface, TokenAccountInterface,
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

    #[error("Cold PDA at index {index} (pubkey {pubkey}) is missing compressed data")]
    MissingPdaCompressed { index: usize, pubkey: Pubkey },

    #[error("Cold ATA at index {index} (pubkey {pubkey}) is missing compressed data")]
    MissingAtaCompressed { index: usize, pubkey: Pubkey },

    #[error("Cold mint at index {index} (mint {mint}) is missing compressed hash")]
    MissingMintHash { index: usize, mint: Pubkey },
}

/// Fetch proof per hash
async fn fetch_individual_proofs<I: Indexer>(
    hashes: &[[u8; 32]],
    indexer: &I,
) -> Result<Vec<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    let mut proofs = Vec::with_capacity(hashes.len());
    for hash in hashes {
        let result = indexer
            .get_validity_proof(vec![*hash], vec![], None)
            .await?;
        proofs.push(result.value);
    }
    Ok(proofs)
}

/// Fetch batched proofs for multiple hashes
async fn fetch_batched_proofs<I: Indexer>(
    hashes: &[[u8; 32]],
    batch_size: usize,
    indexer: &I,
) -> Result<Vec<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    let mut proofs = Vec::with_capacity(hashes.len().div_ceil(batch_size));
    for chunk in hashes.chunks(batch_size) {
        let result = indexer
            .get_validity_proof(chunk.to_vec(), vec![], None)
            .await?;
        proofs.push(result.value);
    }
    Ok(proofs)
}

/// Context for building ATA decompress instructions.
/// Extracts necessary data from TokenAccountInterface.
struct AtaDecompressContext<'a> {
    compressed: &'a CompressedTokenAccount,
    wallet_owner: Pubkey,
    mint: Pubkey,
    bump: u8,
}

impl<'a> AtaDecompressContext<'a> {
    fn from_interface(iface: &'a TokenAccountInterface) -> Option<Self> {
        let compressed = iface.compressed()?;
        let wallet_owner = iface.owner(); // After fix: parsed.owner = wallet
        let mint = iface.mint();
        let bump = iface.ata_bump()?; // Re-derives from wallet + mint
        Some(Self {
            compressed,
            wallet_owner,
            mint,
            bump,
        })
    }
}

/// Build decompress instructions for ATA accounts.
/// Returns N create_ata + 1 decompress instruction.
/// Assumes all inputs are cold (caller filtered).
pub fn create_decompress_ata_instructions(
    accounts: &[&TokenAccountInterface],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Vec<Instruction>, LoadAccountsError> {
    let contexts: SmallVec<[AtaDecompressContext; 8]> = accounts
        .iter()
        .filter_map(|a| AtaDecompressContext::from_interface(a))
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
    contexts: &[AtaDecompressContext],
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

// =============================================================================
// ACCOUNTSPEC-BASED FUNCTIONS (UNIFIED API)
// =============================================================================

use crate::compressible_program::{AccountSpec, PdaSpec};

/// Maximum ATAs per decompress instruction.
const MAX_ATAS_PER_INSTRUCTION: usize = 8;

/// Build load instructions from a slice of AccountSpec.
///
/// Primary entry point. Returns empty vec if all accounts are hot.
#[allow(clippy::too_many_arguments)]
pub async fn create_load_instructions<V, I>(
    specs: &[AccountSpec<V>],
    fee_payer: Pubkey,
    compression_config: Pubkey,
    rent_sponsor: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, LoadAccountsError>
where
    V: Pack + Clone + std::fmt::Debug,
    I: Indexer,
{
    // FAST PATH: Check if any cold - O(n) scan
    if !crate::compressible_program::any_cold(specs) {
        return Ok(vec![]);
    }

    // Filter cold specs by type inline
    let cold_pdas: Vec<_> = specs
        .iter()
        .filter_map(|s| match s {
            AccountSpec::Pda(p) if p.is_cold() => Some(p),
            _ => None,
        })
        .collect();

    let cold_atas: Vec<_> = specs
        .iter()
        .filter_map(|s| match s {
            AccountSpec::Ata(a) if a.is_cold() => Some(a),
            _ => None,
        })
        .collect();

    let cold_mints: Vec<_> = specs
        .iter()
        .filter_map(|s| match s {
            AccountSpec::Mint(m) if m.is_cold() => Some(m),
            _ => None,
        })
        .collect();

    // Collect hashes for proof fetching
    let pda_hashes: Vec<[u8; 32]> = cold_pdas
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingPdaCompressed {
                index: i,
                pubkey: s.address(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let ata_hashes: Vec<[u8; 32]> = cold_atas
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingAtaCompressed {
                index: i,
                pubkey: s.key,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mint_hashes: Vec<[u8; 32]> = cold_mints
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingMintHash {
                index: i,
                mint: s.key,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Fetch proofs concurrently
    let (pda_proofs, ata_proofs, mint_proofs) = futures::join!(
        fetch_individual_proofs(&pda_hashes, indexer),
        fetch_batched_proofs(&ata_hashes, MAX_ATAS_PER_INSTRUCTION, indexer),
        fetch_individual_proofs(&mint_hashes, indexer),
    );

    let pda_proofs = pda_proofs?;
    let ata_proofs = ata_proofs?;
    let mint_proofs = mint_proofs?;

    let mut out = Vec::new();

    // Build PDA decompression instructions. For now, 1 per PDA.
    // TODO: Enable multi
    for (pda_spec, proof) in cold_pdas.iter().zip(pda_proofs.into_iter()) {
        let ix = create_decompress_from_pda_specs(
            &[*pda_spec],
            proof,
            fee_payer,
            compression_config,
            rent_sponsor,
        )?;
        out.push(ix);
    }

    // Build ATA decompression instructions
    let ata_chunks: Vec<_> = cold_atas.chunks(MAX_ATAS_PER_INSTRUCTION).collect();
    for (chunk, proof) in ata_chunks.into_iter().zip(ata_proofs.into_iter()) {
        let ixs = create_decompress_from_ata_interfaces(chunk, proof, fee_payer)?;
        out.extend(ixs);
    }

    // Build mint decompression instructions. For now, 1 per mint.
    for (mint_interface, proof) in cold_mints.iter().zip(mint_proofs.into_iter()) {
        let ix = create_decompress_from_mint_interface(mint_interface, proof, fee_payer)?;
        out.push(ix);
    }

    Ok(out)
}

/// Build decompress instruction from PdaSpecs.
fn create_decompress_from_pda_specs<V>(
    specs: &[&PdaSpec<V>],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
    compression_config: Pubkey,
    rent_sponsor: Pubkey,
) -> Result<Instruction, LoadAccountsError>
where
    V: Pack + Clone + std::fmt::Debug,
{
    use light_client::indexer::CompressedAccount;

    // Check for tokens by program id in compressed account
    let has_tokens = specs.iter().any(|s| {
        s.compressed()
            .map(|c| c.owner == LIGHT_TOKEN_PROGRAM_ID)
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
    let decompressed_account_addresses: Vec<Pubkey> = specs.iter().map(|s| s.address()).collect();

    let compressed_accounts: Vec<(CompressedAccount, V)> = specs
        .iter()
        .map(|s| {
            let compressed_account = s
                .compressed()
                .expect("Cold spec must have compressed data")
                .clone();
            (compressed_account, s.variant.clone())
        })
        .collect();

    // Use program_id from first spec (all should be same program)
    let program_id = specs.first().map(|s| s.program_id()).unwrap_or_default();

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

/// Build decompress instructions from TokenAccountInterface (ATAs).
fn create_decompress_from_ata_interfaces(
    interfaces: &[&TokenAccountInterface],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Vec<Instruction>, LoadAccountsError> {
    create_decompress_ata_instructions(interfaces, proof, fee_payer)
}

/// Build decompress mint instruction from AccountInterface.
fn create_decompress_from_mint_interface(
    mint_interface: &AccountInterface,
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Instruction, LoadAccountsError> {
    let account_info = &proof.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(input_queue);

    // Parse mint data from interface
    let mint_data = mint_interface.as_mint().ok_or_else(|| {
        LoadAccountsError::BuildInstruction("Mint interface missing mint_data".into())
    })?;

    let compressed_address = mint_interface.mint_compressed_address().ok_or_else(|| {
        LoadAccountsError::BuildInstruction("Mint interface missing compressed_address".into())
    })?;

    let mint_instruction_data = MintInstructionData::try_from(mint_data)
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
            address: compressed_address,
            mint: Some(mint_instruction_data),
        },
        proof: ValidityProof(proof.proof.into()),
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    }
    .instruction()
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}
