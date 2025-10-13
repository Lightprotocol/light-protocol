use account_compression::{utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount};
use anchor_lang::{
    prelude::*, solana_program::program_error::ProgramError, AnchorDeserialize, Discriminator,
};
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData, PackedMerkleContext},
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext,
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    pubkey::AsPubkey,
};
use light_ctoken_types::state::{CompressedTokenAccountState, TokenData};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_system_program::{
    account_traits::{InvokeAccounts, SignerAccounts},
    errors::SystemProgramError,
};
use light_zero_copy::num_trait::ZeroCopyNumTrait;

use crate::{
    constants::{
        BUMP_CPI_AUTHORITY, NOT_FROZEN, TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
        TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
    },
    spl_compression::process_compression_or_decompression,
    ErrorCode, TransferInstruction,
};

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
#[inline(always)]
pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: CompressedTokenInstructionDataTransfer,
) -> Result<()> {
    bench_sbf_start!("t_context_and_check_sig");
    if inputs.input_token_data_with_context.is_empty()
        && inputs.compress_or_decompress_amount.is_none()
    {
        return err!(crate::ErrorCode::NoInputTokenAccountsProvided);
    }
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
    let hashed_mint = hash_to_bn254_field_size_be(&inputs.mint.to_bytes());

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
            (Some(vec), Some(ctx.accounts.authority.key()))
        } else {
            (None, None)
        }
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
        ctx.remaining_accounts,
    )?;
    bench_sbf_end!("t_create_output_compressed_accounts");

    bench_sbf_start!("t_add_token_data_to_input_compressed_accounts");
    if !compressed_input_accounts.is_empty() {
        add_data_hash_to_input_compressed_accounts::<false>(
            &mut compressed_input_accounts,
            input_token_data.as_slice(),
            &hashed_mint,
            ctx.remaining_accounts,
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
                    owner: ctx.accounts.authority.key().into(),
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
        output_compressed_accounts,
        inputs.with_transaction_hash,
        inputs.proof,
        inputs.cpi_context,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.remaining_accounts,
    )
}
pub const BATCHED_DISCRIMINATOR: &[u8] = b"BatchMta";
pub const OUTPUT_QUEUE_DISCRIMINATOR: &[u8] = b"queueacc";

/// Helper function to determine the appropriate token account discriminator based on tree type
pub fn get_token_account_discriminator(tree_discriminator: &[u8]) -> Result<[u8; 8]> {
    match tree_discriminator {
        StateMerkleTreeAccount::DISCRIMINATOR => Ok(TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR),
        BATCHED_DISCRIMINATOR | OUTPUT_QUEUE_DISCRIMINATOR => {
            Ok(TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR)
        }
        _ => err!(SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch),
    }
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
    mint_pubkey: impl AsPubkey,
    pubkeys: &[impl AsPubkey],
    delegate: Option<Pubkey>,
    is_delegate: Option<Vec<bool>>,
    amounts: &[impl ZeroCopyNumTrait],
    lamports: Option<Vec<Option<impl ZeroCopyNumTrait>>>,
    hashed_mint: &[u8; 32],
    merkle_tree_indices: &[u8],
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<u64> {
    let mut sum_lamports = 0;
    let hashed_delegate_store = if let Some(delegate) = delegate {
        hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
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
        // +    1       tlv (None)
        let capacity = if delegate.is_some() { 107 } else { 75 };
        let mut token_data_bytes = Vec::with_capacity(capacity);
        // 1,000 CU token data and serialize
        let token_data = TokenData {
            mint: (mint_pubkey).to_anchor_pubkey().into(),
            owner: (*owner).to_anchor_pubkey().into(),
            amount: (*amount).into(),
            delegate: delegate.map(|delegate_pubkey| delegate_pubkey.into()),
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        // TODO: remove serialization, just write bytes.
        token_data.serialize(&mut token_data_bytes).unwrap();
        bench_sbf_start!("token_data_hash");
        let hashed_owner = hash_to_bn254_field_size_be(owner.to_pubkey_bytes().as_slice());

        let mut amount_bytes = [0u8; 32];
        let discriminator_bytes =
            &remaining_accounts[merkle_tree_indices[i] as usize].try_borrow_data()?[0..8];
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                amount_bytes[24..].copy_from_slice(amount.to_bytes_le().as_slice());
                Ok(())
            }
            BATCHED_DISCRIMINATOR => {
                amount_bytes[24..].copy_from_slice(amount.to_bytes_be().as_slice());
                Ok(())
            }
            OUTPUT_QUEUE_DISCRIMINATOR => {
                amount_bytes[24..].copy_from_slice(amount.to_bytes_be().as_slice());
                Ok(())
            }
            _ => {
                msg!(
                    "{} is no Merkle tree or output queue account. ",
                    remaining_accounts[merkle_tree_indices[i] as usize].key()
                );
                err!(SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch)
            }
        }?;

        let data_hash = TokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate,
        )
        .map_err(ProgramError::from)?;

        let discriminator = get_token_account_discriminator(discriminator_bytes)?;

        let data = CompressedAccountData {
            discriminator,
            data: token_data_bytes,
            data_hash,
        };

        bench_sbf_end!("token_data_hash");
        let lamports = lamports
            .as_ref()
            .and_then(|lamports| lamports[i])
            .unwrap_or(0u64.into());
        sum_lamports += lamports.into();
        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID.into(),
                lamports: lamports.into(),
                data: Some(data),
                address: None,
            },
            merkle_tree_index: merkle_tree_indices[i],
        };
    }
    Ok(sum_lamports)
}

