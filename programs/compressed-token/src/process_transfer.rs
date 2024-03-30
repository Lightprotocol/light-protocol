use anchor_lang::{prelude::*, AnchorDeserialize};
use anchor_spl::token::{Token, TokenAccount};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_utils::hash_to_bn254_field_size_le;
use psp_compressed_pda::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
    },
    utils::CompressedProof,
    InstructionDataTransfer as PspCompressedPdaInstructionDataTransfer,
};

use crate::{spl_compression::process_compression, ErrorCode};

/// Process a token transfer instruction
///
/// 1. unpack compressed input accounts and input token data, this uses standardized signer / delegate and will fail in proof verification in case either is invalid
/// 2. TODO: if is delegate check delegated amount and decrease it, there needs to be an output compressed account with the same compressed account data as the input compressed account
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
    let inputs: CompressedTokenInstructionDataTransfer =
        CompressedTokenInstructionDataTransfer::deserialize(&mut inputs.as_slice())?;

    let (mut compressed_input_accounts, input_token_data) = inputs
        .get_input_compressed_accounts_with_merkle_context_and_check_signer(
            &ctx.accounts.authority.key(),
        )?;
    
    // TODO: check if this is already implemented  
    // if is_delegate {
    //     unimplemented!("delegate check not implemented");
    // }
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
    process_compression(&inputs, &ctx)?;

    let output_compressed_accounts = crate::create_output_compressed_accounts(
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
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
    )?;

    cpi_execute_compressed_transaction_transfer(
        &ctx,
        compressed_input_accounts,
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
        new_address_params: Vec::new(),
        compression_lamports: None,
        is_compress: false,
    };

    let mut inputs = Vec::new();
    PspCompressedPdaInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let (_, bump) = get_cpi_authority_pda();
    let bump = &[bump];
    let id = account_compression::ID.to_bytes();
    let seeds = [b"cpi_authority".as_slice(), id.as_slice(), bump];

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
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: None,
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
    #[account(mut)]
    pub authority: Signer<'info>,
    // This is the cpi signer
    /// CHECK: that mint authority is derived from signer
    #[account(seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump,)]
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
    #[account(mut)]
    pub token_pool_pda: Option<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub decompress_token_account: Option<Account<'info, TokenAccount>>,
    pub token_program: Option<Program<'info, Token>>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InputTokenDataWithContext {
    pub amount: u64,
    pub delegate_index: Option<u8>,
    pub delegated_amount: Option<u64>,
    pub is_native: Option<u64>,
    pub merkle_tree_pubkey_index: u8,
    pub nullifier_queue_pubkey_index: u8,
    pub leaf_index: u32,
}

/*
* assume:
* - all input compressed accounts have the same owner (the token program) no need to send
* - all input compressed token data has the same owner, get the owner from signer pubkey
* instruction data:
* mint
* signer_is_delegate: bool
* owner: is either signer or first place in pubkey array if signer_is_delegate
*/
// TODO: enable delegation fully by preserving delegation for every input utxo with a delegate create one output utxo with that delegate, take funds from utxos in reverse input order
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataTransfer {
    pub proof: Option<CompressedProof>,
    pub root_indices: Vec<u16>,
    pub mint: Pubkey, // TODO: truncate mint pubkey offchain
    pub signer_is_delegate: bool,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub output_compressed_accounts: Vec<TokenTransferOutputData>,
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub pubkey_array: Vec<Pubkey>,
    pub is_compress: bool,
    pub compression_amount: Option<u64>,
}

