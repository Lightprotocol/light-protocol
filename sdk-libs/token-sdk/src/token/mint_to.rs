use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, traits::LightInstructionData,
};
use light_token_interface::instructions::mint_action::{
    CompressedMintWithContext, CpiContext, MintToTokenAction,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::compressed_token::mint_action::{
    get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
    MintActionMetaConfigCpiWrite,
};
// TODO: move to compressed_token.
/// Parameters for minting tokens to a ctoken account.
#[derive(Debug, Clone)]
pub struct MintToTokenParams {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_to_actions: Vec<MintToTokenAction>,
    pub mint_authority: Pubkey,
    pub proof: ValidityProof,
}

impl MintToTokenParams {
    pub fn new(
        compressed_mint_inputs: CompressedMintWithContext,
        amount: u64,
        mint_authority: Pubkey,
        proof: ValidityProof,
    ) -> Self {
        Self {
            compressed_mint_inputs,
            mint_to_actions: vec![MintToTokenAction {
                account_index: 0, // TODO: make dynamic
                amount,
            }],
            mint_authority,
            proof,
        }
    }

    pub fn add_mint_to_action(mut self, account_index: u8, amount: u64) -> Self {
        self.mint_to_actions.push(MintToTokenAction {
            account_index,
            amount,
        });
        self
    }
}

/// # Create a mint to ctoken instruction:
/// ```rust,no_run
/// # use solana_pubkey::Pubkey;
/// use light_token_sdk::token::{MintToToken, MintToTokenParams, CompressedMintWithContext};
/// use light_token_sdk::ValidityProof;
/// # let compressed_mint_with_context: CompressedMintWithContext = todo!();
/// # let validity_proof: ValidityProof = todo!();
/// # let mint_authority = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let state_tree_pubkey = Pubkey::new_unique();
/// # let input_queue = Pubkey::new_unique();
/// # let output_queue = Pubkey::new_unique();
/// # let ctoken_account = Pubkey::new_unique();
///
/// let params = MintToTokenParams::new(
///     compressed_mint_with_context, // from rpc
///     1000, // amount
///     mint_authority,
///     validity_proof, // from rpc
/// );
/// let instruction = MintToToken::new(
///     params,
///     payer,
///     state_tree_pubkey,
///     input_queue,
///     output_queue,
///     vec![ctoken_account],
/// ).instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct MintToToken {
    pub payer: Pubkey,
    pub state_tree_pubkey: Pubkey,
    pub input_queue: Pubkey,
    pub output_queue: Pubkey,
    pub ctoken_accounts: Vec<Pubkey>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub params: MintToTokenParams,
}

impl MintToToken {
    pub fn new(
        params: MintToTokenParams,
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
            light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new(
                self.params.compressed_mint_inputs.clone(),
                self.params.proof.0,
            );

        // Add all mint_to_ctoken actions
        for action in self.params.mint_to_actions {
            instruction_data = instruction_data.with_mint_to_token(action);
        }

        if let Some(ctx) = self.cpi_context {
            instruction_data = instruction_data.with_cpi_context(ctx);
        }

        let meta_config = if let Some(cpi_context_pubkey) = self.cpi_context_pubkey {
            MintActionMetaConfig::new_cpi_context(
                &instruction_data,
                self.payer,
                self.params.mint_authority,
                cpi_context_pubkey,
            )?
        } else {
            MintActionMetaConfig::new(
                self.payer,
                self.params.mint_authority,
                self.state_tree_pubkey,
                self.input_queue,
                self.output_queue,
            )
            .with_ctoken_accounts(self.ctoken_accounts)
        };

        let account_metas = meta_config.to_account_metas();

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}

// ============================================================================
// Params Struct: MintToTokenCpiWriteParams
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToTokenCpiWriteParams {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub mint_to_actions: Vec<MintToTokenAction>,
    pub mint_authority: Pubkey,
    pub cpi_context: CpiContext,
}

impl MintToTokenCpiWriteParams {
    pub fn new(
        compressed_mint_inputs: CompressedMintWithContext,
        amount: u64,
        mint_authority: Pubkey,
        cpi_context: CpiContext,
    ) -> Self {
        Self {
            compressed_mint_inputs,
            mint_to_actions: vec![MintToTokenAction {
                account_index: 0, // TODO: make dynamic
                amount,
            }],
            mint_authority,
            cpi_context,
        }
    }

