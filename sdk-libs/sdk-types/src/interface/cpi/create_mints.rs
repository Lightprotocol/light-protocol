//! Generic CPI for creating multiple compressed mints.
//!
//! This module provides framework-agnostic mint creation via `CreateMintsCpi`,
//! generic over `AccountInfoTrait`. Account order matches the cToken program
//! expectations (see `MintActionMetaConfig::to_account_metas` for reference).
//!
//! # Flow
//!
//! - N=1 (no CPI context offset): Single CPI (create + decompress)
//! - N>1 or offset>0: 2N-1 CPIs (N-1 writes + 1 execute with decompress + N-1 decompress)

use alloc::{vec, vec::Vec};

use light_account_checks::{AccountInfoTrait, CpiMeta};
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, traits::LightInstructionData,
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
    COMPRESSED_MINT_SEED, LIGHT_TOKEN_PROGRAM_ID,
};

use crate::error::LightSdkTypesError;

/// Default rent payment epochs (~24 hours).
pub const DEFAULT_RENT_PAYMENT: u8 = 16;
/// Default lamports for write operations (~3 hours per write).
pub const DEFAULT_WRITE_TOP_UP: u32 = 766;

// ============================================================================
// Types
// ============================================================================

/// Parameters for a single mint within a batch creation.
///
/// All pubkeys are `[u8; 32]` for framework independence.
/// `mint` and `compression_address` are derived internally from `mint_seed_pubkey`.
#[derive(Debug, Clone)]
pub struct SingleMintParams<'a> {
    pub decimals: u8,
    pub mint_authority: [u8; 32],
    /// Optional mint bump. If `None`, derived from `find_mint_address(mint_seed_pubkey)`.
    pub mint_bump: Option<u8>,
    pub freeze_authority: Option<[u8; 32]>,
    /// Mint seed pubkey (signer) for this mint. Used to derive `mint` PDA and `compression_address`.
    pub mint_seed_pubkey: [u8; 32],
    /// Optional authority seeds for PDA signing.
    pub authority_seeds: Option<&'a [&'a [u8]]>,
    /// Optional mint signer seeds for PDA signing.
    pub mint_signer_seeds: Option<&'a [&'a [u8]]>,
    /// Optional token metadata for the mint (reference to avoid stack overflow).
    pub token_metadata: Option<&'a TokenMetadataInstructionData>,
}

/// Parameters for creating one or more compressed mints with decompression.
///
/// Creates N compressed mints and decompresses all to Solana Mint accounts.
/// Uses CPI context pattern when N > 1 for efficiency.
#[derive(Debug, Clone)]
pub struct CreateMintsParams<'a> {
    /// Parameters for each mint to create.
    pub mints: &'a [SingleMintParams<'a>],
    /// Single proof covering all new addresses.
    pub proof: CompressedProof,
    /// Root index for the address merkle tree (shared by all mints in batch).
    pub address_merkle_tree_root_index: u16,
    /// Rent payment in epochs for the Mint account (must be 0 or >= 2).
    /// Default: 16 (~24 hours).
    pub rent_payment: u8,
    /// Lamports allocated for future write operations.
    /// Default: 766 (~3 hours per write).
    pub write_top_up: u32,
    /// Offset for assigned_account_index when sharing CPI context with other accounts.
    /// When creating mints alongside PDAs, this offset should be set to the number of
    /// PDAs already written to the CPI context.
    /// Default: 0 (no offset).
    pub cpi_context_offset: u8,
    /// Index of the output queue in tree accounts.
    /// Default: 0.
    pub output_queue_index: u8,
    /// Index of the address merkle tree in tree accounts.
    /// Default: 1.
    pub address_tree_index: u8,
    /// Index of the state merkle tree in tree accounts.
    /// Required for decompress operations (discriminator validation).
    /// Default: 2.
    pub state_tree_index: u8,
    /// Base leaf index from the output queue (required when N > 1).
    /// Read from the queue's batch_metadata.next_index before any CPIs.
    /// For N=1 with offset=0, pass 0.
    pub base_leaf_index: u32,
}

