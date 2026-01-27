use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_compressed_token_sdk::compressed_token::{
    decompress_full::pack_for_decompress_full_with_ata,
    transfer2::{
        create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config, Transfer2Inputs,
    },
    CTokenAccount2,
};
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo};
use light_token_interface::state::TokenData;
use light_token_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::{AccountState, ExtensionStruct, TokenDataVersion},
};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    // compat::{AccountState, TokenData},
    instruction::derive_associated_token_account,
};

/// # Decompress compressed tokens to a cToken account
///
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::Decompress;
/// # use light_token::compat::TokenData;
/// # use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
/// # let destination = Pubkey::new_unique();
/// # let payer = Pubkey::new_unique();
/// # let signer = Pubkey::new_unique();
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
///     destination,
///     payer,
///     signer,
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
    /// Destination cToken account (must exist)
    pub destination: Pubkey,
    /// Fee payer
    pub payer: Pubkey,
    /// Signer (wallet owner, delegate, or permanent delegate)
    pub signer: Pubkey,
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
        // Extract version from discriminator
        let version = TokenDataVersion::from_discriminator(self.discriminator)
            .map_err(|_| ProgramError::InvalidAccountData)? as u8;

        // Check if this is an ATA decompress (is_ata flag in stored TLV)
        let is_ata = self.token_data.tlv.as_ref().is_some_and(|exts| {
            exts.iter()
                .any(|e| matches!(e, ExtensionStruct::CompressedOnly(co) if co.is_ata != 0))
        });

        // For ATA decompress, derive the bump from wallet owner + mint
        // The signer is the wallet owner for ATAs
        let ata_bump = if is_ata {
            let (_, bump) =
                derive_associated_token_account(&self.signer, &self.token_data.mint.into());
            bump
        } else {
            0
        };

        // Insert signer (wallet owner, delegate, or permanent delegate) as a signer account
        let owner_index = packed_accounts.insert_or_get_config(self.signer, true, false);

        // Convert TLV extensions from state format to instruction format
        let is_frozen = self.token_data.state == AccountState::Frozen as u8;
        let tlv: Option<Vec<ExtensionInstructionData>> =
            self.token_data.tlv.as_ref().map(|extensions| {
                extensions
                    .iter()
                    .filter_map(|ext| match ext {
                        ExtensionStruct::CompressedOnly(compressed_only) => {
                            Some(ExtensionInstructionData::CompressedOnly(
                                CompressedOnlyExtensionInstructionData {
                                    delegated_amount: compressed_only.delegated_amount,
                                    withheld_transfer_fee: compressed_only.withheld_transfer_fee,
                                    is_frozen,
                                    compression_index: 0,
                                    is_ata: compressed_only.is_ata != 0,
                                    bump: ata_bump,
                                    owner_index,
                                },
                            ))
                        }
                        _ => None,
                    })
                    .collect()
            });

        // Clone tlv for passing to Transfer2Inputs.in_tlv
        let in_tlv = tlv.clone().map(|t| vec![t]);
        let amount: u64 = self.token_data.amount;
        let indices = pack_for_decompress_full_with_ata(
            &self.token_data.into(),
            &tree_info,
            self.destination,
            &mut packed_accounts,
            tlv,
            version,
            is_ata,
        );
        // Build CTokenAccount2 with decompress operation
        let mut token_account = CTokenAccount2::new(vec![indices.source])
            .map_err(|_| ProgramError::InvalidAccountData)?;
        token_account
            .decompress(amount, indices.destination_index)
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
            in_tlv,
            ..Default::default()
        };

        create_transfer2_instruction(inputs).map_err(ProgramError::from)
    }
}
