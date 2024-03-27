use anchor_lang::{prelude::*, AnchorDeserialize};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_le;
use psp_compressed_pda::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
    },
    utils::CompressedProof,
    InstructionDataTransfer as PspCompressedPdaInstructionDataTransfer,
};

use crate::ErrorCode;

/// Process a token transfer instruction
///
/// 1. check signer / delegate
/// 2. if is delegate check delegated amount and decrease it, there needs to be an output compressed account with the same compressed account data as the input compressed account
/// 3. check in compressed_accounts are of same mint
/// 4. check sum of input compressed account is equal to sum of output compressed accounts
/// 5.1 create_output_compressed_accounts
/// 5.2 create delegate change compressed_accounts
/// 6. serialize and add token_data data to in compressed_accounts
/// 7. invoke psp_compressed_pda::execute_compressed_transaction
pub fn process_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let mut inputs: InstructionDataTransfer =
        InstructionDataTransfer::deserialize(&mut inputs.as_slice())?;

    let is_delegate =
        check_signer_or_delegate(&ctx.accounts.authority.key(), &inputs.input_token_data)?;
    if is_delegate {
        unimplemented!("delegate check not implemented");
    }

    let mint = check_mint(&inputs.input_token_data)?;

    sum_check(
        &inputs.input_token_data,
        &inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.amount)
            .collect::<Vec<u64>>(),
        None,
        true,
    )?;

    let output_compressed_accounts = crate::create_output_compressed_accounts(
        mint,
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data| data.owner)
            .collect::<Vec<Pubkey>>()
            .as_slice(),
        inputs
            .output_compressed_accounts
            .iter()
            .map(|data: &TokenTransferOutputData| data.amount)
            .collect::<Vec<u64>>()
            .as_slice(),
        Some(
            inputs
                .output_compressed_accounts
                .iter()
                .map(|data: &TokenTransferOutputData| data.lamports)
                .collect::<Vec<Option<u64>>>()
                .as_slice(),
        ),
    );
    // TODO: add create delegate change compressed_accounts
    add_token_data_to_input_compressed_accounts(
        &mut inputs.input_compressed_accounts_with_merkle_context,
        inputs.input_token_data.as_slice(),
    )?;

    cpi_execute_compressed_transaction_transfer(
        &ctx,
        inputs.input_compressed_accounts_with_merkle_context,
        inputs.root_indices,
        &output_compressed_accounts,
        inputs.output_state_merkle_tree_account_indices,
        inputs.proof,
    )?;
    Ok(())
}

/// Create output compressed accounts
/// 1. enforces discriminator
/// 2. hashes token data
pub fn add_token_data_to_input_compressed_accounts(
    input_compressed_accounts_with_merkle_context: &mut [CompressedAccountWithMerkleContext],
    input_token_data: &[TokenData],
) -> Result<()> {
    for (i, compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter_mut()
        .enumerate()
    {
        let data = CompressedAccountData {
            discriminator: 2u64.to_le_bytes(),
            data: input_token_data[i].try_to_vec().unwrap(),
            data_hash: input_token_data[i].hash().unwrap(),
        };
        compressed_account_with_context.compressed_account.data = Some(data);
    }
    Ok(())
}

#[inline(never)]
pub fn cpi_execute_compressed_transaction_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
    input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext>,
    input_root_indices: Vec<u16>,
    output_compressed_accounts: &[CompressedAccount],
    output_state_merkle_tree_account_indices: Vec<u8>,
    proof: Option<CompressedProof>,
) -> Result<()> {
    let inputs_struct = PspCompressedPdaInstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: output_compressed_accounts.to_vec(),
        input_root_indices,
        output_state_merkle_tree_account_indices,
        proof,
        new_address_seeds: Vec::new(),
        address_merkle_tree_root_indices: Vec::new(),
        address_merkle_tree_account_indices: Vec::new(),
        address_queue_account_indices: Vec::new(),
    };

    let mut inputs = Vec::new();
    PspCompressedPdaInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let (_, bump) = get_cpi_authority_pda();
    let bump = &[bump];
    let seeds = [b"cpi_authority".as_slice(), bump];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = psp_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.cpi_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .psp_account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        cpi_signature_account: None,
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs)?;
    Ok(())
}

