//! Create multiple Light Mints and decompress all to Solana Mint accounts.
//!
//! This module provides functionality for batch creating Light Mints with
//! optimal CPI batching. When creating multiple mints, it uses the CPI context
//! pattern to minimize transaction overhead.
//!
//! # Flow
//!
//! - N=1: Single CPI (create + decompress)
//! - N>1: 2N-1 CPIs (N-1 writes + 1 execute with decompress + N-1 decompress)

use light_batched_merkle_tree::queue_ref::BatchedQueueRef;
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_compressed_token_sdk::compressed_token::mint_action::{
    get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
    MintActionMetaConfigCpiWrite,
};
use light_token_interface::{
    instructions::{
        extensions::{ExtensionInstructionData, TokenMetadataInstructionData},
        mint_action::{
            Action, CpiContext, CreateMint, DecompressMintAction,
            MintActionCompressedInstructionData, MintInstructionData,
        },
    },
    state::MintMetadata,
    LIGHT_TOKEN_PROGRAM_ID,
};
use solana_account_info::AccountInfo;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::SystemAccountInfos;

/// Default rent payment epochs (~24 hours)
pub const DEFAULT_RENT_PAYMENT: u8 = 16;
/// Default lamports for write operations (~3 hours per write)
pub const DEFAULT_WRITE_TOP_UP: u32 = 766;

/// Parameters for a single mint within a batch creation.
///
/// Does not include proof since proof is shared across all mints in the batch.
/// `mint` and `compression_address` are derived internally from `mint_seed_pubkey`.
#[derive(Debug, Clone)]
pub struct SingleMintParams<'a> {
    pub decimals: u8,
    pub mint_authority: Pubkey,
    /// Optional mint bump. If `None`, derived from `find_mint_address(mint_seed_pubkey)`.
    pub mint_bump: Option<u8>,
    pub freeze_authority: Option<Pubkey>,
    /// Mint seed pubkey (signer) for this mint. Used to derive `mint` PDA and `compression_address`.
    pub mint_seed_pubkey: Pubkey,
    /// Optional authority seeds for PDA signing
    pub authority_seeds: Option<&'a [&'a [u8]]>,
    /// Optional mint signer seeds for PDA signing
    pub mint_signer_seeds: Option<&'a [&'a [u8]]>,
    /// Optional token metadata for the mint (reference to avoid stack overflow)
    pub token_metadata: Option<&'a TokenMetadataInstructionData>,
}

/// Parameters for creating one or more Light Mints with decompression.
///
/// Creates N Light Mints and decompresses all to Solana Mint accounts.
/// Uses CPI context pattern when N > 1 for efficiency.
#[derive(Debug, Clone)]
pub struct CreateMintsParams<'a> {
    /// Parameters for each mint to create
    pub mints: &'a [SingleMintParams<'a>],
    /// Single proof covering all new addresses
    pub proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof,
    /// Root index for the address merkle tree (shared by all mints in batch).
    pub address_merkle_tree_root_index: u16,
    /// Rent payment in epochs for the Mint account (must be 0 or >= 2).
    /// Default: 16 (~24 hours)
    pub rent_payment: u8,
    /// Lamports allocated for future write operations.
    /// Default: 766 (~3 hours per write)
    pub write_top_up: u32,
    /// Offset for assigned_account_index when sharing CPI context with other accounts.
    /// When creating mints alongside PDAs, this offset should be set to the number of
    /// PDAs already written to the CPI context.
    /// Default: 0 (no offset)
    pub cpi_context_offset: u8,
    /// Index of the output queue in tree accounts.
    /// Default: 0
    pub output_queue_index: u8,
    /// Index of the address merkle tree in tree accounts.
    /// Default: 1
    pub address_tree_index: u8,
    /// Index of the state merkle tree in tree accounts.
    /// Required for decompress operations (discriminator validation).
    /// Default: 2
    pub state_tree_index: u8,
}