#[cfg(feature = "cpi-context")]
impl<'a> CreateMintsParams<'a> {
    /// Create params from proof data and CPI accounts.
    ///
    /// Extracts tree indices and computes base_leaf_index automatically (only for N > 1 mints).
    /// Uses default values for rent_payment, write_top_up, and cpi_context_offset.
    pub fn from_proof<AI: light_account_checks::AccountInfoTrait + Clone>(
        mints: &'a [SingleMintParams<'a>],
        proof_data: &crate::interface::CreateAccountsProof,
        cpi_accounts: &crate::cpi_accounts::v2::CpiAccounts<'_, AI>,
    ) -> Result<Self, LightSdkTypesError> {
        let proof = proof_data
            .proof
            .0
            .ok_or(LightSdkTypesError::InvalidInstructionData)?;

        let state_tree_index = proof_data
            .state_tree_index
            .ok_or(LightSdkTypesError::InvalidInstructionData)?;

        let output_queue_index = proof_data.output_state_tree_index;

        // Only read base_leaf_index when there are multiple mints (needed for decompress indexing)
        let base_leaf_index = if mints.len() > 1 {
            let output_queue = cpi_accounts.get_tree_account_info(output_queue_index as usize)?;
            get_output_queue_next_index(output_queue)?
        } else {
            0
        };

        Ok(Self {
            mints,
            proof,
            address_merkle_tree_root_index: proof_data.address_tree_info.root_index,
            rent_payment: DEFAULT_RENT_PAYMENT,
            write_top_up: DEFAULT_WRITE_TOP_UP,
            cpi_context_offset: 0,
            output_queue_index,
            address_tree_index: proof_data
                .address_tree_info
                .address_merkle_tree_pubkey_index,
            state_tree_index,
            base_leaf_index,
        })
    }
}

/// Infrastructure accounts needed for mint creation CPI.
///
/// These accounts are passed from the user's Accounts struct.
pub struct CreateMintsStaticAccounts<'a, AI: AccountInfoTrait + Clone> {
    /// Fee payer for the transaction.
    pub fee_payer: &'a AI,
    /// CompressibleConfig account for the light-token program.
    pub compressible_config: &'a AI,
    /// Rent sponsor PDA.
    pub rent_sponsor: &'a AI,
    /// CPI authority PDA for signing.
    pub cpi_authority: &'a AI,
}

/// CPI struct for creating multiple compressed mints.
///
/// Generic over `AccountInfoTrait` to work with both solana and pinocchio backends.
/// Uses named account fields for clarity and safety.
pub struct CreateMintsCpi<'a, AI: AccountInfoTrait + Clone> {
    /// Mint seed accounts (signers) - one per mint.
    pub mint_seed_accounts: &'a [AI],
    /// Fee payer (also used as authority).
    pub payer: &'a AI,
    /// Address tree for new mint addresses.
    pub address_tree: &'a AI,
    /// Output queue for compressed accounts.
    pub output_queue: &'a AI,
    /// State merkle tree (required for decompress discriminator validation).
    pub state_merkle_tree: &'a AI,
    /// CompressibleConfig account.
    pub compressible_config: &'a AI,
    /// Mint PDA accounts (writable) - one per mint.
    pub mints: &'a [AI],
    /// Rent sponsor PDA.
    pub rent_sponsor: &'a AI,
    /// Light system program.
    pub light_system_program: &'a AI,
    /// CPI authority PDA.
    pub cpi_authority_pda: &'a AI,
    /// Registered program PDA.
    pub registered_program_pda: &'a AI,
    /// Account compression authority.
    pub account_compression_authority: &'a AI,
    /// Account compression program.
    pub account_compression_program: &'a AI,
    /// System program.
    pub system_program: &'a AI,
    /// CPI context account.
    pub cpi_context_account: &'a AI,
    /// Parameters.
    pub params: CreateMintsParams<'a>,
}

