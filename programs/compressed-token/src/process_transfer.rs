use crate::{
    constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    create_output_compressed_accounts,
    spl_compression::process_compression,
    token_data::{AccountState, TokenData},
    ErrorCode,
};
use anchor_lang::{prelude::*, AnchorDeserialize};
use anchor_spl::token::{Token, TokenAccount};
use light_hasher::Poseidon;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
            PackedMerkleContext,
        },
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;
use std::mem;

/// Process a token transfer instruction
/// build inputs -> sum check -> build outputs -> add token data to inputs -> invoke cpi
/// 1.  Unpack compressed input accounts and input token data, this uses
///     standardized signer / delegate and will fail in proof verification in
///     case either is invalid.
/// 2.  TODO: if is delegate check delegated amount and decrease it, there needs
///     to be an output compressed account with the same compressed account data
///     as the input compressed account.
/// 3.  Check that compressed accounts are of same mint.
/// 4.  Check that sum of input compressed accounts is equal to sum of output
///     compressed accounts
/// 5.  create_output_compressed_accounts
/// 6.  Serialize and add token_data data to in compressed_accounts.
/// 7.  Invoke light_system_program::execute_compressed_transaction.
pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    bench_sbf_start!("t_deserialize");
    let inputs: CompressedTokenInstructionDataTransfer =
        CompressedTokenInstructionDataTransfer::deserialize(&mut inputs.as_slice())?;
    bench_sbf_end!("t_deserialize");
    if inputs.signer_is_delegate {
        unimplemented!("Delegate is not implemented yet.");
    }
    bench_sbf_start!("t_context_and_check_sig");
    let (mut compressed_input_accounts, input_token_data) = inputs
        .get_input_compressed_accounts_with_merkle_context_and_check_signer(
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
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
        inputs.compression_amount.as_ref(),
        inputs.is_compress,
    )?;
    bench_sbf_end!("t_sum_check");
    bench_sbf_start!("t_process_compression");
    if inputs.compression_amount.is_some() {
        process_compression(&inputs, &ctx)?;
    }
    bench_sbf_end!("t_process_compression");
    bench_sbf_start!("t_create_output_compressed_accounts");
    let hashed_mint = hash_to_bn254_field_size_be(&inputs.mint.to_bytes())
        .unwrap()
        .0;

    let mut output_compressed_accounts = vec![
        OutputCompressedAccountWithPackedContext::default();
        inputs.output_compressed_accounts.len()
    ];
    create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.owner)
            .collect::<Vec<Pubkey>>()
            .as_slice(),
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
                .map(|data: &PackedTokenTransferOutputData| {
                    if data.lamports.is_some() {
                        unimplemented!(
                            "Joint Token and lamports transfers are not implemented yet."
                        );
                    }
                    data.lamports
                })
                .collect::<Vec<Option<u64>>>()
                .as_slice(),
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
        add_token_data_to_input_compressed_accounts(
            &mut compressed_input_accounts,
            input_token_data.as_slice(),
            &hashed_mint,
        )?;
    }
    bench_sbf_end!("t_add_token_data_to_input_compressed_accounts");

    cpi_execute_compressed_transaction_transfer(
        &ctx,
        compressed_input_accounts,
        &output_compressed_accounts,
        inputs.proof,
        inputs.cpi_context,
    )?;
    Ok(())
}

/// Create output compressed accounts
/// 1. enforces discriminator
/// 2. hashes token data
pub fn add_token_data_to_input_compressed_accounts(
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
        let mut data = Vec::with_capacity(mem::size_of::<TokenData>());
        input_token_data[i].serialize(&mut data)?;
        let amount = input_token_data[i].amount.to_le_bytes();
        if input_token_data[i].delegate.is_none() && input_token_data[i].delegated_amount == 0 {
            let data = CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data,
                data_hash: TokenData::hash_with_hashed_values::<Poseidon>(
                    hashed_mint,
                    &hashed_owner,
                    &amount,
                    &input_token_data[i].is_native,
                )
                .map_err(ProgramError::from)?,
            };
            compressed_account_with_context.compressed_account.data = Some(data);
        } else {
            if input_token_data[i].delegate.is_none() {
                return err!(crate::ErrorCode::DelegateUndefined);
            }
            let hashed_delegate =
                hash_to_bn254_field_size_be(&input_token_data[i].delegate.unwrap().to_bytes())
                    .unwrap()
                    .0;
            let delegate_amount = input_token_data[i].delegated_amount.to_le_bytes();
            let data = CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data,
                data_hash: TokenData::hash_with_delegate_hashed_values::<Poseidon>(
                    hashed_mint,
                    &hashed_owner,
                    &amount,
                    input_token_data[i].is_native,
                    &hashed_delegate,
                    &delegate_amount,
                )
                .map_err(ProgramError::from)?,
            };
            compressed_account_with_context.compressed_account.data = Some(data);
        }
    }
    Ok(())
}

