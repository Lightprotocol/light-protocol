use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Discriminator};
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedReadOnlyCompressedAccount,
    },
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{
            NewAddressParamsAssignedPacked, NewAddressParamsPacked,
            OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
        },
        invoke_cpi::InstructionDataInvokeCpi,
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
};
use light_hasher::{errors::HasherError, DataHasher, Poseidon};
use light_system_program::program::LightSystemProgram;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq)]
pub enum CreatePdaMode {
    Functional,
    BatchFunctional,
    ProgramIsSigner,
    ProgramIsNotSigner,
    InvalidSignerSeeds,
    InvalidInvokingProgram,
    WriteToAccountNotOwned,
    NoData,
    BatchAddressFunctional,
    InvalidBatchTreeAccount,
    OneReadOnlyAddress,
    TwoReadOnlyAddresses,
    InvalidReadOnlyAddress,
    InvalidReadOnlyRootIndex,
    InvalidReadOnlyMerkleTree,
    ReadOnlyProofOfInsertedAddress,
    UseReadOnlyAddressInAccount,
    InvalidReadOnlyAccount,
    InvalidReadOnlyAccountRootIndex,
    InvalidReadOnlyAccountMerkleTree,
    InvalidReadOnlyAccountOutputQueue,
    InvalidProofReadOnlyAccount,
    ReadOnlyProofOfInsertedAccount,
    ProofIsNoneReadOnlyAccount,
    AccountNotInValueVecMarkedProofByIndex,
    InvalidLeafIndex,
    ReadOnlyZkpOfInsertedAccount,
}

pub fn process_create_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    data: [u8; 31],
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    owner_program: Pubkey,
    cpi_context: Option<CompressedCpiContext>,
    mode: CreatePdaMode,
    bump: u8,
    read_only_address: Option<Vec<PackedReadOnlyAddress>>,
    read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
    input_accounts: Option<Vec<PackedCompressedAccountWithMerkleContext>>,
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(
        data,
        &ctx,
        &new_address_params,
        &owner_program,
        mode.clone(),
    )?;

    match mode {
        CreatePdaMode::ProgramIsNotSigner => {
            cpi_compressed_pda_transfer_as_non_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
            )?;
        }
        // functional test
        CreatePdaMode::ProgramIsSigner
        | CreatePdaMode::BatchAddressFunctional
        | CreatePdaMode::InvalidBatchTreeAccount
        | CreatePdaMode::OneReadOnlyAddress
        | CreatePdaMode::TwoReadOnlyAddresses
        | CreatePdaMode::InvalidReadOnlyAddress
        | CreatePdaMode::InvalidReadOnlyRootIndex
        | CreatePdaMode::InvalidReadOnlyMerkleTree
        | CreatePdaMode::UseReadOnlyAddressInAccount
        | CreatePdaMode::ReadOnlyProofOfInsertedAddress
        | CreatePdaMode::InvalidReadOnlyAccount
        | CreatePdaMode::InvalidReadOnlyAccountRootIndex
        | CreatePdaMode::InvalidReadOnlyAccountMerkleTree
        | CreatePdaMode::ReadOnlyProofOfInsertedAccount
        | CreatePdaMode::BatchFunctional
        | CreatePdaMode::Functional
        | CreatePdaMode::InvalidProofReadOnlyAccount
        | CreatePdaMode::InvalidReadOnlyAccountOutputQueue
        | CreatePdaMode::ProofIsNoneReadOnlyAccount
        | CreatePdaMode::AccountNotInValueVecMarkedProofByIndex
        | CreatePdaMode::InvalidLeafIndex
        | CreatePdaMode::ReadOnlyZkpOfInsertedAccount => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                read_only_address,
                read_only_accounts,
                input_accounts,
                mode,
            )?;
        }
        CreatePdaMode::InvalidSignerSeeds => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                read_only_address,
                None,
                None,
                CreatePdaMode::InvalidSignerSeeds,
            )?;
        }
        CreatePdaMode::InvalidInvokingProgram => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                read_only_address,
                None,
                None,
                CreatePdaMode::InvalidInvokingProgram,
            )?;
        }
        CreatePdaMode::WriteToAccountNotOwned => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                read_only_address,
                None,
                None,
                CreatePdaMode::WriteToAccountNotOwned,
            )?;
        }
        CreatePdaMode::NoData => {
            cpi_compressed_pda_transfer_as_program(
                &ctx,
                proof,
                new_address_params,
                compressed_pda,
                cpi_context,
                bump,
                read_only_address,
                None,
                None,
                CreatePdaMode::NoData,
            )?;
        }
    }
    Ok(())
}

/// Functional:
/// 1. ProgramIsSigner
fn cpi_compressed_pda_transfer_as_non_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> Result<()> {
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    };

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };
    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

