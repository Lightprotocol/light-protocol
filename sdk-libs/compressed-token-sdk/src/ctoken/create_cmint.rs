use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
};
use light_ctoken_types::{
    instructions::{
        extensions::ExtensionInstructionData,
        mint_action::{CompressedMintInstructionData, CompressedMintWithContext, CpiContext},
    },
    COMPRESSED_MINT_SEED,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compressed_token::mint_action::{
    get_mint_action_instruction_account_metas, get_mint_action_instruction_account_metas_cpi_write,
    MintActionMetaConfig, MintActionMetaConfigCpiWrite,
};

// ============================================================================
// Params Struct: CreateCMintParams
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateCMintParams {
    pub decimals: u8,
    pub version: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

// ============================================================================
// Builder Struct: CreateCMint
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateCMint {
    pub mint_signer: Pubkey,
    pub payer: Pubkey,
    pub address_tree_pubkey: Pubkey,
    pub output_queue: Pubkey,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub params: CreateCMintParams,
}

impl CreateCMint {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decimals: u8,
        mint_authority: Pubkey,
        mint_signer: Pubkey,
        payer: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
        proof: CompressedProof,
        address_merkle_tree_root_index: u16,
    ) -> Self {
        let compression_address =
            derive_compressed_mint_address(&mint_signer, &address_tree_pubkey);
        let mint = find_spl_mint_address(&mint_signer).0;

        Self {
            mint_signer,
            payer,
            address_tree_pubkey,
            output_queue,
            cpi_context: None,
            cpi_context_pubkey: None,
            params: CreateCMintParams {
                decimals,
                mint_authority,
                freeze_authority: None,
                proof,
                address_merkle_tree_root_index,
                extensions: None,
                version: 3,
                compression_address,
                mint,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_address(
        decimals: u8,
        mint_authority: Pubkey,
        mint_signer: Pubkey,
        payer: Pubkey,
        address_tree_pubkey: Pubkey,
        output_queue: Pubkey,
        proof: CompressedProof,
        address_merkle_tree_root_index: u16,
        compression_address: [u8; 32],
        mint: Pubkey,
    ) -> Self {
        Self {
            mint_signer,
            payer,
            address_tree_pubkey,
            output_queue,
            cpi_context: None,
            cpi_context_pubkey: None,
            params: CreateCMintParams {
                decimals,
                mint_authority,
                freeze_authority: None,
                proof,
                address_merkle_tree_root_index,
                extensions: None,
                version: 3,
                compression_address,
                mint,
            },
        }
    }

    pub fn with_freeze_authority(mut self, freeze_authority: Pubkey) -> Self {
        self.params.freeze_authority = Some(freeze_authority);
        self
    }

    pub fn with_extensions(mut self, extensions: Vec<ExtensionInstructionData>) -> Self {
        self.params.extensions = Some(extensions);
        self
    }

    pub fn with_cpi_context(mut self, cpi_context: CpiContext, cpi_context_pubkey: Pubkey) -> Self {
        self.cpi_context = Some(cpi_context);
        self.cpi_context_pubkey = Some(cpi_context_pubkey);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let compression_address = self.params.compression_address;

        let compressed_mint_instruction_data = CompressedMintInstructionData {
            supply: 0,
            decimals: self.params.decimals,
            metadata: light_ctoken_types::state::CompressedMintMetadata {
                version: self.params.version,
                mint: self.params.mint.to_bytes().into(),
                spl_mint_initialized: false,
            },
            mint_authority: Some(self.params.mint_authority.to_bytes().into()),
            freeze_authority: self
                .params
                .freeze_authority
                .map(|auth| auth.to_bytes().into()),
            extensions: self.params.extensions,
        };

        let compressed_mint_with_context = CompressedMintWithContext {
            address: compression_address,
            mint: compressed_mint_instruction_data.clone(),
            leaf_index: 0,
            prove_by_index: false,
            root_index: self.params.address_merkle_tree_root_index,
        };

        let mut instruction_data =
            light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new_mint(
                compression_address,
                self.params.address_merkle_tree_root_index,
                self.params.proof,
                compressed_mint_instruction_data,
            );

        if let Some(ctx) = self.cpi_context {
            instruction_data = instruction_data.with_cpi_context(ctx);
        }

        let meta_config = if let Some(cpi_context_pubkey) = self.cpi_context_pubkey {
            MintActionMetaConfig::new_cpi_context(
                &instruction_data,
                self.params.mint_authority,
                self.payer,
                cpi_context_pubkey,
            )?
        } else {
            MintActionMetaConfig::new_create_mint(
                &instruction_data,
                self.params.mint_authority,
                self.mint_signer,
                self.payer,
                self.address_tree_pubkey,
                self.output_queue,
            )?
        };

        let account_metas =
            get_mint_action_instruction_account_metas(meta_config, &compressed_mint_with_context);

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}

// ============================================================================
// Params Struct: CreateCMintCpiWriteParams
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateCMintCpiWriteParams {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub address_merkle_tree_root_index: u16,
    pub compression_address: [u8; 32],
    pub cpi_context: CpiContext,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    pub version: u8,
}

// ============================================================================
// Builder Struct: CreateCompressedMintCpiWrite
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateCompressedMintCpiWrite {
    pub mint_signer: Pubkey,
    pub payer: Pubkey,
    pub cpi_context_pubkey: Pubkey,
    pub params: CreateCMintCpiWriteParams,
}

impl CreateCompressedMintCpiWrite {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        decimals: u8,
        mint_authority: Pubkey,
        mint_signer: Pubkey,
        payer: Pubkey,
        compression_address: [u8; 32],
        cpi_context: CpiContext,
        cpi_context_pubkey: Pubkey,
        address_merkle_tree_root_index: u16,
    ) -> Self {
        Self {
            mint_signer,
            payer,
            cpi_context_pubkey,
            params: CreateCMintCpiWriteParams {
                decimals,
                mint_authority,
                freeze_authority: None,
                address_merkle_tree_root_index,
                compression_address,
                cpi_context,
                extensions: None,
                version: 3,
            },
        }
    }

    pub fn with_freeze_authority(mut self, freeze_authority: Pubkey) -> Self {
        self.params.freeze_authority = Some(freeze_authority);
        self
    }

    pub fn with_extensions(mut self, extensions: Vec<ExtensionInstructionData>) -> Self {
        self.params.extensions = Some(extensions);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        if !self.params.cpi_context.first_set_context && !self.params.cpi_context.set_context {
            solana_msg::msg!(
                "Invalid CPI context first cpi set or set context must be true {:?}",
                self.params.cpi_context
            );
            return Err(ProgramError::InvalidAccountData);
        }

        let compressed_mint_instruction_data = CompressedMintInstructionData {
            supply: 0,
            decimals: self.params.decimals,
            metadata: light_ctoken_types::state::CompressedMintMetadata {
                version: self.params.version,
                mint: find_spl_mint_address(&self.mint_signer).0.to_bytes().into(),
                spl_mint_initialized: false,
            },
            mint_authority: Some(self.params.mint_authority.to_bytes().into()),
            freeze_authority: self
                .params
                .freeze_authority
                .map(|auth| auth.to_bytes().into()),
            extensions: self.params.extensions,
        };

        let instruction_data =
            light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new_mint_write_to_cpi_context(
                self.params.compression_address,
                self.params.address_merkle_tree_root_index,
                compressed_mint_instruction_data,
                self.params.cpi_context,
            );

        let meta_config = MintActionMetaConfigCpiWrite {
            fee_payer: self.payer,
            mint_signer: Some(self.mint_signer),
            authority: self.params.mint_authority,
            cpi_context: self.cpi_context_pubkey,
            mint_needs_to_sign: true,
        };

        let account_metas = get_mint_action_instruction_account_metas_cpi_write(meta_config);

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}

// ============================================================================
// AccountInfos Struct: CreateCompressedMintInfos (for CPI usage)
// ============================================================================

pub struct CreateCompressedMintInfos<'info> {
    pub mint_signer: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub address_tree: AccountInfo<'info>,
    pub output_queue: AccountInfo<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub params: CreateCMintParams,
}

impl<'info> CreateCompressedMintInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateCMint::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        let mut account_infos = vec![
            self.mint_signer,
            self.payer,
            self.address_tree,
            self.output_queue,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        let mut account_infos = vec![
            self.mint_signer,
            self.payer,
            self.address_tree,
            self.output_queue,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&CreateCompressedMintInfos<'info>> for CreateCMint {
    fn from(account_infos: &CreateCompressedMintInfos<'info>) -> Self {
        Self {
            mint_signer: *account_infos.mint_signer.key,
            payer: *account_infos.payer.key,
            address_tree_pubkey: *account_infos.address_tree.key,
            output_queue: *account_infos.output_queue.key,
            cpi_context: account_infos.cpi_context.clone(),
            cpi_context_pubkey: account_infos
                .cpi_context_account
                .as_ref()
                .map(|acc| *acc.key),
            params: account_infos.params.clone(),
        }
    }
}

// ============================================================================
// AccountInfos Struct: CreateCompressedMintCpiWriteInfos
// ============================================================================

pub struct CreateCompressedMintCpiWriteInfos<'info> {
    pub mint_signer: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub cpi_context_account: AccountInfo<'info>,
    pub params: CreateCMintCpiWriteParams,
}

impl<'info> CreateCompressedMintCpiWriteInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateCompressedMintCpiWrite::from(self).instruction()
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [self.mint_signer, self.payer, self.cpi_context_account];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&CreateCompressedMintCpiWriteInfos<'info>> for CreateCompressedMintCpiWrite {
    fn from(account_infos: &CreateCompressedMintCpiWriteInfos<'info>) -> Self {
        Self {
            mint_signer: *account_infos.mint_signer.key,
            payer: *account_infos.payer.key,
            cpi_context_pubkey: *account_infos.cpi_context_account.key,
            params: account_infos.params.clone(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Derives the compressed mint address from the mint seed and address tree
pub fn derive_compressed_mint_address(
    mint_seed: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &find_spl_mint_address(mint_seed).0.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
    )
}

/// Derives the compressed mint address from an SPL mint address
pub fn derive_cmint_from_spl_mint(mint: &Pubkey, address_tree_pubkey: &Pubkey) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &mint.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID,
    )
}

/// Finds the SPL mint address from a mint seed
pub fn find_spl_mint_address(mint_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}