impl<'a> CreateMintsParams<'a> {
    #[inline(never)]
    pub fn new(
        mints: &'a [SingleMintParams<'a>],
        proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof,
        address_merkle_tree_root_index: u16,
    ) -> Self {
        Self {
            mints,
            proof,
            address_merkle_tree_root_index,
            rent_payment: DEFAULT_RENT_PAYMENT,
            write_top_up: DEFAULT_WRITE_TOP_UP,
            cpi_context_offset: 0,
            output_queue_index: 0,
            address_tree_index: 1,
            state_tree_index: 2,
        }
    }

    pub fn with_rent_payment(mut self, rent_payment: u8) -> Self {
        self.rent_payment = rent_payment;
        self
    }

    pub fn with_write_top_up(mut self, write_top_up: u32) -> Self {
        self.write_top_up = write_top_up;
        self
    }

    /// Set offset for assigned_account_index when sharing CPI context.
    ///
    /// Use this when creating mints alongside PDAs. The offset should be
    /// the number of accounts already written to the CPI context.
    pub fn with_cpi_context_offset(mut self, offset: u8) -> Self {
        self.cpi_context_offset = offset;
        self
    }

    /// Set the output queue index in tree accounts.
    pub fn with_output_queue_index(mut self, index: u8) -> Self {
        self.output_queue_index = index;
        self
    }

    /// Set the address merkle tree index in tree accounts.
    pub fn with_address_tree_index(mut self, index: u8) -> Self {
        self.address_tree_index = index;
        self
    }

    /// Set the state merkle tree index in tree accounts.
    /// Required for decompress operations (discriminator validation).
    pub fn with_state_tree_index(mut self, index: u8) -> Self {
        self.state_tree_index = index;
        self
    }
}

/// CPI struct for on-chain programs to create multiple mints.
///
/// Uses named account fields for clarity and safety - no manual index calculations.
///
/// # Example
///
/// ```rust,ignore
/// use light_token::instruction::{CreateMintsCpi, CreateMintsParams, SingleMintParams, SystemAccountInfos};
///
/// let params = CreateMintsParams::new(vec![mint_params_1, mint_params_2], proof);
///
/// CreateMintsCpi {
///     mint_seeds: vec![mint_signer1.clone(), mint_signer2.clone()],
///     payer: payer.clone(),
///     address_tree: address_tree.clone(),
///     output_queue: output_queue.clone(),
///     compressible_config: config.clone(),
///     mints: vec![mint_pda1.clone(), mint_pda2.clone()],
///     rent_sponsor: rent_sponsor.clone(),
///     system_accounts: SystemAccountInfos { ... },
///     cpi_context_account: cpi_context.clone(),
///     params,
/// }.invoke()?;
/// ```
pub struct CreateMintsCpi<'a, 'info> {
    /// Mint seed accounts (signers) - one per mint
    pub mint_seed_accounts: &'a [AccountInfo<'info>],
    /// Fee payer (also used as authority)
    pub payer: AccountInfo<'info>,
    /// Address tree for new mint addresses
    pub address_tree: AccountInfo<'info>,
    /// Output queue for compressed accounts
    pub output_queue: AccountInfo<'info>,
    /// State merkle tree (required for decompress discriminator validation)
    pub state_merkle_tree: AccountInfo<'info>,
    /// CompressibleConfig account
    pub compressible_config: AccountInfo<'info>,
    /// Mint PDA accounts (writable) - one per mint
    pub mints: &'a [AccountInfo<'info>],
    /// Rent sponsor PDA
    pub rent_sponsor: AccountInfo<'info>,
    /// Standard Light Protocol system accounts
    pub system_accounts: SystemAccountInfos<'info>,
    /// CPI context account
    pub cpi_context_account: AccountInfo<'info>,
    /// Parameters
    pub params: CreateMintsParams<'a>,
}

impl<'a, 'info> CreateMintsCpi<'a, 'info> {
    /// Validate that the struct is properly constructed.
    #[inline(never)]
    pub fn validate(&self) -> Result<(), ProgramError> {
        let n = self.params.mints.len();
        if n == 0 {
            return Err(ProgramError::InvalidArgument);
        }
        if self.mint_seed_accounts.len() != n {
            return Err(ProgramError::InvalidArgument);
        }
        if self.mints.len() != n {
            return Err(ProgramError::InvalidArgument);
        }
        Ok(())
    }