// TODO: consider moving this function to helpers
/// Get static cpi signer seeds
pub fn get_cpi_signer_seeds() -> [&'static [u8]; 2] {
    let bump: &[u8; 1] = &[254];
    let seeds: [&'static [u8]; 2] = [b"cpi_authority", bump];
    seeds
}

#[inline(never)]
pub fn cpi_execute_compressed_transaction_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
    input_compressed_accounts_with_merkle_context: Vec<PackedCompressedAccountWithMerkleContext>,
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    proof: Option<CompressedProof>,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    bench_sbf_start!("t_cpi_prep");

    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];

    let cpi_context_account = cpi_context.map(|cpi_context| {
        ctx.remaining_accounts[cpi_context.cpi_context_account_index as usize].to_account_info()
    });
    let inputs_struct = light_system_program::invoke_cpi::instruction::InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: output_compressed_accounts.to_vec(),
        proof,
        new_address_params: Vec::new(),
        compression_lamports: None,
        is_compress: false,
        signer_seeds: signer_seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context,
    };
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.fee_payer.to_account_info(),
        authority: ctx.accounts.cpi_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        compressed_sol_pda: None,
        compression_recipient: None,
        cpi_context_account,
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        signer_seeds_ref,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    bench_sbf_end!("t_cpi_prep");

    bench_sbf_start!("t_invoke_cpi");
    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    bench_sbf_end!("t_invoke_cpi");

    Ok(())
}

