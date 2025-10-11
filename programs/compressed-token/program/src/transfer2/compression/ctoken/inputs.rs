use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::{
    instructions::transfer2::{
        ZCompressedTokenInstructionDataTransfer2, ZCompression, ZCompressionMode,
        ZMultiTokenTransferOutputData,
    },
    CTokenError,
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

/// Compress and close specific inputs
pub struct CompressAndCloseInputs<'a> {
    pub destination: &'a AccountInfo,
    pub rent_sponsor: &'a AccountInfo,
    pub compressed_token_account: &'a ZMultiTokenTransferOutputData<'a>,
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
}

impl<'a> CTokenCompressionInputs<'a> {
    /// Constructor for compression operations from Transfer2 instruction
    pub fn from_compression(
        compression: &ZCompression,
        token_account_info: &'a AccountInfo,
        inputs: &'a ZCompressedTokenInstructionDataTransfer2,
        packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
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
                    .get(compression.get_compressed_token_account_index()? as usize)
                    .ok_or(CTokenError::AccountFrozen)?,
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
        }
    }
}