    /// Execute all CPIs to create and decompress all mints.
    ///
    /// Signer seeds are extracted from `SingleMintParams::mint_signer_seeds` and
    /// `SingleMintParams::authority_seeds` for each CPI call (0, 1, or 2 seeds per call).
    #[inline(never)]
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.validate()?;
        let n = self.params.mints.len();

        // Use single mint path only when:
        // - N=1 AND
        // - No CPI context offset (no PDAs were written to CPI context first)
        if n == 1 && self.params.cpi_context_offset == 0 {
            self.invoke_single_mint()
        } else {
            self.invoke_multiple_mints()
        }
    }

    /// Handle the single mint case: create + decompress in one CPI.
    #[inline(never)]
    fn invoke_single_mint(self) -> Result<(), ProgramError> {
        let mint_params = &self.params.mints[0];
        let (mint, bump) = get_mint_and_bump(mint_params);

        let mint_data =
            build_mint_instruction_data(mint_params, self.mint_seed_accounts[0].key, mint, bump);

        let decompress_action = DecompressMintAction {
            rent_payment: self.params.rent_payment,
            write_top_up: self.params.write_top_up,
        };

        let instruction_data = MintActionCompressedInstructionData::new_mint(
            self.params.address_merkle_tree_root_index,
            self.params.proof,
            mint_data,
        )
        .with_decompress_mint(decompress_action);

        let mut meta_config = MintActionMetaConfig::new_create_mint(
            *self.payer.key,
            *self.payer.key,
            *self.mint_seed_accounts[0].key,
            *self.address_tree.key,
            *self.output_queue.key,
        )
        .with_compressible_mint(
            *self.mints[0].key,
            *self.compressible_config.key,
            *self.rent_sponsor.key,
        );
        meta_config.input_queue = Some(*self.output_queue.key);

        self.invoke_mint_action(instruction_data, meta_config, 0)
    }

    /// Handle the multiple mints case: N-1 writes + 1 execute + N-1 decompress.
    #[inline(never)]
    fn invoke_multiple_mints(self) -> Result<(), ProgramError> {
        let n = self.params.mints.len();

        // Get base leaf index before any CPIs modify the queue
        let base_leaf_index = get_base_leaf_index(&self.output_queue)?;

        let decompress_action = DecompressMintAction {
            rent_payment: self.params.rent_payment,
            write_top_up: self.params.write_top_up,
        };

        // Write mints 0..N-2 to CPI context
        for i in 0..(n - 1) {
            self.invoke_cpi_write(i)?;
        }

        // Execute: create last mint + decompress it
        self.invoke_execute(n - 1, &decompress_action)?;

        // Decompress remaining mints (0..N-2)
        for i in 0..(n - 1) {
            self.invoke_decompress(i, base_leaf_index, &decompress_action)?;
        }

        Ok(())
    }

    /// Invoke a CPI write instruction for a single mint.
    /// Extracts signer seeds from mint params (0, 1, or 2 seeds).
    #[inline(never)]
    fn invoke_cpi_write(&self, index: usize) -> Result<(), ProgramError> {
        let mint_params = &self.params.mints[index];
        let offset = self.params.cpi_context_offset;
        let (mint, bump) = get_mint_and_bump(mint_params);

        // When sharing CPI context with PDAs:
        // - first_set_context: only true for index 0 AND offset 0 (first write to context)
        // - set_context: true if appending to existing context (index > 0 or offset > 0)
        // - assigned_account_index: offset + index (to not collide with PDA indices)
        let cpi_context = CpiContext {
            set_context: index > 0 || offset > 0,
            first_set_context: index == 0 && offset == 0,
            in_tree_index: self.params.address_tree_index,
            in_queue_index: self.params.output_queue_index,
            out_queue_index: self.params.output_queue_index,
            token_out_queue_index: 0,
            assigned_account_index: offset + index as u8,
            read_only_address_trees: [0; 4],
            address_tree_pubkey: self.address_tree.key.to_bytes(),
        };

        let mint_data = build_mint_instruction_data(
            mint_params,
            self.mint_seed_accounts[index].key,
            mint,
            bump,
        );

        let instruction_data = MintActionCompressedInstructionData::new_mint_write_to_cpi_context(
            self.params.address_merkle_tree_root_index,
            mint_data,
            cpi_context,
        );

        let cpi_write_config = MintActionMetaConfigCpiWrite {
            fee_payer: *self.payer.key,
            mint_signer: Some(*self.mint_seed_accounts[index].key),
            authority: *self.payer.key,
            cpi_context: *self.cpi_context_account.key,
        };

        let account_metas = get_mint_action_instruction_account_metas_cpi_write(cpi_write_config);
        let ix_data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        // Account order matches get_mint_action_instruction_account_metas_cpi_write:
        // [0]: light_system_program
        // [1]: mint_signer (optional, when present)
        // [2]: authority
        // [3]: fee_payer
        // [4]: cpi_authority_pda
        // [5]: cpi_context
        let account_infos = [
            self.system_accounts.light_system_program.clone(),
            self.mint_seed_accounts[index].clone(),
            self.payer.clone(),
            self.payer.clone(),
            self.system_accounts.cpi_authority_pda.clone(),
            self.cpi_context_account.clone(),
        ];
        let instruction = Instruction {
            program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data: ix_data,
        };

        // Build signer seeds - pack present seeds at start of array
        let mut seeds: [&[&[u8]]; 2] = [&[], &[]];
        let mut num_signers = 0;
        if let Some(s) = mint_params.mint_signer_seeds {
            seeds[num_signers] = s;
            num_signers += 1;
        }
        if let Some(s) = mint_params.authority_seeds {
            seeds[num_signers] = s;
            num_signers += 1;
        }
        solana_cpi::invoke_signed(&instruction, &account_infos, &seeds[..num_signers])
    }

    /// Invoke the execute instruction (create last mint + decompress).
    /// Extracts signer seeds from mint params (0, 1, or 2 seeds).
    #[inline(never)]
    fn invoke_execute(
        &self,
        last_idx: usize,
        decompress_action: &DecompressMintAction,
    ) -> Result<(), ProgramError> {
        let mint_params = &self.params.mints[last_idx];
        let offset = self.params.cpi_context_offset;
        let (mint, bump) = get_mint_and_bump(mint_params);

        let mint_data = build_mint_instruction_data(
            mint_params,
            self.mint_seed_accounts[last_idx].key,
            mint,
            bump,
        );

        // Create struct directly to reduce stack usage (avoid builder pattern intermediates)
        let instruction_data = MintActionCompressedInstructionData {
            leaf_index: 0,
            prove_by_index: false,
            root_index: self.params.address_merkle_tree_root_index,
            max_top_up: u16::MAX, // No limit
            create_mint: Some(CreateMint::default()),
            actions: vec![Action::DecompressMint(*decompress_action)],
            proof: Some(self.params.proof),
            cpi_context: Some(CpiContext {
                set_context: false,
                first_set_context: false,
                in_tree_index: self.params.address_tree_index,
                in_queue_index: self.params.address_tree_index,
                out_queue_index: self.params.output_queue_index,
                token_out_queue_index: 0,
                assigned_account_index: offset + last_idx as u8,
                read_only_address_trees: [0; 4],
                address_tree_pubkey: self.address_tree.key.to_bytes(),
            }),
            mint: Some(mint_data),
        };

        let mut meta_config = MintActionMetaConfig::new_create_mint(
            *self.payer.key,
            *self.payer.key,
            *self.mint_seed_accounts[last_idx].key,
            *self.address_tree.key,
            *self.output_queue.key,
        )
        .with_compressible_mint(
            *self.mints[last_idx].key,
            *self.compressible_config.key,
            *self.rent_sponsor.key,
        );
        meta_config.cpi_context = Some(*self.cpi_context_account.key);
        meta_config.input_queue = Some(*self.output_queue.key);

        self.invoke_mint_action(instruction_data, meta_config, last_idx)
    }

    /// Invoke decompress for a single mint.
    /// Extracts signer seeds from mint params (0, 1, or 2 seeds).
    #[inline(never)]
    fn invoke_decompress(
        &self,
        index: usize,
        base_leaf_index: u32,
        decompress_action: &DecompressMintAction,
    ) -> Result<(), ProgramError> {
        let mint_params = &self.params.mints[index];
        let (mint, bump) = get_mint_and_bump(mint_params);

        let mint_data = build_mint_instruction_data(
            mint_params,
            self.mint_seed_accounts[index].key,
            mint,
            bump,
        );

        let instruction_data = MintActionCompressedInstructionData {
            leaf_index: base_leaf_index + index as u32,
            prove_by_index: true,
            root_index: 0,
            max_top_up: u16::MAX, // No limit
            create_mint: None,
            actions: vec![Action::DecompressMint(*decompress_action)],
            proof: None,
            cpi_context: None,
            mint: Some(mint_data),
        };

        // For prove_by_index, the tree_pubkey must be state_merkle_tree for discriminator validation
        let meta_config = MintActionMetaConfig::new(
            *self.payer.key,
            *self.payer.key,
            *self.state_merkle_tree.key, // tree_pubkey - state merkle tree for discriminator check
            *self.output_queue.key,      // input_queue
            *self.output_queue.key,      // output_queue
        )
        .with_compressible_mint(
            *self.mints[index].key,
            *self.compressible_config.key,
            *self.rent_sponsor.key,
        );

        self.invoke_mint_action(instruction_data, meta_config, index)
    }

    /// Invoke a mint action instruction.
    /// Extracts signer seeds from mint params at the given index (0, 1, or 2 seeds).
    #[inline(never)]
    fn invoke_mint_action(
        &self,
        instruction_data: MintActionCompressedInstructionData,
        meta_config: MintActionMetaConfig,
        mint_index: usize,
    ) -> Result<(), ProgramError> {
        let account_metas = meta_config.to_account_metas();
        let ix_data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        // Collect all account infos needed for the CPI
        let mut account_infos = vec![self.payer.clone()];

        // System accounts
        account_infos.push(self.system_accounts.light_system_program.clone());

        // Add all mint seeds
        for mint_seed in self.mint_seed_accounts {
            account_infos.push(mint_seed.clone());
        }

        // More system accounts
        account_infos.push(self.system_accounts.cpi_authority_pda.clone());
        account_infos.push(self.system_accounts.registered_program_pda.clone());
        account_infos.push(self.system_accounts.account_compression_authority.clone());
        account_infos.push(self.system_accounts.account_compression_program.clone());
        account_infos.push(self.system_accounts.system_program.clone());

        // CPI context, queues, trees
        account_infos.push(self.cpi_context_account.clone());
        account_infos.push(self.output_queue.clone());
        account_infos.push(self.state_merkle_tree.clone());
        account_infos.push(self.address_tree.clone());
        account_infos.push(self.compressible_config.clone());
        account_infos.push(self.rent_sponsor.clone());

        // Add all mint PDAs
        for mint in self.mints {
            account_infos.push(mint.clone());
        }

        let instruction = Instruction {
            program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            accounts: account_metas,
            data: ix_data,
        };

        // Build signer seeds - pack present seeds at start of array
        let mint_params = &self.params.mints[mint_index];
        let mut seeds: [&[&[u8]]; 2] = [&[], &[]];
        let mut num_signers = 0;
        if let Some(s) = mint_params.mint_signer_seeds {
            seeds[num_signers] = s;
            num_signers += 1;
        }
        if let Some(s) = mint_params.authority_seeds {
            seeds[num_signers] = s;
            num_signers += 1;
        }
        solana_cpi::invoke_signed(&instruction, &account_infos, &seeds[..num_signers])
    }
}

