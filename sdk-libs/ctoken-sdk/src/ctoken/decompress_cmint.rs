use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, traits::LightInstructionData,
};
use light_ctoken_interface::instructions::mint_action::{
    CompressedMintWithContext, DecompressMintAction, MintActionCompressedInstructionData,
};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::{config_pda, rent_sponsor_pda};
use crate::compressed_token::mint_action::MintActionMetaConfig;

pub use super::find_cmint_address;

/// Decompress a compressed mint to a CMint Solana account.
///
/// Creates an on-chain CMint PDA that becomes the source of truth.
/// The CMint is always compressible.
///
/// # Example
/// ```rust,ignore
/// let instruction = DecompressCMint {
///     mint_seed_pubkey,
///     payer,
///     authority,
///     state_tree,
///     input_queue,
///     output_queue,
///     compressed_mint_with_context,
///     proof,
///     rent_payment: 16,       // epochs (~24 hours rent)
///     write_top_up: 766,      // lamports (~3 hours rent per write)
/// }.instruction()?;
/// ```
#[derive(Debug, Clone)]
pub struct DecompressCMint {
    /// Mint seed pubkey (used to derive CMint PDA)
    pub mint_seed_pubkey: Pubkey,
    /// Fee payer
    pub payer: Pubkey,
    /// Mint authority (must sign)
    pub authority: Pubkey,
    /// State tree for the compressed mint
    pub state_tree: Pubkey,
    /// Input queue for reading compressed mint
    pub input_queue: Pubkey,
    /// Output queue for updated compressed mint
    pub output_queue: Pubkey,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Validity proof for the compressed mint
    pub proof: ValidityProof,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}

impl DecompressCMint {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Derive CMint PDA
        let (cmint_pda, cmint_bump) = find_cmint_address(&self.mint_seed_pubkey);

        // Build DecompressMintAction
        let action = DecompressMintAction {
            cmint_bump,
            rent_payment: self.rent_payment,
            write_top_up: self.write_top_up,
        };

        // Build instruction data
        let instruction_data = MintActionCompressedInstructionData::new(
            self.compressed_mint_with_context,
            self.proof.0,
        )
        .with_decompress_mint(action);

        // Build account metas with compressible CMint
        let meta_config = MintActionMetaConfig::new(
            self.payer,
            self.authority,
            self.state_tree,
            self.input_queue,
            self.output_queue,
        )
        .with_compressible_cmint(cmint_pda, config_pda(), rent_sponsor_pda())
        .with_mint_signer_no_sign(self.mint_seed_pubkey);

        let account_metas = meta_config.to_account_metas();

        let data = instruction_data
            .data()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        Ok(Instruction {
            program_id: Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID),
            accounts: account_metas,
            data,
        })
    }
}