fn check_signer_or_delegate(signer: &Pubkey, token_data_elements: &[TokenData]) -> Result<bool> {
    let mut is_delegate = false;
    for token_data in token_data_elements {
        if token_data.owner == *signer {
        } else if token_data.delegate.is_some() && token_data.delegate.unwrap() == *signer {
            is_delegate = true;
        } else {
            msg!(
                "Signer check failed token_data.owner {:?} != authority {:?}",
                token_data.owner,
                signer
            );
            return Err(ErrorCode::SignerCheckFailed.into());
        }
    }
    Ok(is_delegate)
}

fn check_mint(token_data_elemets: &[TokenData]) -> Result<Pubkey> {
    let mint = token_data_elemets[0].mint;
    for token_data in token_data_elemets {
        if token_data.mint != mint {
            return Err(ErrorCode::MintCheckFailed.into());
        }
    }
    Ok(mint)
}

pub fn sum_check(
    input_token_data_elements: &[TokenData],
    output_amounts: &[u64],
    compression_amount: Option<&u64>,
    is_compress: bool,
) -> anchor_lang::Result<()> {
    let mut sum: u64 = 0;
    for input_token_data in input_token_data_elements.iter() {
        sum = sum
            .checked_add(input_token_data.amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeInputSumFailed)?;
    }

    for amount in output_amounts.iter() {
        sum = sum
            .checked_sub(*amount)
            .ok_or(ProgramError::ArithmeticOverflow)
            .map_err(|_| ErrorCode::ComputeOutputSumFailed)?;
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
    #[account(mut)]
    pub authority: Signer<'info>,
    // This is the cpi signer
    /// CHECK: that mint authority is derived from signer
    #[account(seeds = [b"cpi_authority"], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump, seeds::program = psp_compressed_pda::ID,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    pub self_program: Program<'info, crate::program::PspCompressedToken>,
}

// TODO: parse compressed_accounts a more efficient way, since owner is sent multiple times this way
// This struct is equivalent to the InstructionDataTransfer, but uses the imported types from the psp_compressed_pda
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataTransfer {
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    input_compressed_accounts_with_merkle_context: Vec<CompressedAccountWithMerkleContext>,
    input_token_data: Vec<TokenData>,
    output_compressed_accounts: Vec<TokenTransferOutputData>,
    output_state_merkle_tree_account_indices: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct TokenTransferOutputData {
    pub owner: Pubkey,
    pub amount: u64,
    pub lamports: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64, // TODO: make instruction data optional
                               // TODO: validate that we don't need close authority
                               // /// Optional authority to close the account.
                               // pub close_authority: Option<Pubkey>,
}
// keeping this client struct for now because ts encoding is complaining about the enum, state is replaced with u8 in this struct
#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TokenDataClient {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: u8,
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    // TODO: validate that we don't need close authority
    // /// Optional authority to close the account.
    // pub close_authority: Option<Pubkey>,
}

impl DataHasher for TokenData {
    fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        let delegate = match self.delegate {
            Some(delegate) => {
                hash_to_bn254_field_size_le(delegate.to_bytes().as_slice())
                    .unwrap()
                    .0
            }
            None => [0u8; 32],
        };
        // let close_authority = match self.close_authority {
        //     Some(close_authority) => {
        //         hash_to_bn254_field_size_le(close_authority.to_bytes().as_slice())
        //             .unwrap()
        //             .0
        //     }
        //     None => [0u8; 32],
        // };
        // TODO: implement a trait hash_default value for Option<u64> and use it for other optional values
        let option_value: u8 = match self.is_native {
            Some(_) => 1,
            None => 0,
        };

        // TODO: optimize hashing scheme, to not hash rarely used values
        Poseidon::hashv(&[
            &hash_to_bn254_field_size_le(self.mint.to_bytes().as_slice())
                .unwrap()
                .0,
            &hash_to_bn254_field_size_le(self.owner.to_bytes().as_slice())
                .unwrap()
                .0,
            &self.amount.to_le_bytes(),
            &delegate,
            &(self.state as u8).to_le_bytes(),
            &[
                &[option_value][..],
                &self.is_native.unwrap_or_default().to_le_bytes(),
            ]
            .concat(),
            &self.delegated_amount.to_le_bytes(),
            // &close_authority,
        ])
    }
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"cpi_authority"], &crate::ID)
}

