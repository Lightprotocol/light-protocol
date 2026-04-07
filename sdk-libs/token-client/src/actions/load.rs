//! Load action for light token accounts.
//!
//! Behavior mirrors JS token-interface `createLoadInstructions` for the common
//! light-ATA path:
//! - optional wrap of SPL/T22 ATA balances into light ATA
//! - optional decompress of one primary cold compressed account into light ATA
//! - owner/delegate authority checks

use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_token::{
    constants::{LIGHT_TOKEN_PROGRAM_ID, SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID},
    instruction::{CreateAssociatedTokenAccount, Decompress, TransferFromSpl},
    spl_interface::{find_spl_interface_pda, has_restricted_extensions},
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use spl_token_2022::{solana_program::program_pack::Pack, state::Mint as SplMint};

use crate::read::{
    default_token_data_discriminator, filter_account_for_authority, get_ata_view_for_load_or_none,
    is_authority_for_account, select_primary_cold_account_for_load, TokenAccountSourceType,
};

/// Parameters for loading a light token ATA.
#[derive(Clone, Debug)]
pub struct Load {
    /// Wallet owner of the associated light token account.
    pub owner: Pubkey,
    /// Mint for the account.
    pub mint: Pubkey,
    /// If true, wrap SPL/T22 ATA balances into light ATA first.
    pub wrap: bool,
    /// If false, fail when any source is frozen.
    pub allow_frozen: bool,
    /// Optional decimals override. If omitted, fetched from mint account.
    pub decimals: Option<u8>,
}

pub async fn create_load_instructions<R: Rpc + Indexer>(
    rpc: &R,
    load: &Load,
    fee_payer: Pubkey,
    authority: Pubkey,
) -> Result<Vec<Instruction>, RpcError> {
    load.instructions(rpc, fee_payer, authority).await
}

impl Default for Load {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            mint: Pubkey::default(),
            wrap: true,
            allow_frozen: false,
            decimals: None,
        }
    }
}

impl Load {
    /// Build load instructions without sending.
    pub async fn instructions<R: Rpc + Indexer>(
        &self,
        rpc: &R,
        payer: Pubkey,
        authority: Pubkey,
    ) -> Result<Vec<Instruction>, RpcError> {
        let mut view =
            match get_ata_view_for_load_or_none(rpc, self.owner, self.mint, self.wrap).await? {
                Some(view) => view,
                None => return Ok(vec![]),
            };

        if !self.allow_frozen && view.any_frozen {
            return Err(RpcError::CustomError(
                "Account is frozen. load is not allowed.".to_string(),
            ));
        }

        if authority != self.owner {
            if !is_authority_for_account(&view, &authority) {
                return Err(RpcError::CustomError(
                    "Signer is not the owner or a delegate of the account.".to_string(),
                ));
            }
            view = filter_account_for_authority(&view, &authority);
        }

        if view.sources.is_empty() {
            return Ok(vec![]);
        }

        let light_ata = view.address;
        let has_light_hot = view
            .sources
            .iter()
            .any(|source| source.source_type == TokenAccountSourceType::LightTokenHot);
        let spl_source = view
            .sources
            .iter()
            .find(|source| source.source_type == TokenAccountSourceType::Spl && source.amount > 0);
        let t22_source = view.sources.iter().find(|source| {
            source.source_type == TokenAccountSourceType::Token2022 && source.amount > 0
        });
        let primary_cold = select_primary_cold_account_for_load(&view.sources);

        if spl_source.is_none() && t22_source.is_none() && primary_cold.is_none() {
            return Ok(vec![]);
        }

        let mut instructions = Vec::<Instruction>::new();

        let needs_light_ata = !has_light_hot
            && (primary_cold.is_some()
                || (self.wrap && (spl_source.is_some() || t22_source.is_some())));
        if needs_light_ata {
            let create_ata_ix = CreateAssociatedTokenAccount::new(payer, self.owner, self.mint)
                .idempotent()
                .instruction()
                .map_err(|error| {
                    RpcError::CustomError(format!(
                        "Failed to build light ATA create instruction: {error}"
                    ))
                })?;
            instructions.push(create_ata_ix);
        }

        if self.wrap {
            if spl_source.is_some() || t22_source.is_some() {
                let decimals = match self.decimals {
                    Some(decimals) => decimals,
                    None => fetch_mint_decimals(rpc, self.mint).await?,
                };

                if let Some(source) = spl_source {
                    instructions.push(
                        build_wrap_instruction(
                            rpc,
                            source.address,
                            light_ata,
                            self.mint,
                            source.amount,
                            decimals,
                            payer,
                            authority,
                            SPL_TOKEN_PROGRAM_ID,
                        )
                        .await?,
                    );
                }
                if let Some(source) = t22_source {
                    instructions.push(
                        build_wrap_instruction(
                            rpc,
                            source.address,
                            light_ata,
                            self.mint,
                            source.amount,
                            decimals,
                            payer,
                            authority,
                            SPL_TOKEN_2022_PROGRAM_ID,
                        )
                        .await?,
                    );
                }
            }
        }

        if let Some(primary_cold_account) = primary_cold {
            let proof = rpc
                .get_validity_proof(vec![primary_cold_account.account.hash], vec![], None)
                .await
                .map_err(|error| {
                    RpcError::CustomError(format!(
                        "Failed to fetch validity proof for load: {error}"
                    ))
                })?
                .value;
            let proof_account = proof.accounts.first().ok_or_else(|| {
                RpcError::CustomError("Validity proof did not include account inputs".to_string())
            })?;

            let decompress_ix = Decompress {
                token_data: primary_cold_account.token.clone().into(),
                discriminator: default_token_data_discriminator(&primary_cold_account),
                merkle_tree: primary_cold_account.account.tree_info.tree,
                queue: primary_cold_account.account.tree_info.queue,
                leaf_index: primary_cold_account.account.leaf_index,
                root_index: proof_account.root_index.root_index().unwrap_or_default(),
                destination: light_ata,
                payer,
                signer: authority,
                validity_proof: proof.proof,
            }
            .instruction()
            .map_err(|error| {
                RpcError::CustomError(format!("Failed to build decompress instruction: {error}"))
            })?;

            instructions.push(decompress_ix);
        }

        Ok(instructions)
    }

