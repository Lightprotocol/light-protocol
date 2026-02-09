//! Load cold accounts API.

use light_account::{derive_rent_sponsor_pda, Pack};
use light_compressed_account::{
    compressed_account::PackedMerkleContext, instruction_data::compressed_proof::ValidityProof,
};
use light_compressed_token_sdk::compressed_token::{
    transfer2::{
        create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config, Transfer2Inputs,
    },
    CTokenAccount2,
};
use light_sdk::instruction::PackedAccounts;
use light_token::{
    compat::AccountState,
    instruction::{
        derive_token_ata, CreateAssociatedTokenAccount, DecompressMint, LIGHT_TOKEN_PROGRAM_ID,
    },
};
use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        mint_action::{MintInstructionData, MintWithContext},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{ExtensionStruct, TokenDataVersion},
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use thiserror::Error;

use super::{
    decompress_mint::{DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP},
    instructions::{self, DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR},
    light_program_interface::{AccountSpec, PdaSpec},
    AccountInterface, TokenAccountInterface,
};
use crate::indexer::{
    CompressedAccount, CompressedTokenAccount, Indexer, IndexerError, ValidityProofWithContext,
};

#[derive(Debug, Error)]
pub enum LoadAccountsError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Build instruction failed: {0}")]
    BuildInstruction(String),

    #[error("Token SDK error: {0}")]
    TokenSdk(#[from] light_token::error::TokenSdkError),

    #[error("Cold PDA at index {index} (pubkey {pubkey}) missing data")]
    MissingPdaCompressed { index: usize, pubkey: Pubkey },

    #[error("Cold ATA at index {index} (pubkey {pubkey}) missing data")]
    MissingAtaCompressed { index: usize, pubkey: Pubkey },

    #[error("Cold mint at index {index} (mint {mint}) missing hash")]
    MissingMintHash { index: usize, mint: Pubkey },

    #[error("ATA at index {index} (pubkey {pubkey}) missing compressed data or ATA bump")]
    MissingAtaContext { index: usize, pubkey: Pubkey },

    #[error("Tree info index {index} out of bounds (len {len})")]
    TreeInfoIndexOutOfBounds { index: usize, len: usize },
}

const MAX_ATAS_PER_IX: usize = 8;

/// Build load instructions for cold accounts. Returns empty vec if all hot.
///
/// The rent sponsor PDA is derived internally from the program_id.
/// Seeds: ["rent_sponsor"]
///
/// TODO: reduce ixn count and txn size, reduce roundtrips.
#[allow(clippy::too_many_arguments)]
pub async fn create_load_instructions<V, I>(
    specs: &[AccountSpec<V>],
    fee_payer: Pubkey,
    compression_config: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, LoadAccountsError>
where
    V: Pack<solana_instruction::AccountMeta> + Clone + std::fmt::Debug,
    I: Indexer,
{
    if !super::light_program_interface::any_cold(specs) {
        return Ok(vec![]);
    }

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
            AccountSpec::Ata(a) if a.is_cold() => Some(a.as_ref()),
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

    let pda_hashes = collect_pda_hashes(&cold_pdas)?;
    let ata_hashes = collect_ata_hashes(&cold_atas)?;
    let mint_hashes = collect_mint_hashes(&cold_mints)?;

    let (pda_proofs, ata_proofs, mint_proofs) = futures::join!(
        fetch_proofs(&pda_hashes, indexer),
        fetch_proofs_batched(&ata_hashes, MAX_ATAS_PER_IX, indexer),
        fetch_proofs(&mint_hashes, indexer),
    );

    let pda_proofs = pda_proofs?;
    let ata_proofs = ata_proofs?;
    let mint_proofs = mint_proofs?;

    let mut out = Vec::new();

    // 1. DecompressAccountsIdempotent for all cold PDAs (including token PDAs).
    //    Token PDAs are created on-chain via CPI inside DecompressVariant.
    for (spec, proof) in cold_pdas.iter().zip(pda_proofs) {
        out.push(build_pda_load(
            &[spec],
            proof,
            fee_payer,
            compression_config,
        )?);
    }

    // 2. ATA loads (CreateAssociatedTokenAccount + Transfer2)
    let ata_chunks: Vec<_> = cold_atas.chunks(MAX_ATAS_PER_IX).collect();
    for (chunk, proof) in ata_chunks.into_iter().zip(ata_proofs) {
        out.extend(build_ata_load(chunk, proof, fee_payer)?);
    }

    // 3. Mint loads
    for (iface, proof) in cold_mints.iter().zip(mint_proofs) {
        out.push(build_mint_load(iface, proof, fee_payer)?);
    }
    Ok(out)
}

fn collect_pda_hashes<V>(specs: &[&PdaSpec<V>]) -> Result<Vec<[u8; 32]>, LoadAccountsError> {
    specs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingPdaCompressed {
                index: i,
                pubkey: s.address(),
            })
        })
        .collect()
}

