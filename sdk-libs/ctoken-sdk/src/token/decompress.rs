use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo};
use light_token_interface::state::TokenDataVersion;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    compat::TokenData,
    compressed_token::{
        decompress_full::pack_for_decompress_full,
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
};

/// # Decompress compressed tokens to a token account
///
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::Decompress;
/// # use light_token_sdk::compat::TokenData;
/// # use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
/// # let destination_token_account = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let merkle_tree = Pubkey::new_unique();
/// # let queue = Pubkey::new_unique();
/// # let token_data = TokenData::default();
/// # let discriminator = [0, 0, 0, 0, 0, 0, 0, 4]; // ShaFlat
/// let instruction = Decompress {
///     token_data,
///     discriminator,
///     merkle_tree,
///     queue,
///     leaf_index: 0,
///     root_index: 0,
///     destination_token_account,
///     payer,
///     validity_proof: ValidityProof::new(None),
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decompress {
    /// Token data from the compressed account (compat version with solana_pubkey::Pubkey)
    pub token_data: TokenData,
    /// Compressed Token Account discriminator
    pub discriminator: [u8; 8],
    /// Merkle tree pubkey
    pub merkle_tree: Pubkey,
    /// Queue pubkey
    pub queue: Pubkey,
    /// Leaf index in the Merkle tree
    pub leaf_index: u32,
    /// Root index
    pub root_index: u16,
    /// Destination token account (must exist)
    pub destination_token_account: Pubkey,
    /// Fee payer
    pub payer: Pubkey,
    /// Validity proof for the compressed account
    pub validity_proof: ValidityProof,
}

impl Decompress {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Build packed accounts
        // Note: Don't add system accounts here - Transfer2AccountsMetaConfig adds them
        let mut packed_accounts = PackedAccounts::default();

        // Insert merkle tree and queue to get their indices
        let merkle_tree_pubkey_index = packed_accounts.insert_or_get(self.merkle_tree);
        let queue_pubkey_index = packed_accounts.insert_or_get(self.queue);

        // Build PackedStateTreeInfo
        // prove_by_index is true if validity proof is None (no ZK proof)
        let prove_by_index = self.validity_proof.0.is_none();
        let tree_info = PackedStateTreeInfo {
            merkle_tree_pubkey_index,
            queue_pubkey_index,
            leaf_index: self.leaf_index,
            root_index: self.root_index,
            prove_by_index,
        };

        let version = TokenDataVersion::from_discriminator(self.discriminator)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        let indices = pack_for_decompress_full(
            &self.token_data,
            &tree_info,
            self.destination_token_account,
            &mut packed_accounts,
            version as u8,
        );
        // Build CTokenAccount2 with decompress operation
        let mut token_account = CTokenAccount2::new(vec![indices.source])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        token_account
            .decompress_light_token(self.token_data.amount, indices.destination_index)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Build instruction inputs
        let (packed_account_metas, _, _) = packed_accounts.to_account_metas();
        let meta_config = Transfer2AccountsMetaConfig::new(self.payer, packed_account_metas);
        let transfer_config = Transfer2Config::default().filter_zero_amount_outputs();

        let inputs = Transfer2Inputs {
            meta_config,
            token_accounts: vec![token_account],
            transfer_config,
            validity_proof: self.validity_proof,
            ..Default::default()
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