    /// Build and send the load transaction.
    pub async fn execute<R: Rpc + Indexer>(
        self,
        rpc: &mut R,
        payer: &Keypair,
        authority: &Keypair,
    ) -> Result<Option<Signature>, RpcError> {
        let instructions =
            create_load_instructions(rpc, &self, payer.pubkey(), authority.pubkey()).await?;
        if instructions.is_empty() {
            return Ok(None);
        }

        let mut signers: Vec<&Keypair> = vec![payer];
        if authority.pubkey() != payer.pubkey() {
            signers.push(authority);
        }

        let signature = rpc
            .create_and_send_transaction(&instructions, &payer.pubkey(), &signers)
            .await?;
        Ok(Some(signature))
    }
}

async fn fetch_mint_decimals<R: Rpc>(rpc: &R, mint: Pubkey) -> Result<u8, RpcError> {
    let mint_account = rpc
        .get_account(mint)
        .await?
        .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;

    if mint_account.owner == SPL_TOKEN_PROGRAM_ID || mint_account.owner == SPL_TOKEN_2022_PROGRAM_ID
    {
        if mint_account.data.len() < SplMint::LEN {
            return Err(RpcError::CustomError(format!(
                "Mint account data too short: expected at least {}, got {}",
                SplMint::LEN,
                mint_account.data.len()
            )));
        }
        let mint_state = SplMint::unpack(&mint_account.data[..SplMint::LEN]).map_err(|error| {
            RpcError::CustomError(format!("Failed to parse SPL mint account: {error}"))
        })?;
        return Ok(mint_state.decimals);
    }

    if mint_account.owner == LIGHT_TOKEN_PROGRAM_ID {
        let light_mint = light_token_interface::state::Mint::deserialize(
            &mut &mint_account.data[..],
        )
        .map_err(|error| {
            RpcError::CustomError(format!("Failed to parse light mint account: {error}"))
        })?;
        return Ok(light_mint.base.decimals);
    }

    Err(RpcError::CustomError(format!(
        "Unsupported mint owner for decimals fetch: {}",
        mint_account.owner
    )))
}

async fn build_wrap_instruction<R: Rpc>(
    rpc: &R,
    source_spl_ata: Pubkey,
    destination: Pubkey,
    mint: Pubkey,
    amount: u64,
    decimals: u8,
    payer: Pubkey,
    authority: Pubkey,
    spl_token_program: Pubkey,
) -> Result<Instruction, RpcError> {
    if spl_token_program != SPL_TOKEN_PROGRAM_ID && spl_token_program != SPL_TOKEN_2022_PROGRAM_ID {
        return Err(RpcError::CustomError(format!(
            "Unsupported SPL token program for wrap: {}",
            spl_token_program
        )));
    }

    let restricted = if spl_token_program == SPL_TOKEN_2022_PROGRAM_ID {
        let mint_account = rpc
            .get_account(mint)
            .await?
            .ok_or_else(|| RpcError::CustomError("Mint account not found".to_string()))?;
        has_restricted_extensions(&mint_account.data)
    } else {
        false
    };

    let (spl_interface_pda, bump) = find_spl_interface_pda(&mint, restricted);

    TransferFromSpl {
        amount,
        spl_interface_pda_bump: bump,
        decimals,
        source_spl_token_account: source_spl_ata,
        destination,
        authority,
        mint,
        payer,
        spl_interface_pda,
        spl_token_program,
    }
    .instruction()
    .map_err(|error| RpcError::CustomError(format!("Failed to build wrap instruction: {error}")))
}