#[cfg(not(target_os = "solana"))]
pub mod transfer_sdk {
    use std::collections::HashMap;

    use account_compression::{AccountMeta, NOOP_PROGRAM_ID};
    use anchor_lang::{AnchorDeserialize, AnchorSerialize, InstructionData, ToAccountMetas};
    use psp_compressed_pda::{
        compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
        utils::CompressedProof,
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{InstructionDataTransfer, TokenTransferOutputData};
    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        input_compressed_account_merkle_tree_pubkeys: &[Pubkey],
        nullifier_array_pubkeys: &[Pubkey],
        output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
        input_compressed_accounts: &[CompressedAccount],
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        leaf_indices: &[u32],
        proof: &CompressedProof,
    ) -> Instruction {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        let mut input_compressed_accounts_with_merkle_context: Vec<
            CompressedAccountWithMerkleContext,
        > = Vec::<CompressedAccountWithMerkleContext>::new();
        let mut input_compressed_account_token_data: Vec<crate::TokenData> = Vec::new();
        for (i, (mt, leaf_index)) in input_compressed_account_merkle_tree_pubkeys
            .iter()
            .zip(leaf_indices)
            .enumerate()
        {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i);
                }
            };
            let mut input_compressed_account = input_compressed_accounts[i].clone();
            let token_data = crate::TokenData::deserialize(
                &mut input_compressed_account.data.unwrap().data.as_slice(),
            )
            .unwrap();
            input_compressed_account_token_data.push(token_data);
            input_compressed_account.data = None;
            input_compressed_accounts_with_merkle_context.push(
                CompressedAccountWithMerkleContext {
                    compressed_account: input_compressed_account,
                    index_merkle_tree_account: *remaining_accounts.get(mt).unwrap() as u8,
                    index_nullifier_array_account: 0,
                    leaf_index: *leaf_index,
                },
            );
        }
        let len: usize = remaining_accounts.len();
        for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            input_compressed_accounts_with_merkle_context[i].index_nullifier_array_account =
                *remaining_accounts.get(mt).unwrap() as u8;
        }
        let len: usize = remaining_accounts.len();
        let mut output_state_merkle_tree_account_indices: Vec<u8> =
            vec![0u8; output_compressed_account_merkle_tree_pubkeys.len()];
        for (i, mt) in output_compressed_account_merkle_tree_pubkeys
            .iter()
            .enumerate()
        {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            output_state_merkle_tree_account_indices[i] =
                *remaining_accounts.get(mt).unwrap() as u8;
        }

        let mut remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, i)| (AccountMeta::new(*k, false), *i))
            .collect::<Vec<(AccountMeta, usize)>>();
        // hash maps are not sorted so we need to sort manually and collect into a vector again
        remaining_accounts.sort_by(|a, b| a.1.cmp(&b.1));
        let remaining_accounts = remaining_accounts
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<AccountMeta>>();

        let inputs_struct = InstructionDataTransfer {
            input_compressed_accounts_with_merkle_context,
            output_compressed_accounts: output_compressed_accounts.to_vec(),
            root_indices: root_indices.to_vec(),
            proof: Some(proof.clone()),
            input_token_data: input_compressed_account_token_data,
            // TODO: support multiple output state merkle trees
            output_state_merkle_tree_account_indices,
        };
        let mut inputs = Vec::new();
        InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

        let (cpi_authority_pda, _) = crate::get_cpi_authority_pda();
        let instruction_data = crate::instruction::Transfer { inputs };

        let accounts = crate::accounts::TransferInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            cpi_authority_pda,
            compressed_pda_program: psp_compressed_pda::ID,
            registered_program_pda: psp_compressed_pda::utils::get_registered_program_pda(
                &psp_compressed_pda::ID,
            ),
            noop_program: NOOP_PROGRAM_ID,
            psp_account_compression_authority: psp_compressed_pda::utils::get_cpi_authority_pda(
                &psp_compressed_pda::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
        };

        Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        }
    }
}
