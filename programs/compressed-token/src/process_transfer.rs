use crate::{
    constants::{BUMP_CPI_AUTHORITY, NOT_FROZEN, TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR},
    spl_compression::process_compression_or_decompression,
    token_data::{AccountState, TokenData},
    ErrorCode, TransferInstruction,
};
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{prelude::*, solana_program::program_error::ProgramError, AnchorDeserialize};
use light_hasher::Poseidon;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        accounts::{InvokeAccounts, SignerAccounts},
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
            PackedMerkleContext,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

/// Process a token transfer instruction
/// build inputs -> sum check -> build outputs -> add token data to inputs -> invoke cpi
/// 1.  Unpack compressed input accounts and input token data, this uses
///     standardized signer / delegate and will fail in proof verification in
///     case either is invalid.
/// 2.  Check that compressed accounts are of same mint.
/// 3.  Check that sum of input compressed accounts is equal to sum of output
///     compressed accounts
/// 4.  create_output_compressed_accounts
/// 5.  Serialize and add token_data data to in compressed_accounts.
/// 6.  Invoke light_system_program::execute_compressed_transaction.
pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    bench_sbf_start!("t_deserialize");
    let inputs: CompressedTokenInstructionDataTransfer =
        CompressedTokenInstructionDataTransfer::deserialize(&mut inputs.as_slice())?;
    bench_sbf_end!("t_deserialize");
    bench_sbf_start!("t_context_and_check_sig");
    let (mut compressed_input_accounts, input_token_data, input_lamports) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<NOT_FROZEN>(
            &ctx.accounts.authority.key(),
            &inputs.delegated_transfer,
            ctx.remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    bench_sbf_end!("t_context_and_check_sig");
    bench_sbf_start!("t_sum_check");
    sum_check(
        &input_token_data,
        &inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.amount)
            .collect::<Vec<u64>>(),
        inputs.compress_or_decompress_amount.as_ref(),
        inputs.is_compress,
    )?;
    bench_sbf_end!("t_sum_check");
    bench_sbf_start!("t_process_compression");
    if inputs.compress_or_decompress_amount.is_some() {
        process_compression_or_decompression(&inputs, &ctx)?;
    }
    bench_sbf_end!("t_process_compression");
    bench_sbf_start!("t_create_output_compressed_accounts");
    let hashed_mint = match hash_to_bn254_field_size_be(&inputs.mint.to_bytes()) {
        Some(hashed_mint) => hashed_mint.0,
        None => return err!(ErrorCode::HashToFieldError),
    };

    let mut output_compressed_accounts = vec![
        OutputCompressedAccountWithPackedContext::default();
        inputs.output_compressed_accounts.len()
    ];

    // If delegate is signer of the transaction determine whether there is a
    // change account which remains delegated and mark its position.
    let (is_delegate, delegate) = if let Some(delegated_transfer) = inputs.delegated_transfer {
        let mut vec = vec![false; inputs.output_compressed_accounts.len()];
        if let Some(index) = delegated_transfer.delegate_change_account_index {
            vec[index as usize] = true;
        } else {
            return err!(crate::ErrorCode::InvalidDelegateIndex);
        }
        (Some(vec), Some(ctx.accounts.authority.key()))
    } else {
        (None, None)
    };
    inputs.output_compressed_accounts.iter().for_each(|data| {
        if data.tlv.is_some() {
            unimplemented!("Tlv is unimplemented");
        }
    });
    let output_lamports = create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.owner)
            .collect::<Vec<Pubkey>>()
            .as_slice(),
        delegate,
        is_delegate,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data: &PackedTokenTransferOutputData| data.amount)
            .collect::<Vec<u64>>()
            .as_slice(),
        Some(
            inputs
                .output_compressed_accounts
                .iter()
                .map(|data: &PackedTokenTransferOutputData| data.lamports)
                .collect::<Vec<Option<u64>>>(),
        ),
        &hashed_mint,
        &inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.merkle_tree_index)
            .collect::<Vec<u8>>(),
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    bench_sbf_start!("t_add_token_data_to_input_compressed_accounts");
    if !compressed_input_accounts.is_empty() {
        add_token_data_to_input_compressed_accounts::<false>(
            &mut compressed_input_accounts,
            input_token_data.as_slice(),
            &hashed_mint,
        )?;
    }
    bench_sbf_end!("t_add_token_data_to_input_compressed_accounts");

    // If input and output lamports are unbalanced create a change account
    // without token data.
    let change_lamports = input_lamports - output_lamports;
    if change_lamports > 0 {
        let new_len = output_compressed_accounts.len() + 1;
        // Resize vector to new_len so that no unnecessary memory is allocated.
        // (Rust doubles the size of the vector when pushing to a full vector.)
        output_compressed_accounts.resize(
            new_len,
            OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: ctx.accounts.authority.key(),
                    lamports: change_lamports,
                    data: None,
                    address: None,
                },
                merkle_tree_index: inputs.output_compressed_accounts[0].merkle_tree_index,
            },
        );
    }

    cpi_execute_compressed_transaction_transfer(
        ctx.accounts,
        compressed_input_accounts,
        &output_compressed_accounts,
        inputs.proof,
        inputs.cpi_context,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.remaining_accounts,
    )
}