impl CompressedTokenInstructionDataTransfer {
    pub fn get_input_compressed_accounts_with_merkle_context_and_check_signer(
        &self,
        signer: &Pubkey,
    ) -> Result<(Vec<CompressedAccountWithMerkleContext>, Vec<TokenData>)> {
        let mut input_compressed_accounts_with_merkle_context: Vec<
            CompressedAccountWithMerkleContext,
        > = Vec::<CompressedAccountWithMerkleContext>::new();
        let mut input_token_data_vec: Vec<TokenData> = Vec::new();
        let owner = if self.signer_is_delegate {
            self.pubkey_array[0]
        } else {
            *signer
        };
        for input_token_data in self.input_token_data_with_context.iter() {
            if self.signer_is_delegate
                && *signer != self.pubkey_array[input_token_data.delegate_index.unwrap() as usize]
            {
                return err!(ErrorCode::SignerCheckFailed);
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
                delegate: input_token_data
                    .delegated_amount
                    .map(|_| self.pubkey_array[input_token_data.delegate_index.unwrap() as usize]),
                state: AccountState::Initialized,
                is_native: input_token_data.is_native,
                delegated_amount: input_token_data.delegated_amount.unwrap_or_default(),
            };
            input_token_data_vec.push(token_data);
            input_compressed_accounts_with_merkle_context.push(
                CompressedAccountWithMerkleContext {
                    compressed_account,
                    merkle_tree_pubkey_index: input_token_data.merkle_tree_pubkey_index,
                    nullifier_queue_pubkey_index: input_token_data.nullifier_queue_pubkey_index,
                    leaf_index: input_token_data.leaf_index,
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
    Pubkey::find_program_address(
        &[
            b"cpi_authority",
            account_compression::ID.key().to_bytes().as_slice(),
        ],
        &crate::ID,
    )
}

#[cfg(not(target_os = "solana"))]
pub mod transfer_sdk {
    use std::collections::HashMap;

    use account_compression::{AccountMeta, NOOP_PROGRAM_ID};
    use anchor_lang::{AnchorDeserialize, AnchorSerialize, Id, InstructionData, ToAccountMetas};
    use anchor_spl::token::Token;
    use psp_compressed_pda::{
        compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
        utils::CompressedProof,
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{CompressedTokenInstructionDataTransfer, TokenTransferOutputData};

    #[allow(clippy::too_many_arguments)]
    pub fn create_transfer_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        input_compressed_account_merkle_tree_pubkeys: &[Pubkey],
        nullifier_array_pubkeys: &[Pubkey],
        output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
        output_compressed_accounts: &[TokenTransferOutputData],
        root_indices: &[u16],
        leaf_indices: &[u32],
        proof: &CompressedProof,
        input_token_data: &[crate::TokenData],
        owner_if_delegate_is_signer: Option<Pubkey>,
        is_compress: bool,
        compression_amount: Option<u64>,
        token_pool_pda: Option<Pubkey>,
        decompress_token_account: Option<Pubkey>,
    ) -> Instruction {
        let mint = input_token_data[0].mint;
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        let mut input_token_data_with_context: Vec<crate::InputTokenDataWithContext> = Vec::new();
        let mut pubkey_array: HashMap<Pubkey, u8> = HashMap::new();
        let mut index = 0;
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
            let delegate_index = match input_token_data[i].delegate {
                Some(delegate) => match pubkey_array.get(&delegate) {
                    Some(delegate_index) => Some(*delegate_index),
                    None => {
                        pubkey_array.insert(delegate, index);
                        index += 1;
                        Some(index - 1)
                    }
                },
                None => None,
            };
            let token_data_with_context = crate::InputTokenDataWithContext {
                amount: input_token_data[i].amount,
                delegate_index,
                delegated_amount: if input_token_data[i].delegated_amount == 0 {
                    None
                } else {
                    Some(input_token_data[i].delegated_amount)
                },
                is_native: input_token_data[i].is_native,
                merkle_tree_pubkey_index: *remaining_accounts.get(mt).unwrap() as u8,
                nullifier_queue_pubkey_index: 0,
                leaf_index: *leaf_index,
            };
            input_token_data_with_context.push(token_data_with_context);
        }
        let len: usize = remaining_accounts.len();
        for (i, mt) in nullifier_array_pubkeys.iter().enumerate() {
            match remaining_accounts.get(mt) {
                Some(_) => {}
                None => {
                    remaining_accounts.insert(*mt, i + len);
                }
            };
            input_token_data_with_context[i].nullifier_queue_pubkey_index =
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
        let mut pubkey_array = pubkey_array.into_iter().collect::<Vec<(Pubkey, u8)>>();
        pubkey_array.sort_by(|a, b| a.1.cmp(&b.1));
        let mut pubkey_array = pubkey_array
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<Pubkey>>();
        if let Some(owner_if_delegate_is_signer) = owner_if_delegate_is_signer {
            pubkey_array.insert(0, owner_if_delegate_is_signer);
        }
        let inputs_struct = CompressedTokenInstructionDataTransfer {
            output_compressed_accounts: output_compressed_accounts.to_vec(),
            root_indices: root_indices.to_vec(),
            proof: Some(proof.clone()),
            input_token_data_with_context,
            // TODO: support multiple output state merkle trees
            output_state_merkle_tree_account_indices,
            pubkey_array,
            signer_is_delegate: owner_if_delegate_is_signer.is_some(),
            mint,
            is_compress,
            compression_amount,
        };
        let mut inputs = Vec::new();
        CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

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
            token_pool_pda,
            decompress_token_account,
            token_program: token_pool_pda.map(|_| Token::id()),
        };

        Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        }
    }
}