    pub fn add_mint_to_action(mut self, account_index: u8, amount: u64) -> Self {
        self.mint_to_actions.push(MintToTokenAction {
            account_index,
            amount,
        });
        self
    }
}

// ============================================================================
// Builder Struct: MintToTokenCpiWrite
// ============================================================================

#[derive(Debug, Clone)]
pub struct MintToTokenCpiWrite {
    pub payer: Pubkey,
    pub cpi_context_pubkey: Pubkey,
    pub ctoken_accounts: Vec<Pubkey>,
    pub params: MintToTokenCpiWriteParams,
}

impl MintToTokenCpiWrite {
    pub fn new(
        params: MintToTokenCpiWriteParams,
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
            light_token_interface::instructions::mint_action::MintActionCompressedInstructionData::new(
                self.params.compressed_mint_inputs.clone(),
                None, // No proof for CPI write
            );

        // Add all mint_to_ctoken actions
        for action in self.params.mint_to_actions {
            instruction_data = instruction_data.with_mint_to_token(action);
        }

        instruction_data = instruction_data.with_cpi_context(self.params.cpi_context);

        let meta_config = MintActionMetaConfigCpiWrite {
            fee_payer: self.payer,
            mint_signer: None,
            authority: self.params.mint_authority,
            cpi_context: self.cpi_context_pubkey,
        };

        let account_metas = get_mint_action_instruction_account_metas_cpi_write(meta_config);

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}

// ============================================================================
// AccountInfos Struct: MintToTokenCpi (for CPI usage)
// ============================================================================

/// # Mint to ctoken account via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::{MintToTokenCpi, MintToTokenParams, SystemAccountInfos};
/// # use solana_account_info::AccountInfo;
/// # let authority: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let state_tree: AccountInfo = todo!();
/// # let input_queue: AccountInfo = todo!();
/// # let output_queue: AccountInfo = todo!();
/// # let ctoken_accounts: Vec<AccountInfo> = todo!();
/// # let system_accounts: SystemAccountInfos = todo!();
/// # let params: MintToTokenParams = todo!();
/// MintToTokenCpi {
///     authority,
///     payer,
///     state_tree,
///     input_queue,
///     output_queue,
///     ctoken_accounts,
///     system_accounts,
///     cpi_context: None,
///     cpi_context_account: None,
///     params,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct MintToTokenCpi<'info> {
    /// The authority for the mint operation (mint_authority).
    pub authority: AccountInfo<'info>,
    /// The fee payer for the transaction.
    pub payer: AccountInfo<'info>,
    pub state_tree: AccountInfo<'info>,
    pub input_queue: AccountInfo<'info>,
    pub output_queue: AccountInfo<'info>,
    pub ctoken_accounts: Vec<AccountInfo<'info>>,
    pub system_accounts: crate::token::SystemAccountInfos<'info>,
    pub cpi_context: Option<CpiContext>,
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub params: MintToTokenParams,
}

impl<'info> MintToTokenCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintToToken::try_from(self)?.instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from to_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.authority, // authority
            self.payer,     // fee_payer
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        account_infos.push(self.output_queue);
        account_infos.push(self.state_tree);
        account_infos.push(self.input_queue);
        account_infos.extend(self.ctoken_accounts);

        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;

        // Account order must match the instruction's account metas order (from to_account_metas)
        let mut account_infos = vec![
            self.system_accounts.light_system_program,
            self.authority, // authority
            self.payer,     // fee_payer
            self.system_accounts.cpi_authority_pda,
            self.system_accounts.registered_program_pda,
            self.system_accounts.account_compression_authority,
            self.system_accounts.account_compression_program,
            self.system_accounts.system_program,
        ];

        if let Some(cpi_context_account) = self.cpi_context_account {
            account_infos.push(cpi_context_account);
        }

        account_infos.push(self.output_queue);
        account_infos.push(self.state_tree);
        account_infos.push(self.input_queue);
        account_infos.extend(self.ctoken_accounts);

        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> TryFrom<&MintToTokenCpi<'info>> for MintToToken {
    type Error = ProgramError;

    fn try_from(account_infos: &MintToTokenCpi<'info>) -> Result<Self, Self::Error> {
        if account_infos.params.mint_authority != *account_infos.authority.key {
            solana_msg::msg!(
                "MintToTokenCpi: params.mint_authority ({}) does not match authority account ({})",
                account_infos.params.mint_authority,
                account_infos.authority.key
            );
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(Self {
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
        })
    }
}

// ============================================================================
// AccountInfos Struct: MintToTokenCpiWriteCpi
// ============================================================================

pub struct MintToTokenCpiWriteCpi<'info> {
    pub payer: AccountInfo<'info>,
    pub cpi_context_account: AccountInfo<'info>,
    pub ctoken_accounts: Vec<AccountInfo<'info>>,
    pub params: MintToTokenCpiWriteParams,
}

impl<'info> MintToTokenCpiWriteCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintToTokenCpiWrite::from(self).instruction()
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let mut account_infos = vec![self.payer, self.cpi_context_account];
        account_infos.extend(self.ctoken_accounts);
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&MintToTokenCpiWriteCpi<'info>> for MintToTokenCpiWrite {
    fn from(account_infos: &MintToTokenCpiWriteCpi<'info>) -> Self {
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