/// Creates output compressed accounts.
/// Steps:
/// 1. Allocate memory for token data.
/// 2. Create, hash and serialize token data.
/// 3. Create compressed account data.
/// 4. Repeat for every pubkey.
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_accounts(
    output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
    mint_pubkey: Pubkey,
    pubkeys: &[Pubkey],
    delegate: Option<Pubkey>,
    is_delegate: Option<Vec<bool>>,
    amounts: &[u64],
    lamports: Option<Vec<Option<u64>>>,
    hashed_mint: &[u8; 32],
    merkle_tree_indices: &[u8],
) -> Result<u64> {
    let mut sum_lamports = 0;
    let hashed_delegate_store = if let Some(delegate) = delegate {
        hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
            .unwrap()
            .0
    } else {
        [0u8; 32]
    };
    for (i, (owner, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        let (delegate, hashed_delegate) = if is_delegate
            .as_ref()
            .map(|is_delegate| is_delegate[i])
            .unwrap_or(false)
        {
            (
                delegate.as_ref().map(|delegate_pubkey| *delegate_pubkey),
                Some(&hashed_delegate_store),
            )
        } else {
            (None, None)
        };
        // 107/75 =
        //      32      mint
        // +    32      owner
        // +    8       amount
        // +    1 + 32  option + delegate (optional)
        // +    1       state
        let capacity = if delegate.is_some() { 107 } else { 75 };
        let mut token_data_bytes = Vec::with_capacity(capacity);
        // 1,000 CU token data and serialize
        let token_data = TokenData {
            mint: mint_pubkey,
            owner: *owner,
            amount: *amount,
            delegate,
            state: AccountState::Initialized,
            tlv: None,
        };
        token_data.serialize(&mut token_data_bytes).unwrap();
        bench_sbf_start!("token_data_hash");
        let hashed_owner = hash_to_bn254_field_size_be(owner.as_ref()).unwrap().0;
        let amount_bytes = amount.to_le_bytes();
        let data_hash = TokenData::hash_with_hashed_values::<Poseidon>(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate,
        )
        .map_err(ProgramError::from)?;
        let data: CompressedAccountData = CompressedAccountData {
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data: token_data_bytes,
            data_hash,
        };

        bench_sbf_end!("token_data_hash");
        let lamports = lamports
            .as_ref()
            .and_then(|lamports| lamports[i])
            .unwrap_or(0);
        sum_lamports += lamports;
        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID,
                lamports,
                data: Some(data),
                address: None,
            },
            merkle_tree_index: merkle_tree_indices[i],
        };
    }
    Ok(sum_lamports)
}

