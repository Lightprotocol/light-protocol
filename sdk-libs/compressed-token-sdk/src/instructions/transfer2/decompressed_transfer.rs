use crate::{
    account2::CTokenAccount2, error::TokenSdkError,
    instructions::transfer2::account_metas::Transfer2AccountsMetaConfig,
};
use light_compressed_token_types::ValidityProof;
use light_ctoken_types::instructions::transfer2::{
    Compression, CompressionMode, MultiTokenTransferOutputData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::instruction::{create_transfer2_instruction, Transfer2Config, Transfer2Inputs};

pub fn create_spl_to_ctoken_transfer_instruction(
    source_spl_token_account: Pubkey,
    to: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
) -> Result<Instruction, TokenSdkError> {
    let mut packed_accounts = Vec::new();

    // Mint (index 0)
    packed_accounts.push(AccountMeta::new_readonly(mint, false));

    // Destination token account (index 1)
    packed_accounts.push(AccountMeta::new(to, false));

    // Authority for compression (index 2) - signer
    packed_accounts.push(AccountMeta::new(authority, true));

    // Source SPL token account (index 3) - writable
    packed_accounts.push(AccountMeta::new(source_spl_token_account, false));

    // Token pool PDA (index 4) - writable
    packed_accounts.push(AccountMeta::new(token_pool_pda, false));

    // SPL Token program (index 5) - needed for CPI
    packed_accounts.push(AccountMeta::new_readonly(
        Pubkey::from(light_compressed_token_types::constants::SPL_TOKEN_PROGRAM_ID),
        false,
    ));
    // println!("packed_accounts {:?}", packed_accounts);
    let wrap_spl_to_ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression {
            amount,
            mode: CompressionMode::Compress,
            mint: 0,
            source_or_recipient: 3, // index of source
            authority: 2,
            pool_account_index: 4,
            pool_index: 0, // TODO: make dynamic.
            bump: token_pool_pda_bump,
        }),
        delegate_is_set: false,
        method_used: true,
    };

    let ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint: 0,
            source_or_recipient: 1, // index of destination
            authority: 0,           // unchecked.
            pool_account_index: 0,  // unused.
            pool_index: 0,          // unused.
            bump: 0,
        }),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs following the test pattern
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
            payer,
            packed_accounts,
        ),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![wrap_spl_to_ctoken_account, ctoken_account],
    };

    // Create the actual transfer2 instruction
    create_transfer2_instruction(inputs)
}

/// Transfer SPL tokens to compressed tokens
///
/// This function creates the instruction and immediately invokes it.
/// Similar to SPL Token's transfer wrapper functions.
pub fn transfer_spl_to_ctoken<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    token_pool_pda: AccountInfo<'info>,
    token_pool_pda_bump: u8,
    token_program_authority: AccountInfo<'info>,
    spl_program: AccountInfo<'info>,
) -> Result<(), ProgramError> {
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *authority.key,
        *token_pool_pda.key,
        token_pool_pda_bump,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // let mut account_infos = remaining_accounts.to_vec();
    let account_infos = vec![
        authority.clone(),
        spl_program,
        token_program_authority,
        mint,           // Index 0: Mint
        to,             // Index 1: Destination owner
        authority,      // Index 2: Authority (signer)
        from,           // Index 3: Source SPL token account
        token_pool_pda, // Index 4: Token pool PDA
    ];

    invoke(&instruction, &account_infos)?;
    Ok(())
}

/// Transfer SPL tokens to compressed tokens via CPI signer.
///
/// This function creates the instruction and invokes it with the provided
/// signer seeds.
pub fn transfer_spl_to_ctoken_signed<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    token_pool_pda: AccountInfo<'info>,
    spl_program: AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), TokenSdkError> {
    let instruction = create_spl_to_ctoken_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *authority.key,
        *token_pool_pda.key,
        0,
    )
    .map_err(|_| TokenSdkError::MethodUsed)?;

    let account_infos = vec![
        authority.clone(),
        mint,           // Index 0: Mint
        to,             // Index 1: Destination owner
        authority,      // Index 2: Authority (signer)
        from,           // Index 3: Source SPL token account
        token_pool_pda, // Index 4: Token pool PDA
        spl_program,    // Index 5: SPL Token program
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)
        .map_err(|_| TokenSdkError::MethodUsed)?;
    Ok(())
}