/// Create input compressed account data hash
/// 1. enforces discriminator
/// 2. hashes token data
/// 3. actual data is not needed for input compressed accounts
pub fn add_data_hash_to_input_compressed_accounts<const FROZEN_INPUTS: bool>(
    input_compressed_accounts_with_merkle_context: &mut [InAccount],
    input_token_data: &[TokenData],
    hashed_mint: &[u8; 32],
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<()> {
    for (i, compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter_mut()
        .enumerate()
    {
        let hashed_owner = hash_to_bn254_field_size_be(&input_token_data[i].owner.to_bytes());

        let mut amount_bytes = [0u8; 32];
        let discriminator_bytes = &remaining_accounts[compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey_index
            as usize]
            .try_borrow_data()?[0..8];
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_le_bytes().as_slice());
                Ok(())
            }
            BATCHED_DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_be_bytes().as_slice());
                Ok(())
            }
            OUTPUT_QUEUE_DISCRIMINATOR => {
                amount_bytes[24..]
                    .copy_from_slice(input_token_data[i].amount.to_be_bytes().as_slice());
                Ok(())
            }
            _ => {
                msg!(
                    "{} is no Merkle tree or output queue account. ",
                    remaining_accounts[compressed_account_with_context
                        .merkle_context
                        .merkle_tree_pubkey_index as usize]
                        .key()
                );
                err!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch)
            }
        }?;
        let delegate_store;
        let hashed_delegate = if let Some(delegate) = input_token_data[i].delegate {
            delegate_store = hash_to_bn254_field_size_be(&delegate.to_bytes());
            Some(&delegate_store)
        } else {
            None
        };
        compressed_account_with_context.data_hash = if !FROZEN_INPUTS {
            TokenData::hash_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate,
            )
            .map_err(ProgramError::from)?
        } else {
            TokenData::hash_frozen_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate,
            )
            .map_err(ProgramError::from)?
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
    input_compressed_accounts: Vec<InAccount>,
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    with_transaction_hash: bool,
    proof: Option<CompressedProof>,
    cpi_context: Option<CompressedCpiContext>,
    cpi_authority_pda: AccountInfo<'info>,
    _system_program_account_info: AccountInfo<'info>,
    _invoking_program_account_info: AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    bench_sbf_start!("t_cpi_prep");

    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];

    let cpi_context_account = cpi_context.map(|cpi_context| {
        remaining_accounts[cpi_context.cpi_context_account_index as usize].to_account_info()
    });

    #[cfg(not(feature = "cpi-without-program-ids"))]
    let mode = 0;
    #[cfg(feature = "cpi-without-program-ids")]
    let mode = 1;
    let inputs_struct = InstructionDataInvokeCpiWithReadOnly {
        mode,
        bump: BUMP_CPI_AUTHORITY,
        invoking_program_id: crate::ID.into(),
        with_cpi_context: cpi_context.is_some(),
        cpi_context: cpi_context.unwrap_or_default(),
        with_transaction_hash,
        read_only_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        input_compressed_accounts,
        output_compressed_accounts,
        proof,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
    };

    #[cfg(not(feature = "cpi-without-program-ids"))]
    {
        let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
            fee_payer: ctx.get_fee_payer().to_account_info(),
            authority: cpi_authority_pda,
            registered_program_pda: ctx.get_registered_program_pda().to_account_info(),
            noop_program: ctx.get_noop_program().to_account_info(),
            account_compression_authority: ctx
                .get_account_compression_authority()
                .to_account_info(),
            account_compression_program: ctx.get_account_compression_program().to_account_info(),
            invoking_program: _invoking_program_account_info,
            system_program: ctx.get_system_program().to_account_info(),
            sol_pool_pda: None,
            decompression_recipient: None,
            cpi_context_account,
        };
        let mut cpi_ctx = CpiContext::new_with_signer(
            _system_program_account_info,
            cpi_accounts,
            signer_seeds_ref,
        );

        cpi_ctx.remaining_accounts = remaining_accounts.to_vec();
        bench_sbf_end!("t_cpi_prep");

        bench_sbf_start!("t_invoke_cpi");
        light_system_program::cpi::invoke_cpi_with_read_only(cpi_ctx, inputs_struct)?;
        bench_sbf_end!("t_invoke_cpi");
    }
    #[cfg(feature = "cpi-without-program-ids")]
    {
        let mut inputs = Vec::new();
        InstructionDataInvokeCpiWithReadOnly::serialize(&inputs_struct, &mut inputs)
            .map_err(ProgramError::from)?;

        let mut data = Vec::with_capacity(8 + inputs.len());
        data.extend_from_slice(
            &light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY,
        );
        data.extend(inputs);

        // 4 static accounts
        let accounts_len = 4 + remaining_accounts.len() + cpi_context.is_some() as usize;
        let mut account_infos = Vec::with_capacity(accounts_len);
        let mut account_metas = Vec::with_capacity(accounts_len);
        account_infos.push(ctx.get_fee_payer().to_account_info());
        account_infos.push(cpi_authority_pda);
        account_infos.push(ctx.get_registered_program_pda().to_account_info());
        account_infos.push(ctx.get_account_compression_authority().to_account_info());

        account_metas.push(AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        });
        account_metas.push(AccountMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: account_infos[3].key(),
            is_signer: false,
            is_writable: false,
        });
        let mut remaining_accounts_index = 4;

        if let Some(account_info) = cpi_context_account {
            account_infos.push(account_info);
            account_metas.push(AccountMeta {
                pubkey: account_infos[remaining_accounts_index].key(),
                is_signer: false,
                is_writable: true,
            });
            remaining_accounts_index += 1;
        }
        for account_info in remaining_accounts {
            account_infos.push(account_info.clone());
            account_metas.push(AccountMeta {
                pubkey: account_infos[remaining_accounts_index].key(),
                is_signer: false,
                is_writable: account_infos[remaining_accounts_index].is_writable,
            });
            remaining_accounts_index += 1;
        }

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id: light_system_program::ID,
            accounts: account_metas,
            data,
        };

        anchor_lang::solana_program::program::invoke_signed(
            &instruction,
            account_infos.as_slice(),
            signer_seeds_ref.as_slice(),
        )?;
    }
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
    pub with_transaction_hash: bool,
}

