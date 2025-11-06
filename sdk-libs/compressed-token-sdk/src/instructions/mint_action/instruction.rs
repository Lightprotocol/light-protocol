use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_ctoken_types::{
    self,
    instructions::mint_action::{
        Action, CompressedMintWithContext, CpiContext, CreateMint, CreateSplMintAction,
        MintActionCompressedInstructionData, MintToCompressedAction, Recipient,
        RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
        UpdateMetadataFieldAction,
    },
};
use light_program_profiler::profile;
use solana_instruction::Instruction;
use solana_msg::msg;
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

pub const MINT_ACTION_DISCRIMINATOR: u8 = 103;

/// Input parameters for creating a new mint
#[derive(Debug, Clone)]
pub struct CreateMintInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_seed: Pubkey,
    pub mint_bump: u8,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub proof: Option<CompressedProof>,
    pub address_tree: Pubkey,
    pub output_queue: Pubkey,
}

/// Input parameters for working with an existing mint
#[derive(Debug, Clone)]
pub struct WithMintInputs {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_seed: Pubkey,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub proof: Option<CompressedProof>,
    pub state_tree: Pubkey,
    pub input_queue: Pubkey,
    pub output_queue: Pubkey,
    pub token_pool: Option<TokenPool>, // Required if mint is decompressed
}

/// Input struct for creating a mint action instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
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
    /// Create a new compressed mint (starting point for new mints)
    pub fn new_create_mint(inputs: CreateMintInputs) -> Self {
        Self {
            compressed_mint_inputs: inputs.compressed_mint_inputs,
            mint_seed: inputs.mint_seed,
            create_mint: true,
            mint_bump: Some(inputs.mint_bump),
            authority: inputs.authority,
            payer: inputs.payer,
            proof: inputs.proof,
            actions: Vec::new(),
            address_tree_pubkey: inputs.address_tree,
            input_queue: None,
            output_queue: inputs.output_queue,
            tokens_out_queue: None,
            token_pool: None,
        }
    }

    /// Start with an existing mint (starting point for existing mints)
    pub fn new_with_mint(inputs: WithMintInputs) -> Self {
        Self {
            compressed_mint_inputs: inputs.compressed_mint_inputs,
            mint_seed: inputs.mint_seed,
            create_mint: false,
            mint_bump: None,
            authority: inputs.authority,
            payer: inputs.payer,
            proof: inputs.proof,
            actions: Vec::new(),
            address_tree_pubkey: inputs.state_tree,
            input_queue: Some(inputs.input_queue),
            output_queue: inputs.output_queue,
            tokens_out_queue: None,
            token_pool: inputs.token_pool,
        }
    }

    /// Add CreateSplMint action - creates SPL mint and token pool
    pub fn add_create_spl_mint(mut self, mint_bump: u8, token_pool: TokenPool) -> Self {
        self.actions
            .push(MintActionType::CreateSplMint { mint_bump });
        self.token_pool = Some(token_pool);
        self
    }

    /// Add MintTo action - mint tokens to compressed token accounts
    pub fn add_mint_to(
        mut self,
        recipients: Vec<MintToRecipient>,
        token_account_version: u8,
        tokens_out_queue: Option<Pubkey>,
    ) -> Self {
        self.actions.push(MintActionType::MintTo {
            recipients,
            token_account_version,
        });
        // Set tokens_out_queue if not already set
        if self.tokens_out_queue.is_none() {
            self.tokens_out_queue = tokens_out_queue.or(Some(self.output_queue));
        }
        self
    }

    /// Add MintToCToken action - mint to SPL token accounts
    pub fn add_mint_to_decompressed(mut self, account: Pubkey, amount: u64) -> Self {
        self.actions
            .push(MintActionType::MintToCToken { account, amount });
        self
    }

    /// Add UpdateMintAuthority action
    pub fn add_update_mint_authority(mut self, new_authority: Option<Pubkey>) -> Self {
        self.actions
            .push(MintActionType::UpdateMintAuthority { new_authority });
        self
    }

    /// Add UpdateFreezeAuthority action
    pub fn add_update_freeze_authority(mut self, new_authority: Option<Pubkey>) -> Self {
        self.actions
            .push(MintActionType::UpdateFreezeAuthority { new_authority });
        self
    }

    /// Add UpdateMetadataField action
    pub fn add_update_metadata_field(
        mut self,
        extension_index: u8,
        field_type: u8,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Self {
        self.actions.push(MintActionType::UpdateMetadataField {
            extension_index,
            field_type,
            key,
            value,
        });
        self
    }

    /// Add UpdateMetadataAuthority action
    pub fn add_update_metadata_authority(
        mut self,
        extension_index: u8,
        new_authority: Pubkey,
    ) -> Self {
        self.actions.push(MintActionType::UpdateMetadataAuthority {
            extension_index,
            new_authority,
        });
        self
    }

    /// Add RemoveMetadataKey action
    pub fn add_remove_metadata_key(
        mut self,
        extension_index: u8,
        key: Vec<u8>,
        idempotent: u8,
    ) -> Self {
        self.actions.push(MintActionType::RemoveMetadataKey {
            extension_index,
            key,
            idempotent,
        });
        self
    }
}