pub fn create_ctoken_to_spl_transfer_instruction(
    source_ctoken_account: Pubkey,
    destination_spl_token_account: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
) -> Result<Instruction, TokenSdkError> {
    let mut packed_accounts = Vec::new();

    // Mint (index 0)
    packed_accounts.push(AccountMeta::new_readonly(mint, false));

    // Source ctoken account (index 1) - writable
    packed_accounts.push(AccountMeta::new(source_ctoken_account, false));

    // Destination SPL token account (index 2) - writable
    packed_accounts.push(AccountMeta::new(destination_spl_token_account, false));

    // Authority (index 3) - signer
    packed_accounts.push(AccountMeta::new_readonly(authority, true));

    // Token pool PDA (index 4) - writable
    packed_accounts.push(AccountMeta::new(token_pool_pda, false));

    // SPL Token program (index 5) - needed for CPI
    packed_accounts.push(AccountMeta::new_readonly(
        Pubkey::from(light_compressed_token_types::constants::SPL_TOKEN_PROGRAM_ID),
        false,
    ));

    // First operation: compress from ctoken account to pool using compress_spl
    let compress_to_pool = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress_spl(
            amount,
            0, // mint index
            1, // source ctoken account index
            3, // authority index
            4, // pool_account_index
            0, // pool_index (TODO: make dynamic)
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    // Second operation: decompress from pool to SPL token account using decompress_spl
    let decompress_to_spl = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress_spl(
            amount,
            0, // mint index
            2, // destination SPL token account index
            4, // pool_account_index
            0, // pool_index (TODO: make dynamic)
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
            payer,
            packed_accounts,
        ),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![compress_to_pool, decompress_to_spl],
    };

    // Create the actual transfer2 instruction
    create_transfer2_instruction(inputs)
}

/// Transfer compressed tokens to SPL tokens
///
/// This function creates the instruction and invokes it.
pub fn transfer_ctoken_to_spl<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    token_pool_pda: AccountInfo<'info>,
) -> Result<(), TokenSdkError> {
    let instruction = create_ctoken_to_spl_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *authority.key,
        *token_pool_pda.key,
        0,
    )
    .map_err(|_| TokenSdkError::MethodUsed)?;

    let account_infos = vec![
        authority.clone(),
        mint,           // Index 0: Mint
        to,             // Index 1: Destination owner
        authority,      // Index 2: Authority (signer)
        from,           // Index 3: Source SPL token account
        token_pool_pda, // Index 4: Token pool PDA
    ];

    invoke(&instruction, &account_infos).map_err(|_| TokenSdkError::MethodUsed)?;
    Ok(())
}

/// Transfer compressed tokens to SPL tokens via CPI signer.
///
/// This function creates the instruction and invokes it with the provided
/// signer seeds.
pub fn transfer_ctoken_to_spl_signed<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    amount: u64,
    authority: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    token_pool_pda: AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = create_ctoken_to_spl_transfer_instruction(
        *from.key,
        *to.key,
        amount,
        *authority.key,
        *mint.key,
        *authority.key,
        *token_pool_pda.key,
        0,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    let account_infos = vec![
        authority.clone(),
        mint,           // Index 0: Mint
        to,             // Index 1: Destination owner
        authority,      // Index 2: Authority (signer)
        from,           // Index 3: Source SPL token account
        token_pool_pda, // Index 4: Token pool PDA
    ];

    invoke_signed(&instruction, &account_infos, signer_seeds)?;
    Ok(())
}