fn cpi_compressed_pda_transfer_as_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
    bump: u8,
    mut read_only_address: Option<Vec<PackedReadOnlyAddress>>,
    mut read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
    input_accounts: Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    mode: CreatePdaMode,
) -> Result<()> {
    let invoking_program = match mode {
        CreatePdaMode::InvalidInvokingProgram => ctx.accounts.signer.to_account_info(),
        _ => ctx.accounts.self_program.to_account_info(),
    };
    let compressed_pda = match mode {
        CreatePdaMode::WriteToAccountNotOwned => {
            // account with data needs to be owned by the program
            let mut compressed_pda = compressed_pda;
            compressed_pda.compressed_account.owner = ctx.accounts.signer.key().into();
            compressed_pda
        }
        CreatePdaMode::NoData => {
            let mut compressed_pda = compressed_pda;

            compressed_pda.compressed_account.data = None;
            compressed_pda
        }
        CreatePdaMode::UseReadOnlyAddressInAccount => {
            let mut compressed_pda = compressed_pda;
            compressed_pda.compressed_account.address =
                Some(read_only_address.as_ref().unwrap()[0].address);
            compressed_pda
        }
        _ => compressed_pda,
    };

    let mut inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: input_accounts.unwrap_or_default(),
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    };
    // defining seeds again so that the cpi doesn't fail we want to test the check in the compressed pda program
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    msg!("read only address {:?}", read_only_address);
    msg!("read only accounts {:?}", read_only_accounts);
    if read_only_address.is_some() || read_only_accounts.is_some() {
        if mode == CreatePdaMode::ReadOnlyProofOfInsertedAddress {
            let read_only_address = read_only_address.as_mut().unwrap();
            read_only_address[0].address = inputs_struct.output_compressed_accounts[0]
                .compressed_account
                .address
                .unwrap();
        }
        // We currently only support two addresses hence we need to remove the
        // account and address to make space for two read only addresses.
        if mode == CreatePdaMode::TwoReadOnlyAddresses {
            inputs_struct.output_compressed_accounts = vec![];
            inputs_struct.new_address_params = vec![];
        }
        let mut remaining_accounts = ctx.remaining_accounts.to_vec();

        if let Some(read_only_address) = &mut read_only_address {
            match mode {
                CreatePdaMode::InvalidReadOnlyMerkleTree => {
                    remaining_accounts.push(ctx.accounts.registered_program_pda.to_account_info());
                    msg!(
                        "read_only_address[0].address_merkle_tree_account_index {:?}",
                        read_only_address[0].address_merkle_tree_account_index
                    );
                    read_only_address[0].address_merkle_tree_account_index =
                        (remaining_accounts.len() - 1) as u8;
                    msg!(
                        "read_only_address[0].address_merkle_tree_account_index {:?}",
                        read_only_address[0].address_merkle_tree_account_index
                    );
                }
                CreatePdaMode::InvalidReadOnlyRootIndex => {
                    read_only_address[0].address_merkle_tree_root_index = 1;
                }
                CreatePdaMode::InvalidReadOnlyAddress => {
                    read_only_address[0].address = [0; 32];
                }
                _ => {}
            }
        }
        if let Some(read_only_account) = &mut read_only_accounts {
            match mode {
                CreatePdaMode::InvalidReadOnlyAccountMerkleTree => {
                    read_only_account[0].merkle_context.merkle_tree_pubkey_index =
                        read_only_account[0].merkle_context.queue_pubkey_index;
                }
                CreatePdaMode::InvalidReadOnlyAccountRootIndex => {
                    let init_value = read_only_account[0].root_index;
                    read_only_account[0].root_index =
                        read_only_account[0].root_index.saturating_sub(1);
                    if read_only_account[0].root_index == init_value {
                        read_only_account[0].root_index =
                            read_only_account[0].root_index.saturating_add(1);
                    }
                }
                CreatePdaMode::InvalidReadOnlyAccount => {
                    read_only_account[0].account_hash = [0; 32];
                }
                CreatePdaMode::ProofIsNoneReadOnlyAccount => {
                    inputs_struct.proof = None;
                }
                CreatePdaMode::InvalidProofReadOnlyAccount => {
                    inputs_struct.proof = Some(CompressedProof::default());
                }
                CreatePdaMode::InvalidReadOnlyAccountOutputQueue => {
                    read_only_account[0].merkle_context.queue_pubkey_index =
                        read_only_account[0].merkle_context.merkle_tree_pubkey_index;
                }
                CreatePdaMode::AccountNotInValueVecMarkedProofByIndex => {
                    if read_only_account[0].merkle_context.prove_by_index {
                        panic!("Queue index shouldn't be set for mode AccountNotInValueVecMarkedProofByIndex");
                    }
                    read_only_account[0].merkle_context.prove_by_index = true;
                }
                CreatePdaMode::InvalidLeafIndex => {
                    read_only_account[0].merkle_context.leaf_index += 1;
                }
                CreatePdaMode::ReadOnlyProofOfInsertedAccount => {
                    inputs_struct.new_address_params = vec![];
                    inputs_struct.output_compressed_accounts = vec![];
                    inputs_struct.proof = None;
                }
                CreatePdaMode::ReadOnlyZkpOfInsertedAccount => {
                    inputs_struct.new_address_params = vec![];
                    inputs_struct.output_compressed_accounts = vec![];
                }
                _ => {}
            }
        }

        msg!("read_only_address {:?}", read_only_address);
        let inputs_struct = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump,
            with_transaction_hash: true,
            with_cpi_context: inputs_struct.cpi_context.is_some(),
            invoking_program_id: crate::ID.into(),
            proof: inputs_struct.proof,
            // Should fail because of this.
            new_address_params: inputs_struct
                .new_address_params
                .iter()
                .enumerate()
                .map(|(i, x)| NewAddressParamsAssignedPacked::new(*x, Some(i as u8)))
                .collect::<Vec<_>>(),
            cpi_context: inputs_struct.cpi_context.unwrap_or_default(),
            is_compress: inputs_struct.is_compress,
            compress_or_decompress_lamports: inputs_struct
                .compress_or_decompress_lamports
                .unwrap_or_default(),
            output_compressed_accounts: inputs_struct.output_compressed_accounts,
            input_compressed_accounts: inputs_struct
                .input_compressed_accounts_with_merkle_context
                .iter()
                .map(|x| InAccount {
                    address: x.compressed_account.address,
                    merkle_context: x.merkle_context,
                    lamports: x.compressed_account.lamports,
                    discriminator: x.compressed_account.data.as_ref().unwrap().discriminator,
                    data_hash: x.compressed_account.data.as_ref().unwrap().data_hash,
                    root_index: x.root_index,
                })
                .collect::<Vec<_>>(),
            read_only_addresses: read_only_address.unwrap_or_default(),
            read_only_accounts: read_only_accounts.unwrap_or_default(),
        };

        let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
            fee_payer: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.cpi_signer.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            noop_program: ctx.accounts.noop_program.to_account_info(),
            account_compression_authority: ctx
                .accounts
                .account_compression_authority
                .to_account_info(),
            account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
            invoking_program,
            sol_pool_pda: None,
            decompression_recipient: None,
            system_program: ctx.accounts.system_program.to_account_info(),
            cpi_context_account: None,
        };

        let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

        let mut cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.light_system_program.to_account_info(),
            cpi_accounts,
            &signer_seeds,
        );

        cpi_ctx.remaining_accounts = remaining_accounts;

        light_system_program::cpi::invoke_cpi_with_read_only(cpi_ctx, inputs_struct)?;
    } else {
        let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
            fee_payer: ctx.accounts.signer.to_account_info(),
            authority: ctx.accounts.cpi_signer.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            noop_program: ctx.accounts.noop_program.to_account_info(),
            account_compression_authority: ctx
                .accounts
                .account_compression_authority
                .to_account_info(),
            account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
            invoking_program,
            sol_pool_pda: None,
            decompression_recipient: None,
            system_program: ctx.accounts.system_program.to_account_info(),
            cpi_context_account: None,
        };

        let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

        let mut cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.light_system_program.to_account_info(),
            cpi_accounts,
            &signer_seeds,
        );

        cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

        light_system_program::cpi::invoke_cpi(cpi_ctx, inputs_struct.try_to_vec().unwrap())?;
    }
    Ok(())
}

