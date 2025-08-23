use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::mint_action::{
        Action, CompressedMintWithContext, CpiContext, CreateSplMintAction,
        MintActionCompressedInstructionData, MintToAction, Recipient, RemoveMetadataKeyAction,
        UpdateAuthority, UpdateMetadataAuthorityAction, UpdateMetadataFieldAction,
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
    pub tokens_out_queue: Option<Pubkey>, // Output queue for new token accounts
    pub token_pool: Option<TokenPool>,
}

impl MintActionInputs {
    /// Creates a new MintActionInputs for creating a compressed mint
    pub fn new_for_create_mint(
        compressed_mint_inputs: CompressedMintWithContext,
        actions: Vec<MintActionType>,
        output_queue: Pubkey,
        address_tree_pubkey: Pubkey,
        mint_seed: Pubkey,
        mint_bump: Option<u8>,
        authority: Pubkey,
        payer: Pubkey,
        proof: Option<CompressedProof>,
    ) -> Self {
        Self {
            compressed_mint_inputs,
            mint_seed,
            create_mint: true, // Always true for create mint
            mint_bump,
            authority,
            payer,
            proof,
            actions,
            address_tree_pubkey,
            input_queue: None, // No input queue when creating new mint
            output_queue,
            tokens_out_queue: Some(output_queue), // Default to None, can be set separately if needed
            token_pool: None, // Default to None, can be set separately if needed
        }
    }
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
    },
    UpdateMetadataField {
        extension_index: u8,
        field_type: u8,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    UpdateMetadataAuthority {
        extension_index: u8,
        new_authority: Pubkey,
    },
    RemoveMetadataKey {
        extension_index: u8,
        key: Vec<u8>,
        idempotent: u8,
    },
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintToRecipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct TokenPool {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}

/// Creates a mint action instruction
pub fn create_mint_action_cpi(
    input: MintActionInputs,
    cpi_context: Option<CpiContext>,
    cpi_context_pubkey: Option<Pubkey>,
) -> Result<Instruction> {
    // Convert high-level actions to program-level actions
    let mut program_actions = Vec::new();
    let create_mint = input.create_mint;
    let mint_bump = input.mint_bump.unwrap_or(0u8);

    // Check for lamports, decompressed status, and mint actions before moving
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
    let has_mint_to_actions = input.actions.iter().any(|action| {
        matches!(
            action,
            MintActionType::MintTo { .. } | MintActionType::MintToDecompressed { .. }
        )
    });
    // Match onchain logic: with_mint_signer = create_mint() | has_CreateSplMint_action
    let with_mint_signer = create_mint
        || input
            .actions
            .iter()
            .any(|action| matches!(action, MintActionType::CreateSplMint { .. }));

    // Only require mint to sign when creating a new compressed mint
    let mint_needs_to_sign = create_mint;

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
            MintActionType::MintToDecompressed { account, amount } => {
                use light_ctoken_types::instructions::mint_action::{
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
                    },
                }));
            }
            MintActionType::UpdateMetadataField {
                extension_index,
                field_type,
                key,
                value,
            } => {
                program_actions.push(Action::UpdateMetadataField(UpdateMetadataFieldAction {
                    extension_index,
                    field_type,
                    key,
                    value,
                }));
            }
            MintActionType::UpdateMetadataAuthority {
                extension_index,
                new_authority,
            } => {
                program_actions.push(Action::UpdateMetadataAuthority(
                    UpdateMetadataAuthorityAction {
                        extension_index,
                        new_authority: new_authority.to_bytes().into(),
                    },
                ));
            }
            MintActionType::RemoveMetadataKey {
                extension_index,
                key,
                idempotent,
            } => {
                program_actions.push(Action::RemoveMetadataKey(RemoveMetadataKeyAction {
                    extension_index,
                    key,
                    idempotent,
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
        input_queue: input.input_queue,
        output_queue: input.output_queue,
        tokens_out_queue: input.tokens_out_queue,
        with_lamports,
        is_decompressed,
        has_mint_to_actions,
        with_cpi_context: cpi_context_pubkey,
        create_mint,
        with_mint_signer,
        mint_needs_to_sign,
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
        token_pool_bump: input.token_pool.as_ref().map_or(0, |tp| tp.bump),
        token_pool_index: input.token_pool.as_ref().map_or(0, |tp| tp.index),
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
    create_mint_action_cpi(input, None, None)
}

/// Input struct for creating a mint action CPI write instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MintActionInputsCpiWrite {
    pub compressed_mint_inputs:
        light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
    pub mint_seed: Option<Pubkey>, // Optional - only when creating mint and when creating SPL mint
    pub mint_bump: Option<u8>,     // Bump seed for creating SPL mint
    pub create_mint: bool,         // Whether we're creating a new mint
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub actions: Vec<MintActionType>,
    pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub cpi_context: light_ctoken_types::instructions::mint_action::CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Creates a mint action CPI write instruction (for use in CPI context)
pub fn mint_action_cpi_write(input: MintActionInputsCpiWrite) -> Result<Instruction> {
    use light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData;

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

    // Only require mint to sign when creating a new compressed mint
    let mint_needs_to_sign = create_mint;

    // Collect decompressed accounts for account index mapping (CPI write version)
    let mut decompressed_accounts: Vec<Pubkey> = Vec::new();
    let mut decompressed_account_index = 0u8;

    for action in input.actions {
        match action {
            MintActionType::CreateSplMint { mint_bump: bump } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::CreateSplMint(
                        light_ctoken_types::instructions::mint_action::CreateSplMintAction {
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
                        |r| light_ctoken_types::instructions::mint_action::Recipient {
                            recipient: r.recipient.to_bytes().into(),
                            amount: r.amount,
                        },
                    )
                    .collect();

                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::MintTo(
                        light_ctoken_types::instructions::mint_action::MintToAction {
                            token_account_version,
                            recipients: program_recipients,
                            lamports,
                        },
                    ),
                );
            }
            MintActionType::UpdateMintAuthority { new_authority } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::UpdateMintAuthority(
                        light_ctoken_types::instructions::mint_action::UpdateAuthority {
                            new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                        },
                    ),
                );
            }
            MintActionType::UpdateFreezeAuthority { new_authority } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::UpdateFreezeAuthority(
                        light_ctoken_types::instructions::mint_action::UpdateAuthority {
                            new_authority: new_authority.map(|auth| auth.to_bytes().into()),
                        },
                    ),
                );
            }
            MintActionType::MintToDecompressed { account, amount } => {
                use light_ctoken_types::instructions::mint_action::{
                    DecompressedRecipient, MintToDecompressedAction,
                };

                // Add account to decompressed accounts list and get its index
                decompressed_accounts.push(account);
                let current_index = decompressed_account_index;
                decompressed_account_index += 1;

                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::MintToDecompressed(
                        MintToDecompressedAction {
                            recipient: DecompressedRecipient {
                                account_index: current_index,
                                amount,
                            },
                        },
                    ),
                );
            }
            MintActionType::UpdateMetadataField {
                extension_index,
                field_type,
                key,
                value,
            } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::UpdateMetadataField(
                        UpdateMetadataFieldAction {
                            extension_index,
                            field_type,
                            key,
                            value,
                        },
                    ),
                );
            }
            MintActionType::UpdateMetadataAuthority {
                extension_index,
                new_authority,
            } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::UpdateMetadataAuthority(
                        UpdateMetadataAuthorityAction {
                            extension_index,
                            new_authority: new_authority.to_bytes().into(),
                        },
                    ),
                );
            }
            MintActionType::RemoveMetadataKey {
                extension_index,
                key,
                idempotent,
            } => {
                program_actions.push(
                    light_ctoken_types::instructions::mint_action::Action::RemoveMetadataKey(
                        RemoveMetadataKeyAction {
                            extension_index,
                            key,
                            idempotent,
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
        token_pool_bump: 0,  // Not used in CPI write context
        token_pool_index: 0, // Not used in CPI write context
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
        mint_needs_to_sign,
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
