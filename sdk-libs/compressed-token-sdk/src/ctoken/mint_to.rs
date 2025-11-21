use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, traits::LightInstructionData,
};
use light_ctoken_types::instructions::mint_action::{
    CompressedMintWithContext, CpiContext, MintToCTokenAction,
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
// Params Struct: MintToCTokenParams
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToCTokenParams {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_to_actions: Vec<MintToCTokenAction>,
    pub mint_authority: Pubkey,
    pub proof: ValidityProof,
}

impl MintToCTokenParams {
    pub fn new(
        compressed_mint_inputs: CompressedMintWithContext,
        amount: u64,
        mint_authority: Pubkey,
        proof: ValidityProof,
    ) -> Self {
        Self {
            compressed_mint_inputs,
            mint_to_actions: vec![MintToCTokenAction {
                account_index: 0, // TODO: make dynamic
                amount,
            }],
            mint_authority,
            proof,
        }
    }

    pub fn add_mint_to_action(mut self, account_index: u8, amount: u64) -> Self {
        self.mint_to_actions.push(MintToCTokenAction {
            account_index,
            amount,
        });
        self
    }
}

// ============================================================================
// Builder Struct: MintToCToken
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToCToken {
    pub payer: Pubkey,
    pub state_tree_pubkey: Pubkey,
    pub input_queue: Pubkey,
    pub output_queue: Pubkey,
    pub ctoken_accounts: Vec<Pubkey>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub params: MintToCTokenParams,
}

impl MintToCToken {
    pub fn new(
        params: MintToCTokenParams,
        payer: Pubkey,
        state_tree_pubkey: Pubkey,
        input_queue: Pubkey,
        output_queue: Pubkey,
        ctoken_accounts: Vec<Pubkey>,
    ) -> Self {
        Self {
            payer,
            state_tree_pubkey,
            input_queue,
            output_queue,
            ctoken_accounts,
            cpi_context: None,
            cpi_context_pubkey: None,
            params,
        }
    }

    pub fn with_cpi_context(mut self, cpi_context: CpiContext, cpi_context_pubkey: Pubkey) -> Self {
        self.cpi_context = Some(cpi_context);
        self.cpi_context_pubkey = Some(cpi_context_pubkey);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Build instruction data with mint_to_ctoken actions
        let mut instruction_data =
            light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new(
                self.params.compressed_mint_inputs.clone(),
                self.params.proof.0,
            );

        // Add all mint_to_ctoken actions
        for action in self.params.mint_to_actions {
            instruction_data = instruction_data.with_mint_to_ctoken(action);
        }

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
            MintActionMetaConfig::new(
                &instruction_data,
                self.params.mint_authority,
                self.payer,
                self.state_tree_pubkey,
                self.input_queue,
                self.output_queue,
            )?
            .with_ctoken_accounts(self.ctoken_accounts)
        };

        let account_metas = get_mint_action_instruction_account_metas(
            meta_config,
            &self.params.compressed_mint_inputs,
        );

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
// Params Struct: MintToCTokenCpiWriteParams
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToCTokenCpiWriteParams {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_to_actions: Vec<MintToCTokenAction>,
    pub mint_authority: Pubkey,
    pub cpi_context: CpiContext,
}

impl MintToCTokenCpiWriteParams {
    pub fn new(
        compressed_mint_inputs: CompressedMintWithContext,
        amount: u64,
        mint_authority: Pubkey,
        cpi_context: CpiContext,
    ) -> Self {
        Self {
            compressed_mint_inputs,
            mint_to_actions: vec![MintToCTokenAction {
                account_index: 0, // TODO: make dynamic
                amount,
            }],
            mint_authority,
            cpi_context,
        }
    }

    pub fn add_mint_to_action(mut self, account_index: u8, amount: u64) -> Self {
        self.mint_to_actions.push(MintToCTokenAction {
            account_index,
            amount,
        });
        self
    }
}

// ============================================================================
// Builder Struct: MintToCTokenCpiWrite
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToCTokenCpiWrite {
    pub payer: Pubkey,
    pub cpi_context_pubkey: Pubkey,
    pub ctoken_accounts: Vec<Pubkey>,
    pub params: MintToCTokenCpiWriteParams,
}

impl MintToCTokenCpiWrite {
    pub fn new(
        params: MintToCTokenCpiWriteParams,
        payer: Pubkey,
        cpi_context_pubkey: Pubkey,
        ctoken_accounts: Vec<Pubkey>,
    ) -> Self {
        Self {
            payer,
            cpi_context_pubkey,
            ctoken_accounts,
            params,
        }
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        if !self.params.cpi_context.first_set_context && !self.params.cpi_context.set_context {
            solana_msg::msg!(
                "Invalid CPI context first cpi set or set context must be true {:?}",
                self.params.cpi_context
            );
            return Err(ProgramError::InvalidAccountData);
        }

        // Build instruction data with mint_to_ctoken actions
        let mut instruction_data =
            light_ctoken_types::instructions::mint_action::MintActionCompressedInstructionData::new(
                self.params.compressed_mint_inputs.clone(),
                None, // No proof for CPI write
            );

        // Add all mint_to_ctoken actions
        for action in self.params.mint_to_actions {
            instruction_data = instruction_data.with_mint_to_ctoken(action);
        }

        instruction_data = instruction_data.with_cpi_context(self.params.cpi_context);

        let meta_config = MintActionMetaConfigCpiWrite {
            fee_payer: self.payer,
            mint_signer: None,
            authority: self.params.mint_authority,
            cpi_context: self.cpi_context_pubkey,
            mint_needs_to_sign: false,
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
// AccountInfos Struct: MintToCTokenInfos (for CPI usage)
// ============================================================================

pub struct MintToCTokenInfos<'info> {
    pub payer: AccountInfo<'info>,
    pub state_tree: AccountInfo<'info>,
    pub input_queue: AccountInfo<'info>,
    pub output_queue: AccountInfo<'info>,
    pub ctoken_accounts: Vec<AccountInfo<'info>>,
    pub system_accounts: crate::ctoken::SystemAccountInfos<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub params: MintToCTokenParams,
}

impl<'info> MintToCTokenInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintToCToken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from get_mint_action_instruction_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.payer.clone(), // authority
            self.payer,         // fee_payer
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
            self.state_tree,
            self.input_queue,
            self.output_queue,
        ];

        account_infos.extend(self.ctoken_accounts);

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from get_mint_action_instruction_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.payer.clone(), // authority
            self.payer,         // fee_payer
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
            self.state_tree,
            self.input_queue,
            self.output_queue,
        ];

        account_infos.extend(self.ctoken_accounts);

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&MintToCTokenInfos<'info>> for MintToCToken {
    fn from(account_infos: &MintToCTokenInfos<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            state_tree_pubkey: *account_infos.state_tree.key,
            input_queue: *account_infos.input_queue.key,
            output_queue: *account_infos.output_queue.key,
            ctoken_accounts: account_infos
                .ctoken_accounts
                .iter()
                .map(|acc| *acc.key)
                .collect(),
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
// AccountInfos Struct: MintToCTokenCpiWriteInfos
// ============================================================================

pub struct MintToCTokenCpiWriteInfos<'info> {
    pub payer: AccountInfo<'info>,
    pub cpi_context_account: AccountInfo<'info>,
    pub ctoken_accounts: Vec<AccountInfo<'info>>,
    pub params: MintToCTokenCpiWriteParams,
}

impl<'info> MintToCTokenCpiWriteInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintToCTokenCpiWrite::from(self).instruction()
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let mut account_infos = vec![self.payer, self.cpi_context_account];
        account_infos.extend(self.ctoken_accounts);
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&MintToCTokenCpiWriteInfos<'info>> for MintToCTokenCpiWrite {
    fn from(account_infos: &MintToCTokenCpiWriteInfos<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            cpi_context_pubkey: *account_infos.cpi_context_account.key,
            ctoken_accounts: account_infos
                .ctoken_accounts
                .iter()
                .map(|acc| *acc.key)
                .collect(),
            params: account_infos.params.clone(),
        }
    }
}