pub fn get_input_compressed_accounts_with_merkle_context_and_check_signer<const IS_FROZEN: bool>(
    signer: &Pubkey,
    signer_is_delegate: &Option<DelegatedTransfer>,
    remaining_accounts: &[AccountInfo<'_>],
    input_token_data_with_context: &[InputTokenDataWithContext],
    mint: &Pubkey,
) -> Result<(Vec<InAccount>, Vec<TokenData>, u64)> {
    // Collect the total number of lamports to check whether inputs and outputs
    // are unbalanced. If unbalanced create a non token compressed change
    // account owner by the sender.
    let mut sum_lamports = 0;
    let mut input_compressed_accounts_with_merkle_context: Vec<InAccount> =
        Vec::<InAccount>::with_capacity(input_token_data_with_context.len());
    let mut input_token_data_vec: Vec<TokenData> =
        Vec::with_capacity(input_token_data_with_context.len());

    for input_token_data in input_token_data_with_context.iter() {
        let owner = if input_token_data.delegate_index.is_none() {
            *signer
        } else if let Some(signer_is_delegate) = signer_is_delegate {
            signer_is_delegate.owner
        } else {
            *signer
        };
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
        }

        // Determine discriminator based on tree type
        let discriminator_bytes = &remaining_accounts
            [input_token_data.merkle_context.merkle_tree_pubkey_index as usize]
            .try_borrow_data()?[0..8];
        let discriminator = get_token_account_discriminator(discriminator_bytes)?;

        let compressed_account = InAccount {
            lamports: input_token_data.lamports.unwrap_or_default(),
            discriminator,
            merkle_context: input_token_data.merkle_context,
            root_index: input_token_data.root_index,
            data_hash: [0u8; 32],
            address: None,
        };
        sum_lamports += compressed_account.lamports;
        let state = if IS_FROZEN {
            CompressedTokenAccountState::Frozen as u8
        } else {
            CompressedTokenAccountState::Initialized as u8
        };
        if input_token_data.tlv.is_some() {
            unimplemented!("Tlv is unimplemented.");
        }
        let token_data = TokenData {
            mint: (*mint).into(),
            owner: owner.into(),
            amount: input_token_data.amount,
            delegate: input_token_data.delegate_index.map(|_| {
                remaining_accounts[input_token_data.delegate_index.unwrap() as usize]
                    .key()
                    .into()
            }),
            state,
            tlv: None,
        };
        input_token_data_vec.push(token_data);
        input_compressed_accounts_with_merkle_context.push(compressed_account);
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

    use anchor_lang::{error_code, AnchorSerialize, Id, InstructionData, ToAccountMetas};
    use anchor_spl::{token::Token, token_2022::Token2022};
    use light_compressed_account::{
        compressed_account::{CompressedAccount, MerkleContext, PackedMerkleContext},
        instruction_data::compressed_proof::CompressedProof,
    };
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    use super::{
        DelegatedTransfer, InputTokenDataWithContext, PackedTokenTransferOutputData,
        TokenTransferOutputData,
    };
    use crate::{CompressedTokenInstructionDataTransfer, TokenData};

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
        root_indices: &[Option<u16>],
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
        is_token_22: bool,
        additional_token_pools: &[Pubkey],
        with_transaction_hash: bool,
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
            additional_token_pools,
            with_transaction_hash,
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
        let instruction_data = crate::instruction::Transfer { inputs }.data();
        let authority = if let Some(delegate) = delegate {
            delegate
        } else {
            *owner
        };
        let token_program = if compress_or_decompress_token_account.is_none() {
            None
        } else if is_token_22 {
            Some(Token2022::id())
        } else {
            Some(Token::id())
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
            token_program,
            system_program: solana_sdk::system_program::ID,
        };

        Ok(Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_inputs_and_remaining_accounts_checked(
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        input_merkle_context: &[MerkleContext],
        owner_if_delegate_is_signer: Option<Pubkey>,
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[Option<u16>],
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
                &[],
                false,
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
        root_indices: &[Option<u16>],
        proof: &Option<CompressedProof>,
        mint: Pubkey,
        is_compress: bool,
        compress_or_decompress_amount: Option<u64>,
        delegate_change_account_index: Option<u8>,
        lamports_change_account_merkle_tree: Option<Pubkey>,
        accounts: &[Pubkey],
        with_transaction_hash: bool,
    ) -> (
        HashMap<Pubkey, usize>,
        CompressedTokenInstructionDataTransfer,
    ) {
        let mut additional_accounts = Vec::new();
        additional_accounts.extend_from_slice(accounts);
        if let Some(delegate) = delegate {
            additional_accounts.push(delegate);
            for account in input_token_data.iter() {
                if account.delegate.is_some() && delegate != account.delegate.unwrap().into() {
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
            additional_accounts.push(lamports_change_account_merkle_tree);
            Some(additional_accounts.len() as u8 - 1)
        } else {
            None
        };
        let (remaining_accounts, input_token_data_with_context, _output_compressed_accounts) =
            create_input_output_and_remaining_accounts(
                additional_accounts.as_slice(),
                input_token_data,
                input_compressed_accounts,
                input_merkle_context,
                root_indices,
                output_compressed_accounts,
            );
        let delegated_transfer = if delegate.is_some() {
            let delegated_transfer = DelegatedTransfer {
                owner: input_token_data[0].owner.into(),
                delegate_change_account_index,
            };
            Some(delegated_transfer)
        } else {
            None
        };
        let inputs_struct = CompressedTokenInstructionDataTransfer {
            output_compressed_accounts: _output_compressed_accounts.to_vec(),
            proof: *proof,
            input_token_data_with_context,
            delegated_transfer,
            mint,
            is_compress,
            compress_or_decompress_amount,
            cpi_context: None,
            lamports_change_account_merkle_tree_index,
            with_transaction_hash,
        };

        (remaining_accounts, inputs_struct)
    }

    pub fn create_input_output_and_remaining_accounts(
        additional_accounts: &[Pubkey],
        input_token_data: &[TokenData],
        input_compressed_accounts: &[CompressedAccount],
        input_merkle_context: &[MerkleContext],
        root_indices: &[Option<u16>],
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
            match remaining_accounts.get(&input_merkle_context[i].merkle_tree_pubkey.into()) {
                Some(_) => {}
                None => {
                    remaining_accounts
                        .insert(input_merkle_context[i].merkle_tree_pubkey.into(), index);
                    index += 1;
                }
            };
            let delegate_index = match token_data.delegate {
                Some(delegate) => match remaining_accounts.get(&delegate.into()) {
                    Some(delegate_index) => Some(*delegate_index as u8),
                    None => {
                        remaining_accounts.insert(delegate.into(), index);
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
            // Potential footgun queue index is set in merkle tree but its not used here
            let prove_by_index = root_indices[i].is_none();

            let token_data_with_context = InputTokenDataWithContext {
                amount: token_data.amount,
                delegate_index,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: *remaining_accounts
                        .get(&input_merkle_context[i].merkle_tree_pubkey.into())
                        .unwrap() as u8,
                    queue_pubkey_index: 0,
                    leaf_index: input_merkle_context[i].leaf_index,
                    prove_by_index,
                },
                root_index: root_indices[i].unwrap_or_default(),
                lamports,
                tlv: None,
            };
            input_token_data_with_context.push(token_data_with_context);
        }
        for (i, _) in input_token_data.iter().enumerate() {
            match remaining_accounts.get(&input_merkle_context[i].queue_pubkey.into()) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(input_merkle_context[i].queue_pubkey.into(), index);
                    index += 1;
                }
            };
            input_token_data_with_context[i]
                .merkle_context
                .queue_pubkey_index = *remaining_accounts
                .get(&input_merkle_context[i].queue_pubkey.into())
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
        remaining_accounts.sort_by_key(|(_, idx)| *idx);
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<AccountMeta>>();
        remaining_accounts
    }
}

#[cfg(test)]
mod test {
    use light_ctoken_types::state::CompressedTokenAccountState;

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
        use light_compressed_account::Pubkey;
        let mut inputs = Vec::new();
        for i in input_amounts.iter() {
            inputs.push(TokenData {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                delegate: None,
                state: CompressedTokenAccountState::Initialized as u8,
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
            output_amounts,
            compress_or_decompress_amount,
            is_compress,
        )
    }
}
