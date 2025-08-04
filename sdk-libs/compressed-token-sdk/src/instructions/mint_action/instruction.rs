use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::{
        mint_actions::{Action, CpiContext, CreateSplMintAction, MintActionCompressedInstructionData, UpdateAuthority},
        mint_to_compressed::{MintToAction, Recipient},
    },
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::account_metas::{
        get_mint_action_instruction_account_metas, MintActionMetaConfig,
    },
    AnchorDeserialize, AnchorSerialize,
};

pub const MINT_ACTION_DISCRIMINATOR: u8 = 106;

/// Input struct for creating a mint action instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintActionInputs {
    pub compressed_mint_inputs: light_ctoken_types::instructions::create_compressed_mint::CompressedMintWithContext,
    pub mint_seed: Pubkey,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub proof: Option<CompressedProof>,
    pub actions: Vec<MintActionType>,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
    pub cpi_context: Option<CpiContext>,
}

/// High-level action types for the mint action instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub enum MintActionType {
    CreateSplMint {
        mint_bump: u8,
    },
    MintTo {
        recipients: Vec<MintToRecipient>,
        lamports: Option<u64>,
        token_account_version: u8,
    },
    UpdateMintAuthority {
        new_authority: Option<Pubkey>,
    },
    UpdateFreezeAuthority {
        new_authority: Option<Pubkey>,
    },
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintToRecipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

/// Creates a mint action instruction
pub fn create_mint_action_cpi(
    input: MintActionInputs,
    cpi_context: Option<CpiContext>,
) -> Result<Instruction> {
    // Convert high-level actions to program-level actions
    let mut program_actions = Vec::new();
    let mut create_mint = false;
    let mut mint_bump = 0u8;

    // Check for lamports and decompressed status before moving
    let with_lamports = input.actions.iter().any(|action| matches!(action, MintActionType::MintTo { lamports: Some(_), .. }));
    let is_decompressed = input.actions.iter().any(|action| matches!(action, MintActionType::CreateSplMint { .. }));
    let with_cpi_context = cpi_context.is_some();

    for action in input.actions {
        match action {
            MintActionType::CreateSplMint { mint_bump: bump } => {
                program_actions.push(Action::CreateSplMint(CreateSplMintAction {
                    mint_bump: bump,
                }));
                create_mint = true;
                mint_bump = bump;
            }
            MintActionType::MintTo { recipients, lamports, token_account_version } => {
                let program_recipients: Vec<_> = recipients
                    .into_iter()
                    .map(|r| Recipient {
                        recipient: r.recipient.to_bytes().into(),
                        amount: r.amount,
                    })
                    .collect();

                program_actions.push(Action::MintTo(MintToAction {
                    token_account_version,
                    recipients: program_recipients,
                    lamports,
                }));
            }
            MintActionType::UpdateMintAuthority { new_authority } => {
                program_actions.push(Action::UpdateMintAuthority(UpdateAuthority {
                    new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                }));
            }
            MintActionType::UpdateFreezeAuthority { new_authority } => {
                program_actions.push(Action::UpdateFreezeAuthority(UpdateAuthority {
                    new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                }));
            }
        }
    }

    let instruction_data = MintActionCompressedInstructionData {
        create_mint,
        mint_bump,
        leaf_index: input.compressed_mint_inputs.leaf_index,
        prove_by_index: input.compressed_mint_inputs.prove_by_index,
        root_index: input.compressed_mint_inputs.root_index,
        compressed_address: input.compressed_mint_inputs.address,
        mint: input.compressed_mint_inputs.mint,
        actions: program_actions,
        proof: input.proof,
        cpi_context,
    };

    // Create account meta config
    let meta_config = MintActionMetaConfig {
        fee_payer: Some(input.payer),
        mint_signer: Some(input.mint_seed),
        authority: input.authority,
        address_tree_pubkey: input.address_tree_pubkey,
        output_queue: input.output_queue,
        with_lamports,
        is_decompressed,
        with_cpi_context,
        create_mint,
    };

    // Get account metas
    let accounts = get_mint_action_instruction_account_metas(meta_config);

    // Serialize instruction data
    let data_vec = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: [vec![MINT_ACTION_DISCRIMINATOR], data_vec].concat(),
    })
}

/// Creates a mint action instruction without CPI context
pub fn create_mint_action(input: MintActionInputs) -> Result<Instruction> {
    create_mint_action_cpi(input, None)
}