fn create_compressed_pda_data(
    data: [u8; 31],
    ctx: &Context<'_, '_, '_, '_, CreateCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
    owner_program: &Pubkey,
    mode: CreatePdaMode,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let timelock_compressed_pda = RegisteredUser {
        user_pubkey: *ctx.accounts.signer.key,
        data,
    };
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: timelock_compressed_pda.hash::<Poseidon>().unwrap(),
    };
    let discriminator_bytes = &ctx.remaining_accounts
        [new_address_params.address_merkle_tree_account_index as usize]
        .try_borrow_data()?[0..8];

    let address = match discriminator_bytes {
        AddressMerkleTreeAccount::DISCRIMINATOR => derive_address_legacy(
            &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
                .key()
                .into(),
            &new_address_params.seed,
        )
        .unwrap(),
        BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => derive_address(
            &new_address_params.seed,
            &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
                .key()
                .to_bytes(),
            &crate::ID.to_bytes(),
        ),
        _ => {
            if mode == CreatePdaMode::InvalidBatchTreeAccount {
                derive_address(
                    &new_address_params.seed,
                    &ctx.remaining_accounts
                        [new_address_params.address_merkle_tree_account_index as usize]
                        .key()
                        .to_bytes(),
                    &crate::ID.to_bytes(),
                )
            } else {
                panic!("Invalid discriminator");
            }
        }
    };

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: owner_program.into(), // should be crate::ID, test can provide an invalid owner
            lamports: 0,
            address: Some(address),
            data: Some(compressed_account_data),
        },
        merkle_tree_index: 0,
    })
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct RegisteredUser {
    pub user_pubkey: Pubkey,
    pub data: [u8; 31],
}

impl light_hasher::DataHasher for RegisteredUser {
    fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let truncated_user_pubkey = hash_to_bn254_field_size_be(&self.user_pubkey.to_bytes());
        H::hashv(&[truncated_user_pubkey.as_slice(), self.data.as_slice()])
    }
}

#[derive(Accounts)]
pub struct CreateCompressedPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::SystemCpiTest>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