/// Create output compressed accounts
/// 1. enforces discriminator
/// 2. hashes token data
pub fn add_token_data_to_input_compressed_accounts<const FROZEN_INPUTS: bool>(
    input_compressed_accounts_with_merkle_context: &mut [PackedCompressedAccountWithMerkleContext],
    input_token_data: &[TokenData],
    hashed_mint: &[u8; 32],
) -> Result<()> {
    let hashed_owner = hash_to_bn254_field_size_be(&input_token_data[0].owner.to_bytes())
        .unwrap()
        .0;
    for (i, compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter_mut()
        .enumerate()
    {
        let mut data = Vec::new();
        input_token_data[i].serialize(&mut data)?;
        let amount = input_token_data[i].amount.to_le_bytes();
        let delegate_store;
        let hashed_delegate = if let Some(delegate) = input_token_data[i].delegate {
            delegate_store = hash_to_bn254_field_size_be(&delegate.to_bytes()).unwrap().0;
            Some(&delegate_store)
        } else {
            None
        };
        compressed_account_with_context.compressed_account.data = if !FROZEN_INPUTS {
            Some(CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data,
                data_hash: TokenData::hash_with_hashed_values::<Poseidon>(
                    hashed_mint,
                    &hashed_owner,
                    &amount,
                    &hashed_delegate,
                )
                .map_err(ProgramError::from)?,
            })
        } else {
            Some(CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data,
                data_hash: TokenData::hash_frozen_with_hashed_values::<Poseidon>(
                    hashed_mint,
                    &hashed_owner,
                    &amount,
                    &hashed_delegate,
                )
                .map_err(ProgramError::from)?,
            })
        };
    }
    Ok(())
}

/// Get static cpi signer seeds
pub fn get_cpi_signer_seeds() -> [&'static [u8]; 2] {
    let bump: &[u8; 1] = &[BUMP_CPI_AUTHORITY];
    let seeds: [&'static [u8]; 2] = [CPI_AUTHORITY_PDA_SEED, bump];
    seeds
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn cpi_execute_compressed_transaction_transfer<
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    ctx: &A,
    input_compressed_accounts_with_merkle_context: Vec<PackedCompressedAccountWithMerkleContext>,
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    proof: Option<CompressedProof>,
    cpi_context: Option<CompressedCpiContext>,
    cpi_authority_pda: AccountInfo<'info>,
    system_program_account_info: AccountInfo<'info>,
    invoking_program_account_info: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    bench_sbf_start!("t_cpi_prep");

    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];

    let cpi_context_account = cpi_context.map(|cpi_context| {
        remaining_accounts[cpi_context.cpi_context_account_index as usize].to_account_info()
    });
    let inputs_struct = light_system_program::invoke_cpi::instruction::InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: output_compressed_accounts.to_vec(),
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: signer_seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context,
    };
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).map_err(ProgramError::from)?;

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.get_fee_payer().to_account_info(),
        authority: cpi_authority_pda,
        registered_program_pda: ctx.get_registered_program_pda().to_account_info(),
        noop_program: ctx.get_noop_program().to_account_info(),
        account_compression_authority: ctx.get_account_compression_authority().to_account_info(),
        account_compression_program: ctx.get_account_compression_program().to_account_info(),
        invoking_program: invoking_program_account_info,
        system_program: ctx.get_system_program().to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        cpi_context_account,
    };
    let mut cpi_ctx =
        CpiContext::new_with_signer(system_program_account_info, cpi_accounts, signer_seeds_ref);

    cpi_ctx.remaining_accounts = remaining_accounts.to_vec();
    bench_sbf_end!("t_cpi_prep");

    bench_sbf_start!("t_invoke_cpi");
    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    bench_sbf_end!("t_invoke_cpi");

    Ok(())
}

