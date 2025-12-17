use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof,
    traits::{InstructionDiscriminator, LightInstructionData},
};

use crate::instructions::mint_action::{
    Action, CompressedMintInstructionData, CompressedMintWithContext, CpiContext, CreateMint,
    MintActionCompressedInstructionData, MintToAction, MintToCompressedAction,
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

impl MintActionCompressedInstructionData {
    pub fn new(
        mint_with_context: CompressedMintWithContext,
        proof: Option<CompressedProof>,
    ) -> Self {
        Self {
            leaf_index: mint_with_context.leaf_index,
            prove_by_index: mint_with_context.prove_by_index,
            root_index: mint_with_context.root_index,
            compressed_address: mint_with_context.address,
            token_pool_bump: 0,
            token_pool_index: 0,
            max_top_up: 0, // No limit by default
            create_mint: None,
            actions: Vec::new(),
            proof,
            cpi_context: None,
            mint: mint_with_context.mint,
        }
    }

    pub fn new_mint(
        compressed_address: [u8; 32],
        root_index: u16,
        proof: CompressedProof,
        mint: CompressedMintInstructionData,
    ) -> Self {
        Self {
            leaf_index: 0,
            prove_by_index: false,
            root_index,
            compressed_address,
            token_pool_bump: 0,
            token_pool_index: 0,
            max_top_up: 0, // No limit by default
            create_mint: Some(CreateMint::default()),
            actions: Vec::new(),
            proof: Some(proof),
            cpi_context: None,
            mint,
        }
    }

    pub fn new_mint_write_to_cpi_context(
        compressed_address: [u8; 32],
        root_index: u16,
        mint: CompressedMintInstructionData,
        cpi_context: CpiContext,
    ) -> Self {
        Self {
            leaf_index: 0,
            prove_by_index: false,
            root_index,
            compressed_address,
            token_pool_bump: 0,
            token_pool_index: 0,
            max_top_up: 0, // No limit by default
            create_mint: Some(CreateMint::default()),
            actions: Vec::new(),
            proof: None, // Proof is verified with execution not write
            cpi_context: Some(cpi_context),
            mint,
        }
    }

    #[must_use = "with_mint_to_compressed returns a new value"]
    pub fn with_mint_to_compressed(mut self, action: MintToCompressedAction) -> Self {
        self.actions.push(Action::MintToCompressed(action));
        self
    }

    #[must_use = "with_mint_to returns a new value"]
    pub fn with_mint_to(mut self, action: MintToAction) -> Self {
        self.actions.push(Action::MintTo(action));
        self
    }

    #[must_use = "with_update_mint_authority returns a new value"]
    pub fn with_update_mint_authority(mut self, authority: UpdateAuthority) -> Self {
        self.actions.push(Action::UpdateMintAuthority(authority));
        self
    }

    #[must_use = "with_update_freeze_authority returns a new value"]
    pub fn with_update_freeze_authority(mut self, authority: UpdateAuthority) -> Self {
        self.actions.push(Action::UpdateFreezeAuthority(authority));
        self
    }

    #[must_use = "with_update_metadata_field returns a new value"]
    pub fn with_update_metadata_field(mut self, action: UpdateMetadataFieldAction) -> Self {
        self.actions.push(Action::UpdateMetadataField(action));
        self
    }

    #[must_use = "with_update_metadata_authority returns a new value"]
    pub fn with_update_metadata_authority(mut self, action: UpdateMetadataAuthorityAction) -> Self {
        self.actions.push(Action::UpdateMetadataAuthority(action));
        self
    }

    #[must_use = "with_remove_metadata_key returns a new value"]
    pub fn with_remove_metadata_key(mut self, action: RemoveMetadataKeyAction) -> Self {
        self.actions.push(Action::RemoveMetadataKey(action));
        self
    }

    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(mut self, cpi_context: CpiContext) -> Self {
        self.cpi_context = Some(cpi_context);
        self
    }

    #[must_use = "with_max_top_up returns a new value"]
    pub fn with_max_top_up(mut self, max_top_up: u16) -> Self {
        self.max_top_up = max_top_up;
        self
    }

    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        if let Some(ref mut ctx) = self.cpi_context {
            ctx.first_set_context = true;
            ctx.set_context = false;
        } else {
            self.cpi_context = Some(CpiContext {
                first_set_context: true,
                ..Default::default()
            });
        }
        self
    }

    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        if let Some(ref mut ctx) = self.cpi_context {
            ctx.set_context = true;
            ctx.first_set_context = false;
        } else {
            self.cpi_context = Some(CpiContext {
                set_context: true,
                ..Default::default()
            });
        }
        self
    }
}