/// Get mint PDA and bump, deriving mint always and bump if None.
#[inline(never)]
fn get_mint_and_bump(params: &SingleMintParams) -> (Pubkey, u8) {
    let (mint, derived_bump) = super::find_mint_address(&params.mint_seed_pubkey);
    let bump = params.mint_bump.unwrap_or(derived_bump);
    (mint, bump)
}

/// Build MintInstructionData for a single mint.
///
/// `mint` and `bump` are derived externally from `mint_seed_pubkey` using `get_mint_and_bump`.
#[inline(never)]
fn build_mint_instruction_data(
    mint_params: &SingleMintParams<'_>,
    mint_signer: &Pubkey,
    mint: Pubkey,
    bump: u8,
) -> MintInstructionData {
    // Convert token_metadata to extensions if present
    let extensions = mint_params
        .token_metadata
        .cloned()
        .map(|metadata| vec![ExtensionInstructionData::TokenMetadata(metadata)]);

    MintInstructionData {
        supply: 0,
        decimals: mint_params.decimals,
        metadata: MintMetadata {
            version: 3,
            mint: mint.to_bytes().into(),
            mint_decompressed: false,
            mint_signer: mint_signer.to_bytes(),
            bump,
        },
        mint_authority: Some(mint_params.mint_authority.to_bytes().into()),
        freeze_authority: mint_params.freeze_authority.map(|a| a.to_bytes().into()),
        extensions,
    }
}