pub fn sum_check(
    input_token_data_elements: &[TokenData],
    output_amounts: &[u64],
    compress_or_decompress_amount: Option<&u64>,
    is_compress: bool,
) -> Result<()> {
    let mut sum: u64 = 0;
    for input_token_data in input_token_data_elements.iter() {
        sum = sum
            .checked_add(input_token_data.amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    if let Some(compress_or_decompress_amount) = compress_or_decompress_amount {
        if is_compress {
            sum = sum
                .checked_add(*compress_or_decompress_amount)
                .ok_or(ProgramError::ArithmeticOverflow)
                .map_err(|_| ErrorCode::ComputeCompressSumFailed)?;
        } else {
            sum = sum
                .checked_sub(*compress_or_decompress_amount)
                .ok_or(ProgramError::ArithmeticOverflow)
                .map_err(|_| ErrorCode::ComputeDecompressSumFailed)?;
        }
    }

    for amount in output_amounts.iter() {
        sum = sum
            .checked_sub(*amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
    }

    if sum == 0 {
        Ok(())
    } else {
        Err(ErrorCode::SumCheckFailed.into())
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InputTokenDataWithContext {
    pub amount: u64,
    pub delegate_index: Option<u8>,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    pub lamports: Option<u64>,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

/// Struct to provide the owner when the delegate is signer of the transaction.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DelegatedTransfer {
    pub owner: Pubkey,
    /// Index of change compressed account in output compressed accounts. In
    /// case that the delegate didn't spend the complete delegated compressed
    /// account balance the change compressed account will be delegated to her
    /// as well.
    pub delegate_change_account_index: Option<u8>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub mint: Pubkey,
    /// Is required if the signer is delegate,
    /// -> delegate is authority account,
    /// owner = Some(owner) is the owner of the token account.
    pub delegated_transfer: Option<DelegatedTransfer>,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    pub is_compress: bool,
    pub compress_or_decompress_amount: Option<u64>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub lamports_change_account_merkle_tree_index: Option<u8>,
}

pub fn get_input_compressed_accounts_with_merkle_context_and_check_signer<const IS_FROZEN: bool>(
    signer: &Pubkey,
    signer_is_delegate: &Option<DelegatedTransfer>,
    remaining_accounts: &[AccountInfo<'_>],
    input_token_data_with_context: &[InputTokenDataWithContext],
    mint: &Pubkey,
) -> Result<(
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<TokenData>,
    u64,
)> {
    // Collect the total number of lamports to check whether inputs and outputs
    // are unbalanced. If unbalanced create a non token compressed change
    // account owner by the sender.
    let mut sum_lamports = 0;
    let mut input_compressed_accounts_with_merkle_context: Vec<
        PackedCompressedAccountWithMerkleContext,
    > = Vec::<PackedCompressedAccountWithMerkleContext>::with_capacity(
        input_token_data_with_context.len(),
    );
    let mut input_token_data_vec: Vec<TokenData> =
        Vec::with_capacity(input_token_data_with_context.len());
    let owner = if let Some(signer_is_delegate) = signer_is_delegate {
        signer_is_delegate.owner
    } else {
        *signer
    };
    for input_token_data in input_token_data_with_context.iter() {
        // This is a check for convenience to throw a meaningful error.
        // The actual security results from the proof verification.
        if signer_is_delegate.is_some()
            && input_token_data.delegate_index.is_some()
            && *signer
                != remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
        {
            msg!(
                "signer {:?} != delegate in remaining accounts {:?}",
                signer,
                remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
            );
            msg!(
                "delegate index {:?}",
                input_token_data.delegate_index.unwrap() as usize
            );
            return err!(ErrorCode::DelegateSignerCheckFailed);
        } else if signer_is_delegate.is_some() && input_token_data.delegate_index.is_none() {
            msg!("Signer is delegate but token data has no delegate.");
            return err!(ErrorCode::DelegateSignerCheckFailed);
        }
        let compressed_account = CompressedAccount {
            owner: crate::ID,
            lamports: input_token_data.lamports.unwrap_or_default(),
            data: None,
            address: None,
        };
        sum_lamports += compressed_account.lamports;
        let state = if IS_FROZEN {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };
        if input_token_data.tlv.is_some() {
            unimplemented!("Tlv is unimplemented.");
        }
        let token_data = TokenData {
            mint: *mint,
            owner,
            amount: input_token_data.amount,
            delegate: input_token_data.delegate_index.map(|_| {
                remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
            }),
            state,
            tlv: None,
        };
        input_token_data_vec.push(token_data);
        input_compressed_accounts_with_merkle_context.push(
            PackedCompressedAccountWithMerkleContext {
                compressed_account,
                merkle_context: input_token_data.merkle_context,
                root_index: input_token_data.root_index,
            },
        );
    }
    Ok((
        input_compressed_accounts_with_merkle_context,
        input_token_data_vec,
        sum_lamports,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct PackedTokenTransferOutputData {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree_index: u8,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTransferOutputData {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree: Pubkey,
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

#[cfg(not(target_os = "solana"))]
pub mod transfer_sdk {
    use std::collections::HashMap;

    use anchor_lang::{AnchorSerialize, Id, InstructionData, ToAccountMetas};
    use anchor_spl::token::Token;
    use light_system_program::{
        invoke::processor::CompressedProof,
        sdk::compressed_account::{CompressedAccount, MerkleContext, PackedMerkleContext},
    };
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    use crate::{token_data::TokenData, CompressedTokenInstructionDataTransfer};
    use anchor_lang::error_code;

    use super::{
        DelegatedTransfer, InputTokenDataWithContext, PackedTokenTransferOutputData,
        TokenTransferOutputData,
    };

    #[error_code]
    pub enum TransferSdkError {
        #[msg("Signer check failed")]
        SignerCheckFailed,
        #[msg("Create transfer instruction failed")]
        CreateTransferInstructionFailed,
        #[msg("Account not found")]
        AccountNotFound,
        #[msg("Serialization error")]
        SerializationError,
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer_instruction(
        fee_payer: &Pubkey,
        owner: &Pubkey,
        input_merkle_context: &[MerkleContext],
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        mint: Pubkey,
        delegate: Option<Pubkey>,
        is_compress: bool,
        compress_or_decompress_amount: Option<u64>,
        token_pool_pda: Option<Pubkey>,
        compress_or_decompress_token_account: Option<Pubkey>,
        sort: bool,
        delegate_change_account_index: Option<u8>,
        lamports_change_account_merkle_tree: Option<Pubkey>,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, mut inputs_struct) = create_inputs_and_remaining_accounts(
            input_token_data,
            input_compressed_accounts,
            input_merkle_context,
            delegate,
            output_compressed_accounts,
            root_indices,
            proof,
            mint,
            is_compress,
            compress_or_decompress_amount,
            delegate_change_account_index,
            lamports_change_account_merkle_tree,
        );
        if sort {
            inputs_struct
                .output_compressed_accounts
                .sort_by_key(|data| data.merkle_tree_index);
        }
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut inputs = Vec::new();
        CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs)
            .map_err(|_| TransferSdkError::SerializationError)?;

        let (cpi_authority_pda, _) = crate::process_transfer::get_cpi_authority_pda();
        let instruction_data = crate::instruction::Transfer { inputs };
        let authority = if let Some(delegate) = delegate {
            delegate
        } else {
            *owner
        };

        let accounts = crate::accounts::TransferInstruction {
            fee_payer: *fee_payer,
            authority,
            cpi_authority_pda,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
            token_pool_pda,
            compress_or_decompress_token_account,
            token_program: token_pool_pda.map(|_| Token::id()),
            system_program: solana_sdk::system_program::ID,
        };

        Ok(Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_inputs_and_remaining_accounts_checked(
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        input_merkle_context: &[MerkleContext],
        owner_if_delegate_is_signer: Option<Pubkey>,
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        mint: Pubkey,
        owner: &Pubkey,
        is_compress: bool,
        compress_or_decompress_amount: Option<u64>,
        delegate_change_account_index: Option<u8>,
        lamports_change_account_merkle_tree: Option<Pubkey>,
    ) -> Result<
        (
            HashMap<Pubkey, usize>,
            CompressedTokenInstructionDataTransfer,
        ),
        TransferSdkError,
    > {
        for token_data in input_token_data {
            // convenience signer check to throw a meaningful error
            if token_data.owner != *owner {
                println!(
                    "owner: {:?}, token_data.owner: {:?}",
                    owner, token_data.owner
                );
                return Err(TransferSdkError::SignerCheckFailed);
            }
        }
        let (remaining_accounts, compressed_accounts_ix_data) =
            create_inputs_and_remaining_accounts(
                input_token_data,
                input_compressed_accounts,
                input_merkle_context,
                owner_if_delegate_is_signer,
                output_compressed_accounts,
                root_indices,
                proof,
                mint,
                is_compress,
                compress_or_decompress_amount,
                delegate_change_account_index,
                lamports_change_account_merkle_tree,
            );
        Ok((remaining_accounts, compressed_accounts_ix_data))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_inputs_and_remaining_accounts(
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        input_merkle_context: &[MerkleContext],
        delegate: Option<Pubkey>,
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        mint: Pubkey,
        is_compress: bool,
        compress_or_decompress_amount: Option<u64>,
        delegate_change_account_index: Option<u8>,
        lamports_change_account_merkle_tree: Option<Pubkey>,
    ) -> (
        HashMap<Pubkey, usize>,
        CompressedTokenInstructionDataTransfer,
    ) {
        let mut additonal_accounts = Vec::new();
        if let Some(delegate) = delegate {
            additonal_accounts.push(delegate);
            for account in input_token_data.iter() {
                if account.delegate.is_some() && delegate != account.delegate.unwrap() {
                    println!("delegate: {:?}", delegate);
                    println!("account.delegate: {:?}", account.delegate.unwrap());
                    panic!("Delegate is not the same as the signer");
                }
            }
        }
        let lamports_change_account_merkle_tree_index = if let Some(
            lamports_change_account_merkle_tree,
        ) = lamports_change_account_merkle_tree
        {
            additonal_accounts.push(lamports_change_account_merkle_tree);
            Some(additonal_accounts.len() as u8 - 1)
        } else {
            None
        };
        let (remaining_accounts, input_token_data_with_context, _output_compressed_accounts) =
            create_input_output_and_remaining_accounts(
                additonal_accounts.as_slice(),
                input_token_data,
                input_compressed_accounts,
                input_merkle_context,
                root_indices,
                output_compressed_accounts,
            );
        let delegated_transfer = if delegate.is_some() {
            let delegated_transfer = DelegatedTransfer {
                owner: input_token_data[0].owner,
                delegate_change_account_index,
            };
            Some(delegated_transfer)
        } else {
            None
        };
        let inputs_struct = CompressedTokenInstructionDataTransfer {
            output_compressed_accounts: _output_compressed_accounts.to_vec(),
            proof: proof.clone(),
            input_token_data_with_context,
            delegated_transfer,
            mint,
            is_compress,
            compress_or_decompress_amount,
            cpi_context: None,
            lamports_change_account_merkle_tree_index,
        };

        (remaining_accounts, inputs_struct)
    }

    pub fn create_input_output_and_remaining_accounts(
        additional_accounts: &[Pubkey],
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        input_merkle_context: &[MerkleContext],
        root_indices: &[u16],
        output_compressed_accounts: &[TokenTransferOutputData],
    ) -> (
        HashMap<Pubkey, usize>,
        Vec<InputTokenDataWithContext>,
        Vec<PackedTokenTransferOutputData>,
    ) {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        let mut index = 0;
        for account in additional_accounts {
            match remaining_accounts.get(account) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*account, index);
                    index += 1;
                }
            };
        }
        let mut input_token_data_with_context: Vec<InputTokenDataWithContext> = Vec::new();

        for (i, token_data) in input_token_data.iter().enumerate() {
            match remaining_accounts.get(&input_merkle_context[i].merkle_tree_pubkey) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(input_merkle_context[i].merkle_tree_pubkey, index);
                    index += 1;
                }
            };
            let delegate_index = match token_data.delegate {
                Some(delegate) => match remaining_accounts.get(&delegate) {
                    Some(delegate_index) => Some(*delegate_index as u8),
                    None => {
                        remaining_accounts.insert(delegate, index);
                        index += 1;
                        Some((index - 1) as u8)
                    }
                },
                None => None,
            };
            let lamports = if input_compressed_accounts[i].lamports != 0 {
                Some(input_compressed_accounts[i].lamports)
            } else {
                None
            };
            let token_data_with_context = InputTokenDataWithContext {
                amount: token_data.amount,
                delegate_index,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: *remaining_accounts
                        .get(&input_merkle_context[i].merkle_tree_pubkey)
                        .unwrap() as u8,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: input_merkle_context[i].leaf_index,
                    queue_index: None,
                },
                root_index: root_indices[i],
                lamports,
                tlv: None,
            };
            input_token_data_with_context.push(token_data_with_context);
        }
        for (i, _) in input_token_data.iter().enumerate() {
            match remaining_accounts.get(&input_merkle_context[i].nullifier_queue_pubkey) {
                Some(_) => {}
                None => {
                    remaining_accounts
                        .insert(input_merkle_context[i].nullifier_queue_pubkey, index);
                    index += 1;
                }
            };
            input_token_data_with_context[i]
                .merkle_context
                .nullifier_queue_pubkey_index = *remaining_accounts
                .get(&input_merkle_context[i].nullifier_queue_pubkey)
                .unwrap() as u8;
        }
        let mut _output_compressed_accounts: Vec<PackedTokenTransferOutputData> =
            Vec::with_capacity(output_compressed_accounts.len());
        for (i, mt) in output_compressed_accounts.iter().enumerate() {
            match remaining_accounts.get(&mt.merkle_tree) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(mt.merkle_tree, index);
                    index += 1;
                }
            };
            _output_compressed_accounts.push(PackedTokenTransferOutputData {
                owner: output_compressed_accounts[i].owner,
                amount: output_compressed_accounts[i].amount,
                lamports: output_compressed_accounts[i].lamports,
                merkle_tree_index: *remaining_accounts.get(&mt.merkle_tree).unwrap() as u8,
                tlv: None,
            });
        }
        (
            remaining_accounts,
            input_token_data_with_context,
            _output_compressed_accounts,
        )
    }

    pub fn to_account_metas(remaining_accounts: HashMap<Pubkey, usize>) -> Vec<AccountMeta> {
        let mut remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, i)| {
                (
                    AccountMeta {
                        pubkey: *k,
                        is_signer: false,
                        is_writable: true,
                    },
                    *i,
                )
            })
            .collect::<Vec<(AccountMeta, usize)>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<AccountMeta>>();
        remaining_accounts
    }
}

#[cfg(test)]
mod test {
    use crate::token_data::AccountState;

    use super::*;

    #[test]
    fn test_sum_check() {
        // SUCCEED: no relay fee, compression
        sum_check_test(&[100, 50], &[150], None, false).unwrap();
        sum_check_test(&[75, 25, 25], &[25, 25, 25, 25, 12, 13], None, false).unwrap();

        // FAIL: no relay fee, compression
        sum_check_test(&[100, 50], &[150 + 1], None, false).unwrap_err();
        sum_check_test(&[100, 50], &[150 - 1], None, false).unwrap_err();
        sum_check_test(&[100, 50], &[], None, false).unwrap_err();
        sum_check_test(&[], &[100, 50], None, false).unwrap_err();

        // SUCCEED: empty
        sum_check_test(&[], &[], None, true).unwrap();
        sum_check_test(&[], &[], None, false).unwrap();
        // FAIL: empty
        sum_check_test(&[], &[], Some(1), false).unwrap_err();
        sum_check_test(&[], &[], Some(1), true).unwrap_err();

        // SUCCEED: with compress
        sum_check_test(&[100], &[123], Some(23), true).unwrap();
        sum_check_test(&[], &[150], Some(150), true).unwrap();
        // FAIL: compress
        sum_check_test(&[], &[150], Some(150 - 1), true).unwrap_err();
        sum_check_test(&[], &[150], Some(150 + 1), true).unwrap_err();

        // SUCCEED: with decompress
        sum_check_test(&[100, 50], &[100], Some(50), false).unwrap();
        sum_check_test(&[100, 50], &[], Some(150), false).unwrap();
        // FAIL: decompress
        sum_check_test(&[100, 50], &[], Some(150 - 1), false).unwrap_err();
        sum_check_test(&[100, 50], &[], Some(150 + 1), false).unwrap_err();
    }

    fn sum_check_test(
        input_amounts: &[u64],
        output_amounts: &[u64],
        compress_or_decompress_amount: Option<u64>,
        is_compress: bool,
    ) -> Result<()> {
        let mut inputs = Vec::new();
        for i in input_amounts.iter() {
            inputs.push(TokenData {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                delegate: None,
                state: AccountState::Initialized,
                amount: *i,
                tlv: None,
            });
        }
        let ref_amount;
        let compress_or_decompress_amount = match compress_or_decompress_amount {
            Some(amount) => {
                ref_amount = amount;
                Some(&ref_amount)
            }
            None => None,
        };
        sum_check(
            inputs.as_slice(),
            &output_amounts,
            compress_or_decompress_amount,
            is_compress,
        )
    }
}