impl<'a, AI: AccountInfoTrait + Clone> CreateMintsCpi<'a, AI> {
    /// Validate that the struct is properly constructed.
    #[inline(never)]
    fn validate(&self) -> Result<(), LightSdkTypesError> {
        let n = self.params.mints.len();
        if n == 0 {
            return Err(LightSdkTypesError::InvalidInstructionData);
        }
        if self.mint_seed_accounts.len() != n {
            return Err(LightSdkTypesError::InvalidInstructionData);
        }
        if self.mints.len() != n {
            return Err(LightSdkTypesError::InvalidInstructionData);
        }
        Ok(())
    }

    /// Execute all CPIs to create and decompress all mints.
    #[inline(never)]
    pub fn invoke(self) -> Result<(), LightSdkTypesError> {
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
    fn invoke_single_mint(self) -> Result<(), LightSdkTypesError> {
        let mint_params = &self.params.mints[0];
        let (mint, bump) = get_mint_and_bump::<AI>(mint_params);

        let mint_data =
            build_mint_instruction_data(mint_params, &self.mint_seed_accounts[0].key(), mint, bump);

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

        let ix_data = instruction_data
            .data()
            .map_err(|_| LightSdkTypesError::Borsh)?;

        let (metas, account_infos) = self.build_mint_action(0, true, true, false);

        self.invoke_mint_action_raw(&ix_data, &account_infos, &metas, 0)
    }

    /// Handle the multiple mints case: N-1 writes + 1 execute + N-1 decompress.
    #[inline(never)]
    fn invoke_multiple_mints(self) -> Result<(), LightSdkTypesError> {
        let n = self.params.mints.len();
        let base_leaf_index = self.params.base_leaf_index;

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
    #[inline(never)]
    fn invoke_cpi_write(&self, index: usize) -> Result<(), LightSdkTypesError> {
        let mint_params = &self.params.mints[index];
        let offset = self.params.cpi_context_offset;
        let (mint, bump) = get_mint_and_bump::<AI>(mint_params);

        let cpi_context = CpiContext {
            set_context: index > 0 || offset > 0,
            first_set_context: index == 0 && offset == 0,
            in_tree_index: self.params.address_tree_index,
            in_queue_index: self.params.output_queue_index,
            out_queue_index: self.params.output_queue_index,
            token_out_queue_index: 0,
            assigned_account_index: offset + index as u8,
            read_only_address_trees: [0; 4],
            address_tree_pubkey: self.address_tree.key(),
        };

        let mint_data = build_mint_instruction_data(
            mint_params,
            &self.mint_seed_accounts[index].key(),
            mint,
            bump,
        );

        let instruction_data = MintActionCompressedInstructionData::new_mint_write_to_cpi_context(
            self.params.address_merkle_tree_root_index,
            mint_data,
            cpi_context,
        );

        let ix_data = instruction_data
            .data()
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // CPI write account order:
        // [0]: light_system_program
        // [1]: mint_signer
        // [2]: authority (payer)
        // [3]: fee_payer (payer)
        // [4]: cpi_authority_pda
        // [5]: cpi_context
        let metas = vec![
            CpiMeta {
                pubkey: self.light_system_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.mint_seed_accounts[index].key(),
                is_signer: true,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.payer.key(),
                is_signer: true,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.payer.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: self.cpi_authority_pda.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.cpi_context_account.key(),
                is_signer: false,
                is_writable: true,
            },
        ];

        let account_infos = vec![
            self.light_system_program.clone(),
            self.mint_seed_accounts[index].clone(),
            self.payer.clone(),
            self.payer.clone(),
            self.cpi_authority_pda.clone(),
            self.cpi_context_account.clone(),
        ];

        self.invoke_mint_action_raw(&ix_data, &account_infos, &metas, index)
    }

    /// Invoke the execute instruction (create last mint + decompress).
    #[inline(never)]
    fn invoke_execute(
        &self,
        last_idx: usize,
        decompress_action: &DecompressMintAction,
    ) -> Result<(), LightSdkTypesError> {
        let mint_params = &self.params.mints[last_idx];
        let offset = self.params.cpi_context_offset;
        let (mint, bump) = get_mint_and_bump::<AI>(mint_params);

        let mint_data = build_mint_instruction_data(
            mint_params,
            &self.mint_seed_accounts[last_idx].key(),
            mint,
            bump,
        );

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
                address_tree_pubkey: self.address_tree.key(),
            }),
            mint: Some(mint_data),
        };