fn collect_ata_hashes(
    ifaces: &[&TokenAccountInterface],
) -> Result<Vec<[u8; 32]>, LoadAccountsError> {
    ifaces
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingAtaCompressed {
                index: i,
                pubkey: s.key,
            })
        })
        .collect()
}

fn collect_mint_hashes(ifaces: &[&AccountInterface]) -> Result<Vec<[u8; 32]>, LoadAccountsError> {
    ifaces
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.hash().ok_or(LoadAccountsError::MissingMintHash {
                index: i,
                mint: s.key,
            })
        })
        .collect()
}

async fn fetch_proofs<I: Indexer>(
    hashes: &[[u8; 32]],
    indexer: &I,
) -> Result<Vec<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }
    let mut proofs = Vec::with_capacity(hashes.len());
    for hash in hashes {
        proofs.push(
            indexer
                .get_validity_proof(vec![*hash], vec![], None)
                .await?
                .value,
        );
    }
    Ok(proofs)
}

async fn fetch_proofs_batched<I: Indexer>(
    hashes: &[[u8; 32]],
    batch_size: usize,
    indexer: &I,
) -> Result<Vec<ValidityProofWithContext>, IndexerError> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }
    let mut proofs = Vec::with_capacity(hashes.len().div_ceil(batch_size));
    for chunk in hashes.chunks(batch_size) {
        proofs.push(
            indexer
                .get_validity_proof(chunk.to_vec(), vec![], None)
                .await?
                .value,
        );
    }
    Ok(proofs)
}

fn build_pda_load<V>(
    specs: &[&PdaSpec<V>],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
    compression_config: Pubkey,
) -> Result<Instruction, LoadAccountsError>
where
    V: Pack<solana_instruction::AccountMeta> + Clone + std::fmt::Debug,
{
    let has_tokens = specs.iter().any(|s| {
        s.compressed()
            .map(|c| c.owner == LIGHT_TOKEN_PROGRAM_ID)
            .unwrap_or(false)
    });

    // Derive rent sponsor PDA from program_id
    let program_id = specs.first().map(|s| s.program_id()).unwrap_or_default();
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let metas = if has_tokens {
        instructions::load::accounts(fee_payer, compression_config, rent_sponsor)
    } else {
        instructions::load::accounts_pda_only(fee_payer, compression_config, rent_sponsor)
    };

    let hot_addresses: Vec<Pubkey> = specs.iter().map(|s| s.address()).collect();
    let cold_accounts: Vec<(CompressedAccount, V)> = specs
        .iter()
        .map(|s| {
            let compressed = s.compressed().expect("cold spec must have data").clone();
            (compressed, s.variant.clone())
        })
        .collect();

    let program_id = specs.first().map(|s| s.program_id()).unwrap_or_default();

    instructions::create_decompress_accounts_idempotent_instruction(
        &program_id,
        &DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
        &hot_addresses,
        &cold_accounts,
        &metas,
        proof,
    )
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}

struct AtaContext<'a> {
    compressed: &'a CompressedTokenAccount,
    wallet_owner: Pubkey,
    mint: Pubkey,
    bump: u8,
}

impl<'a> AtaContext<'a> {
    fn from_interface(
        iface: &'a TokenAccountInterface,
        index: usize,
    ) -> Result<Self, LoadAccountsError> {
        let compressed = iface
            .compressed()
            .ok_or(LoadAccountsError::MissingAtaContext {
                index,
                pubkey: iface.key,
            })?;
        let bump = iface
            .ata_bump()
            .ok_or(LoadAccountsError::MissingAtaContext {
                index,
                pubkey: iface.key,
            })?;
        Ok(Self {
            compressed,
            wallet_owner: iface.owner(),
            mint: iface.mint(),
            bump,
        })
    }
}

