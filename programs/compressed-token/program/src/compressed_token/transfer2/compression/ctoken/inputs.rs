use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_interface::instructions::{
    extensions::ZExtensionInstructionData,
    transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
        ZMultiInputTokenDataWithContext, ZMultiTokenTransferOutputData,
    },
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::{extensions::MintExtensionChecks, MAX_COMPRESSIONS};

/// Decompress-specific inputs from the input compressed account.
/// Only required for decompression with CompressedOnly extension.
pub struct DecompressCompressOnlyInputs<'a> {
    /// Input TLV for decompress operations (from the input compressed account being consumed).
    pub tlv: &'a [ZExtensionInstructionData<'a>],
    /// The input compressed token data being consumed.
    pub input_token_data: &'a ZMultiInputTokenDataWithContext<'a>,
}

impl<'a> DecompressCompressOnlyInputs<'a> {
    /// Extract decompress inputs for CompressedOnly extension state transfer.
    ///
    /// Extracts TLV and input_token_data from the input compressed account for decompress
    /// operations. Also validates compression-input consistency (mode and mint match).
    #[inline(always)]
    pub fn try_extract(
        compression: &ZCompression,
        compression_index: usize,
        compression_to_input: &[Option<u8>; MAX_COMPRESSIONS],
        inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
    ) -> Result<Option<Self>, ProgramError> {
        let Some(input_idx) = compression_to_input[compression_index] else {
            return Ok(None);
        };
        let idx = input_idx as usize;

        // Compression must be Decompress mode to consume an input
        if compression.mode != ZCompressionMode::Decompress {
            msg!(
                "Input linked to non-decompress compression at index {}",
                compression_index
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        // Validate mint matches between compression and input
        let input_token_data = inputs
            .in_token_data
            .get(idx)
            .ok_or(ProgramError::InvalidInstructionData)?;
        if compression.mint != input_token_data.mint {
            msg!(
                "Mint mismatch between compression and input at index {}",
                compression_index
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        // Get TLV slice (use empty slice if not present)
        let tlv = inputs
            .in_tlv
            .as_ref()
            .and_then(|tlvs| tlvs.get(idx))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        Ok(Some(DecompressCompressOnlyInputs {
            tlv,
            input_token_data,
        }))
    }
}

/// Compress and close specific inputs
pub struct CompressAndCloseInputs<'a> {
    pub destination: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub compressed_token_account: Option<&'a ZMultiTokenTransferOutputData<'a>>,
    pub tlv: Option<&'a [ZExtensionInstructionData<'a>]>,
}

/// Input struct for ctoken compression/decompression operations
pub struct CTokenCompressionInputs<'a> {
    pub authority: Option<&'a AccountInfo>,
    pub compress_and_close_inputs: Option<CompressAndCloseInputs<'a>>,
    pub amount: u64,
    pub mint: Pubkey,
    pub token_account_info: &'a AccountInfo,
    pub mode: ZCompressionMode,
    pub packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    /// Mint extension checks result (permanent delegate, transfer fee info).
    /// Used to validate permanent delegate authority for compression operations.
    pub mint_checks: Option<MintExtensionChecks>,
    /// Decompress-specific inputs (TLV, delegate, owner from input compressed account).
    pub decompress_inputs: Option<DecompressCompressOnlyInputs<'a>>,
}

impl<'a> CTokenCompressionInputs<'a> {
    /// Constructor for compression operations from Transfer2 instruction
    pub fn from_compression(
        compression: &ZCompression,
        token_account_info: &'a AccountInfo,
        inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
        mint_checks: Option<MintExtensionChecks>,
        decompress_inputs: Option<DecompressCompressOnlyInputs<'a>>,
    ) -> Result<Self, anchor_lang::prelude::ProgramError> {
        let authority_account = if compression.mode != ZCompressionMode::Decompress {
            Some(packed_accounts.get_u8(
                compression.authority,
                "process_ctoken_compression: authority",
            )?)
        } else {
            // For decompress we don't need a signer check here, -> no authority required.
            None
        };

        let mint_account = *packed_accounts
            .get_u8(compression.mint, "process_ctoken_compression: token mint")?
            .key();

        let compress_and_close_inputs = if compression.mode == ZCompressionMode::CompressAndClose {
            Some(CompressAndCloseInputs {
                destination: packed_accounts.get_u8(
                    compression.get_destination_index()?,
                    "process_ctoken_compression: destination",
                )?,
                rent_sponsor: packed_accounts.get_u8(
                    compression.get_rent_sponsor_index()?,
                    "process_ctoken_compression: rent_sponsor",
                )?,
                compressed_token_account: inputs
                    .out_token_data
                    .get(compression.get_compressed_token_account_index()? as usize),
                tlv: inputs
                    .out_tlv
                    .as_ref()
                    .and_then(|v| {
                        v.get(compression.get_compressed_token_account_index().ok()? as usize)
                    })
                    .map(|data| data.as_slice()),
            })
        } else {
            None
        };

        Ok(Self {
            authority: authority_account,
            compress_and_close_inputs,
            amount: (*compression.amount).into(),
            mint: mint_account,
            token_account_info,
            mode: compression.mode.clone(),
            packed_accounts,
            mint_checks,
            decompress_inputs,
        })
    }

    pub fn mint_ctokens(
        amount: u64,
        mint: Pubkey,
        token_account_info: &'a AccountInfo,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    ) -> Self {
        Self {
            authority: None,
            compress_and_close_inputs: None,
            amount,
            mint,
            token_account_info,
            mode: ZCompressionMode::Decompress,
            packed_accounts,
            mint_checks: None,
            decompress_inputs: None,
        }
    }
}