pub fn sum_check(
    input_token_data_elements: &[TokenData],
    output_amounts: &[u64],
    compression_amount: Option<&u64>,
    is_compress: bool,
) -> Result<()> {
    let mut sum: u64 = 0;
    for input_token_data in input_token_data_elements.iter() {
        sum = sum
            .checked_add(input_token_data.amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    if let Some(compression_amount) = compression_amount {
        if is_compress {
            sum = sum
                .checked_add(*compression_amount)
                .ok_or(ProgramError::ArithmeticOverflow)
                .map_err(|_| ErrorCode::ComputeCompressSumFailed)?;
        } else {
            sum = sum
                .checked_sub(*compression_amount)
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

#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    // This is the cpi signer
    /// CHECK: that mint authority is derived from signer
    #[account(seeds = [b"cpi_authority"], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
    /// CHECK: this account
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(seeds = [b"cpi_authority"], bump, seeds::program = light_system_program::ID,)]
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    pub self_program: Program<'info, crate::program::LightCompressedToken>,
    #[account(mut)]
    pub token_pool_pda: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub decompress_token_account: Option<Account<'info, TokenAccount>>,
    pub token_program: Option<Program<'info, Token>>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InputTokenDataWithContext {
    pub amount: u64,
    pub delegate_index: Option<u8>,
    pub delegated_amount: Option<u64>,
    pub is_native: Option<u64>,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

/*
* Assume:
* - all input compressed accounts have the same owner (the token program) no need to send
* - all input compressed token data has the same owner, get the owner from signer pubkey
* Instruction data:
* - mint
* - signer_is_delegate: bool
* - owner: is either signer or first place in pubkey array if signer_is_delegate
*/
// TODO: enable delegation fully by preserving delegation for every input utxo
// with a delegate create one output utxo with that delegate, take funds from
// utxos in reverse input order
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    // TODO: add root index to InputTokenDataWithContext
    // pub root_indices: Vec<u16>,
    pub mint: Pubkey, // TODO: truncate mint pubkey offchain
    pub signer_is_delegate: bool,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    // TODO: add output state merkle tree account indices to output compressed accounts
    // pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub is_compress: bool,
    pub compression_amount: Option<u64>,
    pub cpi_context: Option<CompressedCpiContext>,
}

impl CompressedTokenInstructionDataTransfer {
    pub fn get_input_compressed_accounts_with_merkle_context_and_check_signer(
        &self,
        signer: &Pubkey,
        remaining_accounts: &[AccountInfo<'_>],
    ) -> Result<(
        Vec<PackedCompressedAccountWithMerkleContext>,
        Vec<TokenData>,
    )> {
        let mut input_compressed_accounts_with_merkle_context: Vec<
            PackedCompressedAccountWithMerkleContext,
        > = Vec::<PackedCompressedAccountWithMerkleContext>::new();
        let mut input_token_data_vec: Vec<TokenData> = Vec::new();
        let owner = if self.signer_is_delegate {
            remaining_accounts[0].key()
        } else {
            *signer
        };
        for input_token_data in self.input_token_data_with_context.iter() {
            if self.signer_is_delegate
                && *signer
                    != remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
            {
                return err!(ErrorCode::DelegateSignerCheckFailed);
            }
            if input_token_data.delegated_amount.is_some()
                && input_token_data.delegate_index.is_none()
            {
                return err!(crate::ErrorCode::DelegateUndefined);
            }
            let compressed_account = CompressedAccount {
                owner: crate::ID,
                lamports: input_token_data.is_native.unwrap_or_default(),
                data: None,
                address: None,
            };
            let token_data = TokenData {
                mint: self.mint,
                owner,
                amount: input_token_data.amount,
                delegate: input_token_data.delegated_amount.map(|_| {
                    remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
                }),
                state: AccountState::Initialized,
                is_native: input_token_data.is_native,
                delegated_amount: input_token_data.delegated_amount.unwrap_or_default(),
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
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct PackedTokenTransferOutputData {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree_index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTransferOutputData {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
    pub merkle_tree: Pubkey,
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"cpi_authority"], &crate::ID)
}

#[cfg(not(target_os = "solana"))]
pub mod transfer_sdk {
    use std::collections::HashMap;

    use anchor_lang::{AnchorSerialize, Id, InstructionData, ToAccountMetas};
    use anchor_spl::token::Token;
    use light_system_program::{
        invoke::processor::CompressedProof,
        sdk::compressed_account::{MerkleContext, PackedMerkleContext},
    };
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    use crate::{
        token_data::TokenData, CompressedTokenInstructionDataTransfer,
        PackedTokenTransferOutputData, TokenTransferOutputData,
    };
    use anchor_lang::error_code;

    #[error_code]
    pub enum TransferSdkError {
        #[msg("Signer check failed")]
        SignerCheckFailed,
        #[msg("Create transfer instruction failed")]
        CreateTransferInstructionFailed,
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        input_merkle_context: &[MerkleContext],
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        input_token_data: &[TokenData],
        mint: Pubkey,
        owner_if_delegate_is_signer: Option<Pubkey>,
        is_compress: bool,
        compression_amount: Option<u64>,
        token_pool_pda: Option<Pubkey>,
        decompress_token_account: Option<Pubkey>,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, inputs_struct) = create_inputs_and_remaining_accounts(
            input_token_data,
            input_merkle_context,
            owner_if_delegate_is_signer,
            output_compressed_accounts,
            root_indices,
            proof,
            mint,
            is_compress,
            compression_amount,
        );
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut inputs = Vec::new();
        CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

        let (cpi_authority_pda, _) = crate::get_cpi_authority_pda();
        let instruction_data = crate::instruction::Transfer { inputs };

        let accounts = crate::accounts::TransferInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
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
            decompress_token_account,
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
        input_merkle_context: &[MerkleContext],
        owner_if_delegate_is_signer: Option<Pubkey>,
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        mint: Pubkey,
        owner: &Pubkey,
        is_compress: bool,
        compression_amount: Option<u64>,
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
                input_merkle_context,
                owner_if_delegate_is_signer,
                output_compressed_accounts,
                root_indices,
                proof,
                mint,
                is_compress,
                compression_amount,
            );
        Ok((remaining_accounts, compressed_accounts_ix_data))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_inputs_and_remaining_accounts(
        input_token_data: &[TokenData],
        input_merkle_context: &[MerkleContext],
        owner_if_delegate_is_signer: Option<Pubkey>,
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        proof: &Option<CompressedProof>,
        mint: Pubkey,
        is_compress: bool,
        compression_amount: Option<u64>,
    ) -> (
        HashMap<Pubkey, usize>,
        CompressedTokenInstructionDataTransfer,
    ) {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        if let Some(owner_if_delegate_is_signer) = owner_if_delegate_is_signer {
            remaining_accounts.insert(owner_if_delegate_is_signer, 0);
        }
        let mut input_token_data_with_context: Vec<crate::InputTokenDataWithContext> = Vec::new();

        let mut index = 0;
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
            let token_data_with_context = crate::InputTokenDataWithContext {
                amount: token_data.amount,
                delegate_index,
                delegated_amount: if token_data.delegated_amount == 0 {
                    None
                } else {
                    Some(token_data.delegated_amount)
                },
                is_native: token_data.is_native,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: *remaining_accounts
                        .get(&input_merkle_context[i].merkle_tree_pubkey)
                        .unwrap() as u8,
                    nullifier_queue_pubkey_index: 0,
                    leaf_index: input_merkle_context[i].leaf_index,
                },
                root_index: root_indices[i],
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
            });
        }

        let inputs_struct = CompressedTokenInstructionDataTransfer {
            output_compressed_accounts: _output_compressed_accounts.to_vec(),
            proof: proof.clone(),
            input_token_data_with_context,
            // TODO: support multiple output state merkle trees
            signer_is_delegate: owner_if_delegate_is_signer.is_some(),
            mint,
            is_compress,
            compression_amount,
            cpi_context: None,
        };

        (remaining_accounts, inputs_struct)
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