/// High-level action types for the mint action instruction
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub enum MintActionType {
    CreateSplMint {
        mint_bump: u8,
    },
    MintTo {
        recipients: Vec<MintToRecipient>,
        token_account_version: u8,
    },
    UpdateMintAuthority {
        new_authority: Option<Pubkey>,
    },
    UpdateFreezeAuthority {
        new_authority: Option<Pubkey>,
    },
    MintToCToken {
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

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct MintToRecipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct TokenPool {
    pub pubkey: Pubkey,
    pub bump: u8,
    pub index: u8,
}
// TODO: remove duplicate code
/// Creates a mint action instruction
#[profile]
pub fn create_mint_action_cpi(
    input: MintActionInputs,
    cpi_context: Option<CpiContext>,
    cpi_context_pubkey: Option<Pubkey>,
) -> Result<Instruction> {
    // Convert high-level actions to program-level actions
    let mut program_actions = Vec::new();
    let mint_bump = input.mint_bump.unwrap_or(0u8);
    let create_mint = if input.create_mint {
        Some(CreateMint {
            mint_bump,
            ..Default::default()
        })
    } else {
        None
    };

    // Check for lamports, decompressed status, and mint actions before moving
    let with_lamports = false;
    let spl_mint_initialized = input
        .actions
        .iter()
        .any(|action| matches!(action, MintActionType::CreateSplMint { .. }))
        || input
            .compressed_mint_inputs
            .mint
            .metadata
            .spl_mint_initialized;
    let has_mint_to_actions = input.actions.iter().any(|action| {
        matches!(
            action,
            MintActionType::MintTo { .. } | MintActionType::MintToCToken { .. }
        )
    });
    // Match onchain logic: with_mint_signer = create_mint() | has_CreateSplMint_action
    let with_mint_signer = create_mint.is_some()
        || input
            .actions
            .iter()
            .any(|action| matches!(action, MintActionType::CreateSplMint { .. }));

    // Only require mint to sign when creating a new compressed mint
    let mint_needs_to_sign = create_mint.is_some();

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
                token_account_version,
            } => {
                // TODO: cleanup once lamports are removed.
                let program_recipients: Vec<_> = recipients
                    .into_iter()
                    .map(|r| Recipient {
                        recipient: r.recipient.to_bytes().into(),
                        amount: r.amount,
                    })
                    .collect();

                program_actions.push(Action::MintToCompressed(MintToCompressedAction {
                    token_account_version,
                    recipients: program_recipients,
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
            MintActionType::MintToCToken { account, amount } => {
                use light_ctoken_types::instructions::mint_action::{
                    DecompressedRecipient, MintToCTokenAction,
                };

                // Add account to decompressed accounts list and get its index
                decompressed_accounts.push(account);
                let current_index = decompressed_account_index;
                decompressed_account_index += 1;

                program_actions.push(Action::MintToCToken(MintToCTokenAction {
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
        spl_mint_initialized,
        has_mint_to_actions,
        with_cpi_context: cpi_context_pubkey,
        create_mint: create_mint.is_some(),
        with_mint_signer,
        mint_needs_to_sign,
        ctoken_accounts: decompressed_accounts,
    };

    // Get account metas (before moving compressed_mint_inputs)
    let accounts =
        get_mint_action_instruction_account_metas(meta_config, &input.compressed_mint_inputs);
    msg!("account metas {:?}", accounts);
    let instruction_data = MintActionCompressedInstructionData {
        create_mint,
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
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize, PartialEq)]
pub struct MintActionInputsCpiWrite {
    pub compressed_mint_inputs:
        light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
    pub mint_seed: Option<Pubkey>, // Optional - only when creating mint and when creating SPL mint
    pub mint_bump: Option<u8>,     // Bump seed for creating SPL mint
    pub create_mint: bool,         // Whether we're creating a new mint
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub actions: Vec<MintActionType>,
    //pub input_queue: Option<Pubkey>, // Input queue for existing compressed mint operations
    pub cpi_context: light_ctoken_types::instructions::mint_action::CpiContext,
    pub cpi_context_pubkey: Pubkey,
}

/// Input parameters for creating a new mint in CPI write mode
#[derive(Debug, Clone)]
pub struct CreateMintCpiWriteInputs {
    pub compressed_mint_inputs:
        light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
    pub mint_seed: Pubkey,
    pub mint_bump: u8,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub cpi_context_pubkey: Pubkey,
    pub first_set_context: bool,
    pub address_tree_index: u8,
    pub output_queue_index: u8,
    pub assigned_account_index: u8,
}

/// Input parameters for working with an existing mint in CPI write mode
#[derive(Debug, Clone)]
pub struct WithMintCpiWriteInputs {
    pub compressed_mint_inputs:
        light_ctoken_types::instructions::mint_action::CompressedMintWithContext,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub cpi_context_pubkey: Pubkey,
    pub first_set_context: bool,
    pub state_tree_index: u8,
    pub input_queue_index: u8,
    pub output_queue_index: u8,
    pub assigned_account_index: u8,
}

impl MintActionInputsCpiWrite {
    /// Create a new compressed mint for CPI write (starting point for new mints)
    pub fn new_create_mint(inputs: CreateMintCpiWriteInputs) -> Self {
        let cpi_context = light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false, // For CPI write, we're reading from context
            first_set_context: inputs.first_set_context,
            in_tree_index: inputs.address_tree_index, // For create_mint, this is the address tree
            in_queue_index: 0,                        // Not used for create_mint
            out_queue_index: inputs.output_queue_index,
            token_out_queue_index: 0, // Set when adding MintTo action
            assigned_account_index: inputs.assigned_account_index,
            read_only_address_trees: [0; 4],
            address_tree_pubkey: light_ctoken_types::CMINT_ADDRESS_TREE,
        };

        Self {
            compressed_mint_inputs: inputs.compressed_mint_inputs,
            mint_seed: Some(inputs.mint_seed),
            mint_bump: Some(inputs.mint_bump),
            create_mint: true,
            authority: inputs.authority,
            payer: inputs.payer,
            actions: Vec::new(),
            cpi_context,
            cpi_context_pubkey: inputs.cpi_context_pubkey,
        }
    }

    /// Start with an existing mint for CPI write (starting point for existing mints)
    pub fn new_with_mint(inputs: WithMintCpiWriteInputs) -> Self {
        let cpi_context = light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false, // For CPI write, we're reading from context
            first_set_context: inputs.first_set_context,
            in_tree_index: inputs.state_tree_index,
            in_queue_index: inputs.input_queue_index,
            out_queue_index: inputs.output_queue_index,
            token_out_queue_index: 0, // Set when adding MintTo action
            assigned_account_index: inputs.assigned_account_index,
            ..Default::default()
        };

        Self {
            compressed_mint_inputs: inputs.compressed_mint_inputs,
            mint_seed: None,
            mint_bump: None,
            create_mint: false,
            authority: inputs.authority,
            payer: inputs.payer,
            actions: Vec::new(),
            cpi_context,
            cpi_context_pubkey: inputs.cpi_context_pubkey,
        }
    }

    /// Add MintTo action - mint tokens to compressed token accounts
    /// Returns error if mint is decompressed (no SPL mint modifications in CPI write)
    pub fn add_mint_to(
        mut self,
        recipients: Vec<MintToRecipient>,
        token_account_version: u8,
        token_out_queue_index: u8, // Index for token output queue
    ) -> Result<Self> {
        // Cannot mint if the mint is decompressed
        // In CPI write, we cannot modify SPL mint supply
        if self
            .compressed_mint_inputs
            .mint
            .metadata
            .spl_mint_initialized
        {
            return Err(TokenSdkError::CannotMintWithDecompressedInCpiWrite);
        }

        // Set token_out_queue_index for the MintTo action
        self.cpi_context.token_out_queue_index = token_out_queue_index;

        self.actions.push(MintActionType::MintTo {
            recipients,
            token_account_version,
        });
        Ok(self)
    }

    /// Add UpdateMintAuthority action
    pub fn add_update_mint_authority(mut self, new_authority: Option<Pubkey>) -> Self {
        self.actions
            .push(MintActionType::UpdateMintAuthority { new_authority });
        self
    }

    /// Add UpdateFreezeAuthority action
    pub fn add_update_freeze_authority(mut self, new_authority: Option<Pubkey>) -> Self {
        self.actions
            .push(MintActionType::UpdateFreezeAuthority { new_authority });
        self
    }

    /// Add UpdateMetadataField action
    pub fn add_update_metadata_field(
        mut self,
        extension_index: u8,
        field_type: u8,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Self {
        self.actions.push(MintActionType::UpdateMetadataField {
            extension_index,
            field_type,
            key,
            value,
        });
        self
    }

    /// Add UpdateMetadataAuthority action
    pub fn add_update_metadata_authority(
        mut self,
        extension_index: u8,
        new_authority: Pubkey,
    ) -> Self {
        self.actions.push(MintActionType::UpdateMetadataAuthority {
            extension_index,
            new_authority,
        });
        self
    }

    /// Add RemoveMetadataKey action
    pub fn add_remove_metadata_key(
        mut self,
        extension_index: u8,
        key: Vec<u8>,
        idempotent: u8,
    ) -> Self {
        self.actions.push(MintActionType::RemoveMetadataKey {
            extension_index,
            key,
            idempotent,
        });
        self
    }
}

/// Creates a mint action CPI write instruction (for use in CPI context)
pub fn mint_action_cpi_write(input: MintActionInputsCpiWrite) -> Result<Instruction> {
    use light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData;
    if input
        .compressed_mint_inputs
        .mint
        .metadata
        .spl_mint_initialized
        || input
            .actions
            .iter()
            .any(|action| matches!(action, MintActionType::CreateSplMint { .. }))
    {
        return Err(TokenSdkError::CannotMintWithDecompressedInCpiWrite);
    }
    // Validate CPI context
    if !input.cpi_context.first_set_context && !input.cpi_context.set_context {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Convert high-level actions to program-level actions
    let mut program_actions = Vec::new();
    let mint_bump = input.mint_bump.unwrap_or(0u8);
    let create_mint = if input.create_mint {
        Some(CreateMint {
            mint_bump,
            ..Default::default()
        })
    } else {
        None
    };

    let with_mint_signer = create_mint.is_some();

    // Only require mint to sign when creating a new compressed mint
    let mint_needs_to_sign = create_mint.is_some();

    for action in input.actions {
        match action {
            MintActionType::MintTo {
                recipients,
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
                    light_ctoken_types::instructions::mint_action::Action::MintToCompressed(
                        light_ctoken_types::instructions::mint_action::MintToCompressedAction {
                            token_account_version,
                            recipients: program_recipients,
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
            _ => return Err(TokenSdkError::CannotMintWithDecompressedInCpiWrite),
        }
    }

    let instruction_data = MintActionCompressedInstructionData {
        create_mint,
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