/// Get base leaf index from output queue account.
#[inline(never)]
fn get_base_leaf_index(output_queue: &AccountInfo) -> Result<u32, ProgramError> {
    let queue = BatchedQueueRef::output_from_account_info(output_queue)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(queue.batch_metadata.next_index as u32)
}

/// Create multiple mints and decompress all to Solana accounts.
///
/// Convenience function that builds a [`CreateMintsCpi`] from a slice of accounts.
///
/// # Arguments
///
/// * `payer` - The fee payer account
/// * `accounts` - The remaining accounts in the expected layout
/// * `params` - Parameters for creating the mints
///
/// # Account Layout
///
/// - `[0]`: light_system_program
/// - `[1..N+1]`: mint_signers (SIGNER)
/// - `[N+1..N+6]`: system PDAs (cpi_authority, registered_program, compression_authority, compression_program, system_program)
/// - `[N+6]`: cpi_context_account (writable)
/// - `[N+7]`: output_queue (writable)
/// - `[N+8]`: state_merkle_tree (writable)
/// - `[N+9]`: address_tree (writable)
/// - `[N+10]`: compressible_config
/// - `[N+11]`: rent_sponsor (writable)
/// - `[N+12..2N+12]`: mint_pdas (writable)
/// - `[2N+12]`: compressed_token_program (for CPI)
#[inline(never)]
pub fn create_mints<'a, 'info>(
    payer: &AccountInfo<'info>,
    accounts: &'info [AccountInfo<'info>],
    params: CreateMintsParams<'a>,
) -> Result<(), ProgramError> {
    if params.mints.is_empty() {
        return Err(ProgramError::InvalidArgument);
    }

    let n = params.mints.len();
    let mint_signers_start = 1;
    let cpi_authority_idx = n + 1;
    let registered_program_idx = n + 2;
    let compression_authority_idx = n + 3;
    let compression_program_idx = n + 4;
    let system_program_idx = n + 5;
    let cpi_context_idx = n + 6;
    let output_queue_idx = n + 7;
    let state_merkle_tree_idx = n + 8;
    let address_tree_idx = n + 9;
    let compressible_config_idx = n + 10;
    let rent_sponsor_idx = n + 11;
    let mint_pdas_start = n + 12;

    // Build named struct from accounts slice
    let cpi = CreateMintsCpi {
        mint_seed_accounts: &accounts[mint_signers_start..mint_signers_start + n],
        payer: payer.clone(),
        address_tree: accounts[address_tree_idx].clone(),
        output_queue: accounts[output_queue_idx].clone(),
        state_merkle_tree: accounts[state_merkle_tree_idx].clone(),
        compressible_config: accounts[compressible_config_idx].clone(),
        mints: &accounts[mint_pdas_start..mint_pdas_start + n],
        rent_sponsor: accounts[rent_sponsor_idx].clone(),
        system_accounts: SystemAccountInfos {
            light_system_program: accounts[0].clone(),
            cpi_authority_pda: accounts[cpi_authority_idx].clone(),
            registered_program_pda: accounts[registered_program_idx].clone(),
            account_compression_authority: accounts[compression_authority_idx].clone(),
            account_compression_program: accounts[compression_program_idx].clone(),
            system_program: accounts[system_program_idx].clone(),
        },
        cpi_context_account: accounts[cpi_context_idx].clone(),
        params,
    };
    cpi.invoke()
}

