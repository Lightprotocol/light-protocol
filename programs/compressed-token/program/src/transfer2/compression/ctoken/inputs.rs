use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_interface::instructions::{
    extensions::ZExtensionInstructionData,
    transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
        ZMultiTokenTransferOutputData,
    },
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::extensions::MintExtensionChecks;

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
    /// Input TLV for decompress operations (from the input compressed account being consumed).
    pub input_tlv: Option<&'a [ZExtensionInstructionData<'a>]>,
    /// Delegate pubkey from input compressed account (for decompress extension state transfer).
    pub input_delegate: Option<&'a AccountInfo>,
}

impl<'a> CTokenCompressionInputs<'a> {
    /// Constructor for compression operations from Transfer2 instruction
    pub fn from_compression(
        compression: &ZCompression,
        token_account_info: &'a AccountInfo,
        inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
        mint_checks: Option<MintExtensionChecks>,
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

        // For Decompress mode, find matching input by mint index and extract TLV and delegate
        let (input_tlv, input_delegate) = if compression.mode == ZCompressionMode::Decompress {
            // Find the input compressed account that matches this decompress by mint index
            let matching_input_index = inputs
                .in_token_data
                .iter()
                .position(|input| input.mint == compression.mint);

            let input_tlv = matching_input_index.and_then(|idx| {
                inputs
                    .in_tlv
                    .as_ref()
                    .and_then(|tlvs| tlvs.get(idx))
                    .map(|v| v.as_slice())
            });

            let input_delegate = matching_input_index.and_then(|idx| {
                let input = inputs.in_token_data.get(idx)?;
                if input.has_delegate() {
                    packed_accounts
                        .get_u8(input.delegate, "input delegate")
                        .ok()
                } else {
                    None
                }
            });

            (input_tlv, input_delegate)
        } else {
            (None, None)
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
            input_tlv,
            input_delegate,
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
            input_tlv: None,
            input_delegate: None,
        }
    }
}
