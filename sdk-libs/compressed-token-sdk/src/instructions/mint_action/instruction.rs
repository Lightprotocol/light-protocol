use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::{
        create_compressed_mint::CompressedMintWithContext,
        mint_actions::{
            Action, CpiContext, CreateSplMintAction, MintActionCompressedInstructionData,
            UpdateAuthority,
        },
        mint_to_compressed::{MintToAction, Recipient},
    },
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::mint_action::account_metas::{
        get_mint_action_instruction_account_metas,
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
    AnchorDeserialize, AnchorSerialize,
};

pub const MINT_ACTION_DISCRIMINATOR: u8 = 106;

/// Input struct for creating a mint action instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintActionInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_seed: Pubkey,
    pub create_mint: bool,     // Whether we're creating a new compressed mint
    pub mint_bump: Option<u8>, // Bump seed for creating SPL mint
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub proof: Option<CompressedProof>,
    pub actions: Vec<MintActionType>,
    pub address_tree_pubkey: Pubkey,
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
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
    MintToDecompressed {
        account: Pubkey,
        amount: u64,
        compressible_config: Option<light_ctoken_types::instructions::extensions::compressible::CompressibleExtensionInstructionData>,
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
    let create_mint = input.create_mint;
    let mint_bump = input.mint_bump.unwrap_or(0u8);

    // Check for lamports and decompressed status before moving
    let with_lamports = input.actions.iter().any(|action| {
        matches!(
            action,
            MintActionType::MintTo {
                lamports: Some(_),
                ..
            }
        )
    });
    let is_decompressed = input
        .actions
        .iter()
        .any(|action| matches!(action, MintActionType::CreateSplMint { .. }))
        || input.compressed_mint_inputs.mint.is_decompressed;
    let with_cpi_context = cpi_context.is_some();
    // Match onchain logic: with_mint_signer = create_mint() | has_CreateSplMint_action
    let with_mint_signer = create_mint
        || input
            .actions
            .iter()
            .any(|action| matches!(action, MintActionType::CreateSplMint { .. }));

    // Collect decompressed accounts for account index mapping
    let mut decompressed_accounts: Vec<Pubkey> = Vec::new();
    let mut decompressed_account_index = 0u8;

    for action in input.actions {
        match action {
            MintActionType::CreateSplMint { mint_bump: bump } => {
                program_actions.push(Action::CreateSplMint(CreateSplMintAction {
                    mint_bump: bump,
                }));
            }
            MintActionType::MintTo {
                recipients,
                lamports,
                token_account_version,
            } => {
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
            MintActionType::MintToDecompressed {
                account,
                amount,
                compressible_config,
            } => {
                use light_ctoken_types::instructions::mint_actions::{
                    DecompressedRecipient, MintToDecompressedAction,
                };

                // Add account to decompressed accounts list and get its index
                decompressed_accounts.push(account);
                let current_index = decompressed_account_index;
                decompressed_account_index += 1;

                program_actions.push(Action::MintToDecompressed(MintToDecompressedAction {
                    recipient: DecompressedRecipient {
                        account_index: current_index,
                        amount,
                        compressible_config,
                    },
                }));
            }
        }
    }

    // Create account meta config first (before moving compressed_mint_inputs)
    let meta_config = MintActionMetaConfig {
        fee_payer: Some(input.payer),
        mint_signer: if with_mint_signer {
            Some(input.mint_seed)
        } else {
            None
        },
        authority: input.authority,
        tree_pubkey: input.address_tree_pubkey,
        output_queue: input.output_queue,
        with_lamports,
        is_decompressed,
        with_cpi_context,
        create_mint,
        with_mint_signer,
        decompressed_token_accounts: decompressed_accounts,
    };

    // Get account metas (before moving compressed_mint_inputs)
    let accounts =
        get_mint_action_instruction_account_metas(meta_config, &input.compressed_mint_inputs);

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

/// Input struct for creating a mint action CPI write instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintActionInputsCpiWrite {
    pub compressed_mint_inputs:
        light_ctoken_types::instructions::create_compressed_mint::CompressedMintWithContext,
    pub mint_seed: Option<Pubkey>, // Optional - only when creating mint and when creating SPL mint
    pub mint_bump: Option<u8>,     // Bump seed for creating SPL mint
    pub create_mint: bool,         // Whether we're creating a new mint
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub actions: Vec<MintActionType>,
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub cpi_context: light_ctoken_types::instructions::mint_actions::CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Creates a mint action CPI write instruction (for use in CPI context)
pub fn mint_action_cpi_write(input: MintActionInputsCpiWrite) -> Result<Instruction> {
    use light_ctoken_types::instructions::mint_actions::MintActionCompressedInstructionData;

    // Validate CPI context
    if !input.cpi_context.first_set_context && !input.cpi_context.set_context {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Convert high-level actions to program-level actions
    let mut program_actions = Vec::new();
    let create_mint = input.create_mint;
    let mint_bump = input.mint_bump.unwrap_or(0u8);

    let with_mint_signer = create_mint
        || input
            .actions
            .iter()
            .any(|action| matches!(action, MintActionType::CreateSplMint { .. }));

    // Collect decompressed accounts for account index mapping (CPI write version)
    let mut decompressed_accounts: Vec<Pubkey> = Vec::new();
    let mut decompressed_account_index = 0u8;

    for action in input.actions {
        match action {
            MintActionType::CreateSplMint { mint_bump: bump } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_actions::Action::CreateSplMint(
                        light_ctoken_types::instructions::mint_actions::CreateSplMintAction {
                            mint_bump: bump,
                        },
                    ),
                );
            }
            MintActionType::MintTo {
                recipients,
                lamports,
                token_account_version,
            } => {
                let program_recipients: Vec<_> = recipients
                    .into_iter()
                    .map(
                        |r| light_ctoken_types::instructions::mint_to_compressed::Recipient {
                            recipient: r.recipient.to_bytes().into(),
                            amount: r.amount,
                        },
                    )
                    .collect();

                program_actions.push(
                    light_ctoken_types::instructions::mint_actions::Action::MintTo(
                        light_ctoken_types::instructions::mint_to_compressed::MintToAction {
                            token_account_version,
                            recipients: program_recipients,
                            lamports,
                        },
                    ),
                );
            }
            MintActionType::UpdateMintAuthority { new_authority } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_actions::Action::UpdateMintAuthority(
                        light_ctoken_types::instructions::mint_actions::UpdateAuthority {
                            new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                        },
                    ),
                );
            }
            MintActionType::UpdateFreezeAuthority { new_authority } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_actions::Action::UpdateFreezeAuthority(
                        light_ctoken_types::instructions::mint_actions::UpdateAuthority {
                            new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                        },
                    ),
                );
            }
            MintActionType::MintToDecompressed {
                account,
                amount,
                compressible_config,
            } => {
                use light_ctoken_types::instructions::mint_actions::{
                    DecompressedRecipient, MintToDecompressedAction,
                };

                // Add account to decompressed accounts list and get its index
                decompressed_accounts.push(account);
                let current_index = decompressed_account_index;
                decompressed_account_index += 1;

                program_actions.push(
                    light_ctoken_types::instructions::mint_actions::Action::MintToDecompressed(
                        MintToDecompressedAction {
                            recipient: DecompressedRecipient {
                                account_index: current_index,
                                amount,
                                compressible_config,
                            },
                        },
                    ),
                );
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
        proof: None, // No proof for CPI write context
        cpi_context: Some(input.cpi_context),
    };

    // Create account meta config for CPI write
    let meta_config = MintActionMetaConfigCpiWrite {
        fee_payer: input.payer,
        mint_signer: if with_mint_signer {
            input.mint_seed
        } else {
            None
        },
        authority: input.authority,
        cpi_context: input.cpi_context_pubkey,
        decompressed_token_accounts: decompressed_accounts,
    };

    // Get account metas
    let accounts = get_mint_action_instruction_account_metas_cpi_write(meta_config);

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