// // ============================================================================
// // Client-side instruction builder
// // ============================================================================

// /// Client-side instruction builder for creating multiple mints.
// ///
// /// This struct is used to build instructions for client-side transaction construction.
// /// For CPI usage within Solana programs, use [`CreateMintsCpi`] instead.
// ///
// /// # Example
// ///
// /// ```rust,ignore
// /// use light_token::instruction::{CreateMints, CreateMintsParams, SingleMintParams};
// ///
// /// let params = CreateMintsParams::new(vec![mint1_params, mint2_params], proof);
// ///
// /// let instructions = CreateMints::new(
// ///     params,
// ///     mint_seed_pubkeys,
// ///     payer,
// ///     address_tree_pubkey,
// ///     output_queue,
// ///     state_merkle_tree,
// ///     cpi_context_pubkey,
// /// ).instructions()?;
// /// ```
// #[derive(Debug, Clone)]
// pub struct CreateMints<'a> {
//     pub payer: Pubkey,
//     pub address_tree_pubkey: Pubkey,
//     pub output_queue: Pubkey,
//     pub state_merkle_tree: Pubkey,
//     pub cpi_context_pubkey: Pubkey,
//     pub params: CreateMintsParams<'a>,
// }

// impl<'a> CreateMints<'a> {
//     #[allow(clippy::too_many_arguments)]
//     pub fn new(
//         params: CreateMintsParams<'a>,
//         payer: Pubkey,
//         address_tree_pubkey: Pubkey,
//         output_queue: Pubkey,
//         state_merkle_tree: Pubkey,
//         cpi_context_pubkey: Pubkey,
//     ) -> Self {
//         Self {
//             payer,
//             address_tree_pubkey,
//             output_queue,
//             state_merkle_tree,
//             cpi_context_pubkey,
//             params,
//         }
//     }

