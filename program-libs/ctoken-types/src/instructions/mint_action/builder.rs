use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof,
    traits::{InstructionDiscriminator, LightInstructionData},
};

use crate::instructions::mint_action::{
    Action, CompressedMintInstructionData, CompressedMintWithContext, CpiContext, CreateMint,
    MintActionCompressedInstructionData, MintToCTokenAction, MintToCompressedAction,
    RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
    UpdateMetadataFieldAction,
};

/// Discriminator for MintAction instruction
pub const MINT_ACTION_DISCRIMINATOR: u8 = 103;

impl InstructionDiscriminator for MintActionCompressedInstructionData {
    fn discriminator(&self) -> &'static [u8] {
        &[MINT_ACTION_DISCRIMINATOR]
    }
}

impl LightInstructionData for MintActionCompressedInstructionData {}

// Builder pattern implementation for MintActionCompressedInstructionData
impl MintActionCompressedInstructionData {
    /// Create instruction data for an **existing** compressed mint.
    ///
    /// Use [`new_mint()`](Self::new_mint) to create a new mint instead.
    ///
    /// # Arguments
    /// * `mint_with_context` - Bundled compressed mint data with merkle context
    ///   - `leaf_index`: Leaf index in state tree (>0 for existing mint)
    ///   - `prove_by_index`: Whether to use proof-by-index for existing mint
    ///   - `root_index`: Root index for validity proof
    ///   - `address`: Deterministic address derived from SPL mint pubkey
    ///   - `mint`: Compressed mint state data
    /// * `proof` - ZK proof for compressed account validation (optional for some operations)
    ///
    /// # Note
    /// - `create_mint` is always set to None (this is for existing mints, not creating new ones)
    /// - `token_pool_bump` and `token_pool_index` are set to 0 (unused until SPL mint creation is supported)
    /// - Use `with_*` methods to add actions and configure the instruction
    pub fn new(
        mint_with_context: CompressedMintWithContext,
        proof: Option<CompressedProof>,
    ) -> Self {
        Self {
            leaf_index: mint_with_context.leaf_index,
            prove_by_index: mint_with_context.prove_by_index,
            root_index: mint_with_context.root_index,
            compressed_address: mint_with_context.address,
            token_pool_bump: 0,  // Unused until SPL mint creation supported
            token_pool_index: 0, // Unused until SPL mint creation supported
            create_mint: None,   // Always None for existing mints
            actions: Vec::new(),
            proof,
            cpi_context: None,
            mint: mint_with_context.mint,
        }
    }

    /// Create instruction data for a **new** compressed mint.
    ///
    /// Use [`new()`](Self::new) for operating on an existing mint instead.
    ///
    /// # Arguments
    /// * `compressed_address` - Deterministic address derived from SPL mint pubkey
    /// * `root_index` - Root index of the address proof (required for new address validation)
    /// * `proof` - ZK proof (REQUIRED - proves the address doesn't exist yet)
    /// * `mint` - Compressed mint state data (must match new mint parameters)
    /// * `create_mint` - CreateMint parameters (enables mint creation on-chain)
    ///
    /// # Constraints for new mints
    /// - `leaf_index` is set to 0 (no existing state tree entry)
    /// - `prove_by_index` is set to false (can't prove by index for new mint)
    /// - `proof` is REQUIRED (must prove address doesn't exist)
    ///
    /// # Example
    /// ```ignore
    /// let ix_data = MintActionCompressedInstructionData::new_mint(
    ///     compressed_mint_address,
    ///     root_index,
    ///     proof,
    ///     mint_data,
    /// )
    /// .with_mint_to_compressed(action);
    /// ```
    pub fn new_mint(
        compressed_address: [u8; 32],
        root_index: u16,
        proof: CompressedProof,
        mint: CompressedMintInstructionData,
    ) -> Self {
        Self {
            leaf_index: 0,         // Always 0 for new mint
            prove_by_index: false, // Always false for new mint
            root_index,
            compressed_address,
            token_pool_bump: 0,  // Unused until SPL mint creation supported
            token_pool_index: 0, // Unused until SPL mint creation supported
            create_mint: Some(CreateMint::default()),
            actions: Vec::new(),
            proof: Some(proof), // Required for new mint
            cpi_context: None,
            mint,
        }
    }

    /// Add MintToCompressed action - mint tokens to compressed token accounts.
    #[must_use = "with_mint_to_compressed returns a new value"]
    pub fn with_mint_to_compressed(mut self, action: MintToCompressedAction) -> Self {
        self.actions.push(Action::MintToCompressed(action));
        self
    }

    /// Add MintToCToken action - mint tokens to decompressed ctoken accounts.
    #[must_use = "with_mint_to_ctoken returns a new value"]
    pub fn with_mint_to_ctoken(mut self, action: MintToCTokenAction) -> Self {
        self.actions.push(Action::MintToCToken(action));
        self
    }

    /// Add UpdateMintAuthority action - update or remove mint authority.
    #[must_use = "with_update_mint_authority returns a new value"]
    pub fn with_update_mint_authority(mut self, authority: UpdateAuthority) -> Self {
        self.actions.push(Action::UpdateMintAuthority(authority));
        self
    }

    /// Add UpdateFreezeAuthority action - update or remove freeze authority.
    #[must_use = "with_update_freeze_authority returns a new value"]
    pub fn with_update_freeze_authority(mut self, authority: UpdateAuthority) -> Self {
        self.actions.push(Action::UpdateFreezeAuthority(authority));
        self
    }

    /// Add UpdateMetadataField action - update a metadata field in TokenMetadata extension.
    #[must_use = "with_update_metadata_field returns a new value"]
    pub fn with_update_metadata_field(mut self, action: UpdateMetadataFieldAction) -> Self {
        self.actions.push(Action::UpdateMetadataField(action));
        self
    }

    /// Add UpdateMetadataAuthority action - update metadata update authority.
    #[must_use = "with_update_metadata_authority returns a new value"]
    pub fn with_update_metadata_authority(mut self, action: UpdateMetadataAuthorityAction) -> Self {
        self.actions.push(Action::UpdateMetadataAuthority(action));
        self
    }

    /// Add RemoveMetadataKey action - remove a key from additional_metadata.
    #[must_use = "with_remove_metadata_key returns a new value"]
    pub fn with_remove_metadata_key(mut self, action: RemoveMetadataKeyAction) -> Self {
        self.actions.push(Action::RemoveMetadataKey(action));
        self
    }

    /// Set CPI context for batched operations across multiple programs.
    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(mut self, cpi_context: CpiContext) -> Self {
        self.cpi_context = Some(cpi_context);
        self
    }

    /// Enable CPI write mode as the first operation in a batch.
    /// This sets `first_set_context = true` in the CPI context.
    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        if let Some(ref mut ctx) = self.cpi_context {
            ctx.first_set_context = true;
            ctx.set_context = false;
        } else {
            // Create default CPI context with first_set_context enabled
            self.cpi_context = Some(CpiContext {
                first_set_context: true,
                ..Default::default()
            });
        }
        self
    }

    /// Enable CPI write mode as a subsequent operation in a batch.
    /// This sets `set_context = true` in the CPI context.
    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        if let Some(ref mut ctx) = self.cpi_context {
            ctx.set_context = true;
            ctx.first_set_context = false;
        } else {
            // Create default CPI context with set_context enabled
            self.cpi_context = Some(CpiContext {
                set_context: true,
                ..Default::default()
            });
        }
        self
    }
}
