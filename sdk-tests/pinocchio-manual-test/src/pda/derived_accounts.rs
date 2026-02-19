use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    create_accounts,
    light_account_checks::{self, packed_accounts::ProgramPackedAccounts},
    LightAccount, LightAccountVariantTrait, LightFinalize, LightPreInit, LightSdkTypesError,
    PackedLightAccountVariantTrait, PdaInitParam, SharedAccounts,
};
use pinocchio::account_info::AccountInfo;

use super::{
    accounts::{CreatePda, CreatePdaParams},
    derived_state::PackedMinimalRecord,
    state::MinimalRecord,
};

// ============================================================================
// Compile-time Size Validation (800-byte limit for compressed accounts)
// ============================================================================

const _: () = {
    const COMPRESSED_SIZE: usize = 8 + MinimalRecord::INIT_SPACE;
    assert!(
        COMPRESSED_SIZE <= 800,
        "Compressed account 'MinimalRecord' exceeds 800-byte compressible account size limit"
    );
};

// ============================================================================
// Manual LightPreInit Implementation
// ============================================================================

impl LightPreInit<AccountInfo, CreatePdaParams> for CreatePda<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreatePdaParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let record = self.record;

        create_accounts::<AccountInfo, 1, 0, 0, 0, _>(
            [PdaInitParam {
                account: self.record,
            }],
            |light_config, current_slot| {
                let mut account_data = record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let record = MinimalRecord::mut_from_account_data(&mut account_data);
                record.set_decompressed(light_config, current_slot);
                Ok(())
            },
            None,
            [],
            [],
            &SharedAccounts {
                fee_payer: self.fee_payer,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::ID,
                compression_config: Some(self.compression_config),
                compressible_config: None,
                rent_sponsor: None,
                cpi_authority: None,
                system_program: None,
            },
            remaining_accounts,
        )
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl LightFinalize<AccountInfo, CreatePdaParams> for CreatePda<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreatePdaParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // No-op for PDA-only flow - compression CPI already executed in light_pre_init
        Ok(())
    }
}

// ============================================================================
// Seeds Structs
// Extracted from: seeds = [b"minimal_record", params.owner.as_ref()]
// ============================================================================

/// Seeds for MinimalRecord PDA.
/// Contains the dynamic seed values (static prefix "minimal_record" is in seed_refs).
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct MinimalRecordSeeds {
    pub owner: [u8; 32],
    pub nonce: u64,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedMinimalRecordSeeds {
    pub owner_idx: u8,
    pub nonce_bytes: [u8; 8],
    pub bump: u8,
}

// ============================================================================
// Variant Structs
// ============================================================================

/// Full variant combining seeds + data.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct MinimalRecordVariant {
    pub seeds: MinimalRecordSeeds,
    pub data: MinimalRecord,
}

/// Packed variant for efficient serialization.
/// Contains packed seeds and data with u8 indices for Pubkey deduplication.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedMinimalRecordVariant {
    pub seeds: PackedMinimalRecordSeeds,
    pub data: PackedMinimalRecord,
}

// ============================================================================
// LightAccountVariant Implementation
// ============================================================================

impl LightAccountVariantTrait<4> for MinimalRecordVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID;

    type Seeds = MinimalRecordSeeds;
    type Data = MinimalRecord;
    type Packed = PackedMinimalRecordVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"minimal_record", params.owner.as_ref(), &params.nonce.to_le_bytes()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![
            b"minimal_record".to_vec(),
            self.seeds.owner.to_vec(),
            self.seeds.nonce.to_le_bytes().to_vec(),
        ]
    }

    /// Get seed references with bump for CPI signing.
    /// Note: For unpacked variants with computed bytes (like nonce.to_le_bytes()),
    /// we cannot return references to temporaries. Use the packed variant instead.
    fn seed_refs_with_bump<'a>(&'a self, _bump_storage: &'a [u8; 1]) -> [&'a [u8]; 4] {
        // The packed variant stores nonce_bytes as [u8; 8], so it can return references.
        // This unpacked variant computes nonce.to_le_bytes() which creates a temporary.
        panic!("Use PackedMinimalRecordVariant::seed_refs_with_bump instead")
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation
// ============================================================================

impl PackedLightAccountVariantTrait<4> for PackedMinimalRecordVariant {
    type Unpacked = MinimalRecordVariant;

    const ACCOUNT_TYPE: light_account_pinocchio::AccountType =
        <MinimalRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack<AI: light_account_checks::AccountInfoTrait>(
        &self,
        accounts: &[AI],
    ) -> std::result::Result<Self::Unpacked, LightSdkTypesError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = MinimalRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

        Ok(MinimalRecordVariant {
            seeds: MinimalRecordSeeds {
                owner: owner.key(),
                nonce: u64::from_le_bytes(self.seeds.nonce_bytes),
            },
            data,
        })
    }

    fn seed_refs_with_bump<'a, AI: light_account_checks::AccountInfoTrait>(
        &'a self,
        _accounts: &'a [AI],
        _bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 4], LightSdkTypesError> {
        // PDA variants use seed_vec() in the decompression path, not seed_refs_with_bump.
        // Returning a reference to the account key requires a key_ref() method on
        // AccountInfoTrait, which is not yet available. Since this method is only
        // called for token account variants, PDA variants return an error.
        Err(LightSdkTypesError::InvalidSeeds)
    }

    fn into_in_token_data(
        &self,
        _tree_info: &light_account_pinocchio::PackedStateTreeInfo,
        _output_queue_index: u8,
    ) -> std::result::Result<
        light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext,
        LightSdkTypesError,
    > {
        Err(LightSdkTypesError::InvalidInstructionData)
    }

    fn into_in_tlv(
        &self,
    ) -> std::result::Result<
        Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>,
        LightSdkTypesError,
    > {
        Ok(None)
    }
}

// ============================================================================
// IntoVariant Implementation for Seeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_account_pinocchio::IntoVariant<MinimalRecordVariant> for MinimalRecordSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<MinimalRecordVariant, LightSdkTypesError> {
        // Deserialize the compressed data (which includes compression_info)
        let record: MinimalRecord =
            BorshDeserialize::deserialize(&mut &data[..]).map_err(|_| LightSdkTypesError::Borsh)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
            return Err(LightSdkTypesError::InvalidSeeds);
        }

        Ok(MinimalRecordVariant {
            seeds: self,
            data: record,
        })
    }
}

// ============================================================================
// Pack Implementation for MinimalRecordVariant (client-side API)
// ============================================================================

/// Implement Pack trait to allow MinimalRecordVariant to be used with `create_load_instructions`.
/// Transforms the variant into PackedLightAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_account_pinocchio::Pack<solana_instruction::AccountMeta> for MinimalRecordVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut light_account_pinocchio::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        use light_account_pinocchio::LightAccountVariantTrait;
        let (_, bump) = self.derive_pda::<AccountInfo>();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(
            crate::derived_variants::PackedLightAccountVariant::MinimalRecord {
                seeds: PackedMinimalRecordSeeds {
                    owner_idx: accounts
                        .insert_or_get(solana_pubkey::Pubkey::from(self.seeds.owner)),
                    nonce_bytes: self.seeds.nonce.to_le_bytes(),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}