//     /// Build account metas for the instruction.
//     pub fn build_account_metas(&self) -> Vec<AccountMeta> {
//         let system_accounts = SystemAccounts::default();

//         let mut accounts = vec![AccountMeta::new_readonly(
//             system_accounts.light_system_program,
//             false,
//         )];

//         // Add mint signers (from each SingleMintParams)
//         for mint_params in self.params.mints {
//             accounts.push(AccountMeta::new_readonly(
//                 mint_params.mint_seed_pubkey,
//                 true,
//             ));
//         }

//         // Add system PDAs
//         accounts.extend(vec![
//             AccountMeta::new_readonly(system_accounts.cpi_authority_pda, false),
//             AccountMeta::new_readonly(system_accounts.registered_program_pda, false),
//             AccountMeta::new_readonly(system_accounts.account_compression_authority, false),
//             AccountMeta::new_readonly(system_accounts.account_compression_program, false),
//             AccountMeta::new_readonly(system_accounts.system_program, false),
//         ]);

//         // CPI context, output queue, address tree
//         accounts.push(AccountMeta::new(self.cpi_context_pubkey, false));
//         accounts.push(AccountMeta::new(self.output_queue, false));
//         accounts.push(AccountMeta::new(self.address_tree_pubkey, false));

//         // Config, rent sponsor
//         accounts.push(AccountMeta::new_readonly(config_pda(), false));
//         accounts.push(AccountMeta::new(rent_sponsor_pda(), false));

//         // State merkle tree
//         accounts.push(AccountMeta::new(self.state_merkle_tree, false));

//         // Add mint PDAs
//         for mint_params in self.params.mints {
//             accounts.push(AccountMeta::new(mint_params.mint, false));
//         }

//         accounts
//     }
// }