        let ix_data = instruction_data
            .data()
            .map_err(|_| LightSdkTypesError::Borsh)?;

        let (metas, account_infos) = self.build_mint_action(last_idx, true, true, true);

        self.invoke_mint_action_raw(&ix_data, &account_infos, &metas, last_idx)
    }

    /// Invoke decompress for a single mint.
    #[inline(never)]
    fn invoke_decompress(
        &self,
        index: usize,
        base_leaf_index: u32,
        decompress_action: &DecompressMintAction,
    ) -> Result<(), LightSdkTypesError> {
        let mint_params = &self.params.mints[index];
        let (mint, bump) = get_mint_and_bump::<AI>(mint_params);

        let mint_data = build_mint_instruction_data(
            mint_params,
            &self.mint_seed_accounts[index].key(),
            mint,
            bump,
        );

        let instruction_data = MintActionCompressedInstructionData {
            leaf_index: base_leaf_index + self.params.cpi_context_offset as u32 + index as u32,
            prove_by_index: true,
            root_index: 0,
            max_top_up: u16::MAX, // No limit
            create_mint: None,
            actions: vec![Action::DecompressMint(*decompress_action)],
            proof: None,
            cpi_context: None,
            mint: Some(mint_data),
        };

        let ix_data = instruction_data
            .data()
            .map_err(|_| LightSdkTypesError::Borsh)?;

        let (metas, account_infos) = self.build_decompress_action(index);

        self.invoke_mint_action_raw(&ix_data, &account_infos, &metas, index)
    }

    /// Low-level invoke: build signer seeds from mint params and call CPI.
    #[inline(never)]
    fn invoke_mint_action_raw(
        &self,
        ix_data: &[u8],
        account_infos: &[AI],
        metas: &[CpiMeta],
        mint_index: usize,
    ) -> Result<(), LightSdkTypesError> {
        let mint_params = &self.params.mints[mint_index];

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

        AI::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            ix_data,
            metas,
            account_infos,
            &seeds[..num_signers],
        )
        .map_err(|_| LightSdkTypesError::CpiFailed)
    }

    /// Build matched account metas and infos for a full mint action CPI.
    ///
    /// Returns `(metas, infos)` in identical order so pinocchio's 1:1
    /// positional CPI requirement is satisfied without runtime reordering.
    ///
    /// Order matches `MintActionMetaConfig::to_account_metas`:
    /// light_system_program, [mint_signer], authority, [compressible_config],
    /// [mint], [rent_sponsor], fee_payer, cpi_authority_pda, registered_program_pda,
    /// account_compression_authority, account_compression_program, system_program,
    /// [cpi_context], output_queue, tree_pubkey, [input_queue]
    #[inline(never)]
    fn build_mint_action(
        &self,
        mint_index: usize,
        has_input_queue: bool,
        has_compressible: bool,
        has_cpi_context: bool,
    ) -> (Vec<CpiMeta>, Vec<AI>) {
        let mut metas = Vec::with_capacity(17);
        let mut infos = Vec::with_capacity(17);

        // light_system_program
        metas.push(CpiMeta {
            pubkey: self.light_system_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.light_system_program.clone());

        // mint_signer (always present for create_mint, must sign)
        metas.push(CpiMeta {
            pubkey: self.mint_seed_accounts[mint_index].key(),
            is_signer: true,
            is_writable: false,
        });
        infos.push(self.mint_seed_accounts[mint_index].clone());

        // authority (payer is authority)
        metas.push(CpiMeta {
            pubkey: self.payer.key(),
            is_signer: true,
            is_writable: false,
        });
        infos.push(self.payer.clone());

        if has_compressible {
            // compressible_config
            metas.push(CpiMeta {
                pubkey: self.compressible_config.key(),
                is_signer: false,
                is_writable: false,
            });
            infos.push(self.compressible_config.clone());

            // mint PDA (writable)
            metas.push(CpiMeta {
                pubkey: self.mints[mint_index].key(),
                is_signer: false,
                is_writable: true,
            });
            infos.push(self.mints[mint_index].clone());

            // rent_sponsor (writable)
            metas.push(CpiMeta {
                pubkey: self.rent_sponsor.key(),
                is_signer: false,
                is_writable: true,
            });
            infos.push(self.rent_sponsor.clone());
        }

        // fee_payer (signer, writable)
        metas.push(CpiMeta {
            pubkey: self.payer.key(),
            is_signer: true,
            is_writable: true,
        });
        infos.push(self.payer.clone());

        // cpi_authority_pda
        metas.push(CpiMeta {
            pubkey: self.cpi_authority_pda.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.cpi_authority_pda.clone());

        // registered_program_pda
        metas.push(CpiMeta {
            pubkey: self.registered_program_pda.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.registered_program_pda.clone());

        // account_compression_authority
        metas.push(CpiMeta {
            pubkey: self.account_compression_authority.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.account_compression_authority.clone());

        // account_compression_program
        metas.push(CpiMeta {
            pubkey: self.account_compression_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.account_compression_program.clone());

        // system_program
        metas.push(CpiMeta {
            pubkey: self.system_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.system_program.clone());

        // cpi_context (optional)
        if has_cpi_context {
            metas.push(CpiMeta {
                pubkey: self.cpi_context_account.key(),
                is_signer: false,
                is_writable: true,
            });
            infos.push(self.cpi_context_account.clone());
        }

        // output_queue (writable)
        metas.push(CpiMeta {
            pubkey: self.output_queue.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.output_queue.clone());

        // tree_pubkey (address_tree for create_mint)
        metas.push(CpiMeta {
            pubkey: self.address_tree.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.address_tree.clone());

        // input_queue (optional, same as output_queue for create_mint)
        if has_input_queue {
            metas.push(CpiMeta {
                pubkey: self.output_queue.key(),
                is_signer: false,
                is_writable: true,
            });
            infos.push(self.output_queue.clone());
        }

        (metas, infos)
    }

    /// Build matched account metas and infos for a decompress CPI.
    ///
    /// For prove_by_index, tree_pubkey must be state_merkle_tree for discriminator validation.
    #[inline(never)]
    fn build_decompress_action(&self, mint_index: usize) -> (Vec<CpiMeta>, Vec<AI>) {
        let mut metas = Vec::with_capacity(14);
        let mut infos = Vec::with_capacity(14);

        // light_system_program
        metas.push(CpiMeta {
            pubkey: self.light_system_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.light_system_program.clone());

        // No mint_signer for decompress

        // authority (payer is authority, signer)
        metas.push(CpiMeta {
            pubkey: self.payer.key(),
            is_signer: true,
            is_writable: false,
        });
        infos.push(self.payer.clone());

        // compressible_config
        metas.push(CpiMeta {
            pubkey: self.compressible_config.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.compressible_config.clone());

        // mint PDA (writable)
        metas.push(CpiMeta {
            pubkey: self.mints[mint_index].key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.mints[mint_index].clone());

        // rent_sponsor (writable)
        metas.push(CpiMeta {
            pubkey: self.rent_sponsor.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.rent_sponsor.clone());

        // fee_payer (signer, writable)
        metas.push(CpiMeta {
            pubkey: self.payer.key(),
            is_signer: true,
            is_writable: true,
        });
        infos.push(self.payer.clone());

        // cpi_authority_pda
        metas.push(CpiMeta {
            pubkey: self.cpi_authority_pda.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.cpi_authority_pda.clone());

        // registered_program_pda
        metas.push(CpiMeta {
            pubkey: self.registered_program_pda.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.registered_program_pda.clone());

        // account_compression_authority
        metas.push(CpiMeta {
            pubkey: self.account_compression_authority.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.account_compression_authority.clone());

        // account_compression_program
        metas.push(CpiMeta {
            pubkey: self.account_compression_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.account_compression_program.clone());

        // system_program
        metas.push(CpiMeta {
            pubkey: self.system_program.key(),
            is_signer: false,
            is_writable: false,
        });
        infos.push(self.system_program.clone());

        // No cpi_context for decompress

        // output_queue (writable)
        metas.push(CpiMeta {
            pubkey: self.output_queue.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.output_queue.clone());

        // tree_pubkey = state_merkle_tree for prove_by_index discriminator check
        metas.push(CpiMeta {
            pubkey: self.state_merkle_tree.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.state_merkle_tree.clone());

        // input_queue = output_queue
        metas.push(CpiMeta {
            pubkey: self.output_queue.key(),
            is_signer: false,
            is_writable: true,
        });
        infos.push(self.output_queue.clone());

        (metas, infos)
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Get mint PDA and bump, deriving mint always and bump if None.
#[inline(never)]
fn get_mint_and_bump<AI: AccountInfoTrait>(params: &SingleMintParams) -> ([u8; 32], u8) {
    let (mint, derived_bump) = find_mint_address::<AI>(&params.mint_seed_pubkey);
    let bump = params.mint_bump.unwrap_or(derived_bump);
    (mint, bump)
}

/// Build `MintInstructionData` for a single mint.
///
/// `mint` and `bump` are derived externally from `mint_seed_pubkey` using `get_mint_and_bump`.
#[inline(never)]
fn build_mint_instruction_data(
    mint_params: &SingleMintParams<'_>,
    mint_signer: &[u8; 32],
    mint: [u8; 32],
    bump: u8,
) -> MintInstructionData {
    let extensions = mint_params
        .token_metadata
        .cloned()
        .map(|metadata| vec![ExtensionInstructionData::TokenMetadata(metadata)]);

    MintInstructionData {
        supply: 0,
        decimals: mint_params.decimals,
        metadata: MintMetadata {
            version: 3,
            mint: mint.into(),
            mint_decompressed: false,
            mint_signer: *mint_signer,
            bump,
        },
        mint_authority: Some(mint_params.mint_authority.into()),
        freeze_authority: mint_params.freeze_authority.map(|a| a.into()),
        extensions,
    }
}

/// Find the mint PDA address for a given mint seed.
///
/// Generic over `AccountInfoTrait` to use the correct backend for PDA derivation.
/// Returns `([u8; 32], u8)` -- the PDA address bytes and bump.
pub fn find_mint_address<AI: AccountInfoTrait>(mint_seed: &[u8; 32]) -> ([u8; 32], u8) {
    AI::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Derive the compressed mint address from a mint seed and address tree pubkey.
///
/// This computes `derive_address(find_mint_address(mint_seed).0, address_tree, LIGHT_TOKEN_PROGRAM_ID)`.
pub fn derive_mint_compressed_address<AI: AccountInfoTrait>(
    mint_seed: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
) -> [u8; 32] {
    light_compressed_account::address::derive_address(
        &find_mint_address::<AI>(mint_seed).0,
        address_tree_pubkey,
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

/// Read the next_index from a batched output queue account.
///
/// Offset 288 = 8 (discriminator) + 232 (QueueMetadata) + 48 (6 x u64 in QueueBatches).
/// This reads the raw bytes to avoid depending on `light-batched-merkle-tree`.
pub fn get_output_queue_next_index<AI: AccountInfoTrait>(
    queue: &AI,
) -> Result<u32, LightSdkTypesError> {
    const NEXT_INDEX_OFFSET: usize = 288;
    let data = queue
        .try_borrow_data()
        .map_err(LightSdkTypesError::AccountError)?;
    if data.len() < NEXT_INDEX_OFFSET + 8 {
        return Err(LightSdkTypesError::AccountDataTooSmall);
    }
    let next_index = u64::from_le_bytes(
        data[NEXT_INDEX_OFFSET..NEXT_INDEX_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    Ok(next_index as u32)
}

// ============================================================================
// High-level CreateMints API
// ============================================================================

/// High-level struct for creating compressed mints.
///
/// Consolidates proof parsing, tree account resolution, and CPI invocation into
/// a single `.invoke()` call. This is the recommended API for creating mints.
///
/// # Example
///
/// ```rust,ignore
/// CreateMints {
///     mints: &sdk_mints,
///     proof_data: &params.create_accounts_proof,
///     mint_seed_accounts,
///     mint_accounts,
///     static_accounts: CreateMintsStaticAccounts { ... },
///     cpi_context_offset: 0,
/// }
/// .invoke(&cpi_accounts)?;
/// ```
#[cfg(feature = "cpi-context")]
pub struct CreateMints<'a, AI: AccountInfoTrait + Clone> {
    /// Per-mint parameters.
    pub mints: &'a [SingleMintParams<'a>],
    /// Proof data containing tree indices, proof, etc.
    pub proof_data: &'a crate::interface::CreateAccountsProof,
    /// Mint seed accounts (signers) - one per mint.
    pub mint_seed_accounts: &'a [AI],
    /// Mint PDA accounts (writable) - one per mint.
    pub mint_accounts: &'a [AI],
    /// Infrastructure accounts (payer, config, rent_sponsor, cpi_authority).
    pub static_accounts: CreateMintsStaticAccounts<'a, AI>,
    /// Offset for assigned_account_index when sharing CPI context with other accounts.
    /// When creating mints alongside PDAs, this should be the number of PDAs already
    /// written to the CPI context. Default: 0.
    pub cpi_context_offset: u8,
}

#[cfg(feature = "cpi-context")]
impl<'a, AI: AccountInfoTrait + Clone> CreateMints<'a, AI> {
    /// Execute mint creation by:
    /// 1. Building CreateMintsParams from proof_data
    /// 2. Resolving tree accounts from cpi_accounts
    /// 3. Invoking CreateMintsCpi
    pub fn invoke(
        self,
        cpi_accounts: &crate::cpi_accounts::v2::CpiAccounts<'_, AI>,
    ) -> Result<(), LightSdkTypesError> {
        let mut params = CreateMintsParams::from_proof(self.mints, self.proof_data, cpi_accounts)?;
        params.cpi_context_offset = self.cpi_context_offset;

        invoke_create_mints(
            self.mint_seed_accounts,
            self.mint_accounts,
            params,
            self.static_accounts,
            cpi_accounts,
        )
    }
}

/// Convenience function that extracts accounts from CpiAccounts and invokes CreateMintsCpi.
///
/// For new code, prefer using [`CreateMints`] with `.invoke()` instead.
#[cfg(feature = "cpi-context")]
fn invoke_create_mints<'a, AI: AccountInfoTrait + Clone>(
    mint_seed_accounts: &'a [AI],
    mint_accounts: &'a [AI],
    params: CreateMintsParams<'a>,
    infra: CreateMintsStaticAccounts<'a, AI>,
    cpi_accounts: &crate::cpi_accounts::v2::CpiAccounts<'_, AI>,
) -> Result<(), LightSdkTypesError> {
    let output_queue = cpi_accounts
        .get_tree_account_info(params.output_queue_index as usize)?
        .clone();
    let state_merkle_tree = cpi_accounts
        .get_tree_account_info(params.state_tree_index as usize)?
        .clone();
    let address_tree = cpi_accounts
        .get_tree_account_info(params.address_tree_index as usize)?
        .clone();

    CreateMintsCpi {
        mint_seed_accounts,
        payer: infra.fee_payer,
        address_tree: &address_tree,
        output_queue: &output_queue,
        state_merkle_tree: &state_merkle_tree,
        compressible_config: infra.compressible_config,
        mints: mint_accounts,
        rent_sponsor: infra.rent_sponsor,
        light_system_program: cpi_accounts.light_system_program()?,
        cpi_authority_pda: infra.cpi_authority,
        registered_program_pda: cpi_accounts.registered_program_pda()?,
        account_compression_authority: cpi_accounts.account_compression_authority()?,
        account_compression_program: cpi_accounts.account_compression_program()?,
        system_program: cpi_accounts.system_program()?,
        cpi_context_account: cpi_accounts.cpi_context()?,
        params,
    }
    .invoke()
}