fn build_ata_load(
    ifaces: &[&TokenAccountInterface],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Vec<Instruction>, LoadAccountsError> {
    let contexts: Vec<AtaContext> = ifaces
        .iter()
        .enumerate()
        .map(|(i, a)| AtaContext::from_interface(a, i))
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = Vec::with_capacity(contexts.len() + 1);

    for ctx in &contexts {
        let ix = CreateAssociatedTokenAccount::new(fee_payer, ctx.wallet_owner, ctx.mint)
            .idempotent()
            .instruction()
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        out.push(ix);
    }

    out.push(build_transfer2(&contexts, proof, fee_payer)?);
    Ok(out)
}

fn build_transfer2(
    contexts: &[AtaContext],
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Instruction, LoadAccountsError> {
    let mut packed = PackedAccounts::default();
    let packed_trees = proof.pack_tree_infos(&mut packed);
    let tree_infos = packed_trees
        .state_trees
        .as_ref()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("no state trees".into()))?;

    let mut token_accounts = Vec::with_capacity(contexts.len());
    let mut tlv_data: Vec<Vec<ExtensionInstructionData>> = Vec::with_capacity(contexts.len());
    let mut has_tlv = false;

    for (i, ctx) in contexts.iter().enumerate() {
        let token = &ctx.compressed.token;
        let tree = tree_infos.packed_tree_infos.get(i).ok_or(
            LoadAccountsError::TreeInfoIndexOutOfBounds {
                index: i,
                len: tree_infos.packed_tree_infos.len(),
            },
        )?;

        let owner_idx = packed.insert_or_get_config(ctx.wallet_owner, true, false);
        let ata_idx = packed.insert_or_get(derive_token_ata(&ctx.wallet_owner, &ctx.mint));
        let mint_idx = packed.insert_or_get(token.mint);
        let delegate_idx = token.delegate.map(|d| packed.insert_or_get(d)).unwrap_or(0);

        let source = MultiInputTokenDataWithContext {
            owner: ata_idx,
            amount: token.amount,
            has_delegate: token.delegate.is_some(),
            delegate: delegate_idx,
            mint: mint_idx,
            version: TokenDataVersion::ShaFlat as u8,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree.merkle_tree_pubkey_index,
                queue_pubkey_index: tree.queue_pubkey_index,
                prove_by_index: tree.prove_by_index,
                leaf_index: tree.leaf_index,
            },
            root_index: tree.root_index,
        };

        let mut ctoken = CTokenAccount2::new(vec![source])
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        ctoken
            .decompress(token.amount, ata_idx)
            .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))?;
        token_accounts.push(ctoken);

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
                                    compression_index: i as u8,
                                    is_ata: true,
                                    bump: ctx.bump,
                                    owner_index: owner_idx,
                                },
                            ))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !tlv.is_empty() {
            has_tlv = true;
        }
        tlv_data.push(tlv);
    }

    let (metas, _, _) = packed.to_account_metas();

    create_transfer2_instruction(Transfer2Inputs {
        meta_config: Transfer2AccountsMetaConfig::new(fee_payer, metas),
        token_accounts,
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        validity_proof: proof.proof,
        in_tlv: if has_tlv { Some(tlv_data) } else { None },
        ..Default::default()
    })
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}

fn build_mint_load(
    iface: &AccountInterface,
    proof: ValidityProofWithContext,
    fee_payer: Pubkey,
) -> Result<Instruction, LoadAccountsError> {
    let acc = proof
        .accounts
        .first()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("proof has no accounts".into()))?;
    let state_tree = acc.tree_info.tree;
    let input_queue = acc.tree_info.queue;
    let output_queue = acc
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(input_queue);

    let mint_data = iface
        .as_mint()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("missing mint_data".into()))?;
    let compressed_address = iface
        .mint_compressed_address()
        .ok_or_else(|| LoadAccountsError::BuildInstruction("missing compressed_address".into()))?;
    let mint_ix_data = MintInstructionData::try_from(mint_data)
        .map_err(|_| LoadAccountsError::BuildInstruction("invalid mint data".into()))?;

    DecompressMint {
        payer: fee_payer,
        authority: fee_payer,
        state_tree,
        input_queue,
        output_queue,
        compressed_mint_with_context: MintWithContext {
            leaf_index: acc.leaf_index as u32,
            prove_by_index: acc.root_index.proof_by_index(),
            root_index: acc.root_index.root_index().unwrap_or_default(),
            address: compressed_address,
            mint: Some(mint_ix_data),
        },
        proof: ValidityProof(proof.proof.into()),
        rent_payment: DEFAULT_RENT_PAYMENT,
        write_top_up: DEFAULT_WRITE_TOP_UP,
    }
    .instruction()
    .map_err(|e| LoadAccountsError::BuildInstruction(e.to_string()))
}
