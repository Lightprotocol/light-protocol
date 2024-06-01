use crate::{create_change_output_compressed_token_account, EscrowTimeLock};
use account_compression::program::AccountCompression;
use anchor_lang::{prelude::*, Bumps};
use light_compressed_token::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
// use light_macros::light_traits;
use light_system_program::{
    invoke::processor::CompressedProof,
    invoke_cpi::account::CpiContextAccount,
    program::LightSystemProgram,
    sdk::{
        accounts::{
            InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount,
            SignerAccounts,
        },
        address::derive_address,
        compressed_account::{CompressedAccount, CompressedAccountData, PackedMerkleContext},
        invoke_cpi::get_compressed_cpi_context_account,
        CompressedCpiContext,
    },
    InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};
// use light_traits::*;
use light_trait_macro::AutoTraits;

#[inline(always)]
fn setup_cpi_accounts<'info>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
) -> light_system_program::cpi::accounts::InvokeCpiInstruction<'info> {
    light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.get_fee_payer().to_account_info(),
        authority: ctx.accounts.get_authority().to_account_info(),
        registered_program_pda: ctx.accounts.get_registered_program_pda().to_account_info(),
        noop_program: ctx.accounts.get_noop_program().to_account_info(),
        account_compression_authority: ctx
            .accounts
            .get_account_compression_authority()
            .to_account_info(),
        account_compression_program: ctx
            .accounts
            .get_account_compression_program()
            .to_account_info(),
        invoking_program: ctx.accounts.get_invoking_program().to_account_info(),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: ctx.accounts.get_system_program().to_account_info(),
        cpi_context_account: ctx
            .accounts
            .get_cpi_context_account()
            .map(|acc| acc.to_account_info()),
    }
}

#[inline(always)]
fn invoke_cpi<'info, 'a, 'b, 'c>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    cpi_accounts: light_system_program::cpi::accounts::InvokeCpiInstruction<'info>,
    inputs: Vec<u8>,
    signer_seeds: &'a [&'b [&'c [u8]]],
) -> Result<()> {
    light_system_program::cpi::invoke_cpi(
        CpiContext::new_with_signer(
            ctx.accounts.get_light_system_program().to_account_info(),
            cpi_accounts,
            signer_seeds,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
        inputs,
    )
}

// Invokes the light system program to transition the state to a compressed
// form. Serializes CPI instruction data, configures necessary accounts, and
// executes the CPI.
fn exec_verify<'info, 'a, 'b, 'c>(
    ctx: Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    inputs_struct: &InstructionDataInvokeCpi,
    signer_seeds: &'a [&'b [&'c [u8]]],
) -> Result<()> {
    let mut inputs: Vec<u8> = Vec::new();
    InstructionDataInvokeCpi::serialize(inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = setup_cpi_accounts(&ctx);
    invoke_cpi(&ctx, cpi_accounts, inputs, signer_seeds)
}

// inline alternative
fn _exec_verify_old<'info>(
    ctx: Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    inputs: Vec<u8>,
    seeds: [&[&[u8]]; 1],
) -> Result<()> {
    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.get_fee_payer().to_account_info(),
        authority: ctx.accounts.get_authority().to_account_info(),
        registered_program_pda: ctx.accounts.get_registered_program_pda().to_account_info(),
        noop_program: ctx.accounts.get_noop_program().to_account_info(),
        account_compression_authority: ctx
            .accounts
            .get_account_compression_authority()
            .to_account_info(),
        account_compression_program: ctx
            .accounts
            .get_account_compression_program()
            .to_account_info(),
        invoking_program: ctx.accounts.get_invoking_program().to_account_info(),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: ctx.accounts.get_system_program().to_account_info(),
        cpi_context_account: ctx
            .accounts
            .get_cpi_context_account()
            .map(|acc| acc.to_account_info()),
    };

    light_system_program::cpi::invoke_cpi(
        CpiContext::new_with_signer(
            ctx.accounts.get_light_system_program().to_account_info(),
            cpi_accounts,
            &seeds,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
        inputs,
    )
}

/// create compressed pda data
/// transfer tokens
/// execute complete transaction
pub fn process_escrow_compressed_tokens_with_compressed_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    lock_up_time: u64,
    escrow_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    new_address_params: NewAddressParamsPacked,
    cpi_context: CompressedCpiContext,
    bump: u8,
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(lock_up_time, &ctx, &new_address_params)?;
    let escrow_token_data = PackedTokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.token_owner_pda.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data],
        &ctx.accounts.signer.key(),
        output_state_merkle_tree_account_indices[1],
    );
    let output_compressed_accounts = vec![escrow_token_data, change_token_data];

    cpi_compressed_token_transfer_pda(
        &ctx,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        proof.clone(),
        cpi_context,
    )?;
    msg!("escrow compressed tokens with compressed pda");
    cpi_compressed_pda_transfer(
        ctx,
        proof,
        new_address_params,
        compressed_pda,
        cpi_context,
        bump,
    )?;
    Ok(())
}

/// Helper function to create data for creating a PDA.
fn create_cpi_inputs_for_new_address<'info>(
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    seeds: &[&[u8]],
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        proof: Some(proof),
        new_address_params: vec![new_address_params],
        compression_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
        cpi_context: cpi_context,
    }
}

fn cpi_compressed_pda_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: CompressedCpiContext,
    bump: u8,
) -> Result<()> {
    // Create CPI signer seed
    let bump_seed = &[bump];
    let signer_key_bytes = ctx.accounts.signer.key.to_bytes();
    let signer_seeds = [&b"escrow"[..], &signer_key_bytes[..], bump_seed];

    // Create inputs struct
    let inputs_struct = create_cpi_inputs_for_new_address(
        proof,
        new_address_params,
        compressed_pda,
        &signer_seeds,
        Some(cpi_context),
    );

    // TODO: CHECK if unnecessary because require account in escrow. Either remove or
    // replace with pure check. May be needed for cpi from another program.
    get_compressed_cpi_context_account(&ctx, &cpi_context)?;

    exec_verify(ctx, &inputs_struct, &[&signer_seeds])?;

    Ok(())
}

fn create_compressed_pda_data(
    lock_up_time: u64,
    ctx: &Context<'_, '_, '_, '_, EscrowCompressedTokensWithCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let current_slot = Clock::get()?.slot;
    let timelock_compressed_pda = EscrowTimeLock {
        slot: current_slot.checked_add(lock_up_time).unwrap(),
    };
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: timelock_compressed_pda
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };
    let derive_address = derive_address(
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .key(),
        &new_address_params.seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: Some(derive_address),
            data: Some(compressed_account_data),
        },
        merkle_tree_index: 0,
    })
}

impl light_hasher::DataHasher for EscrowTimeLock {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        H::hash(&self.slot.to_le_bytes())
    }
}

// TODO: can we turn more accounts into AccountInfos?
#[derive(Accounts, AutoTraits)]
pub struct EscrowCompressedTokensWithCompressedPda<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK:
    #[authority]
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    pub compressed_token_program:
        Program<'info, light_compressed_token::program::LightCompressedToken>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::TokenEscrow>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub cpi_context_account: Account<'info, CpiContextAccount>,
}

impl<'info> InvokeAccounts<'info> for EscrowCompressedTokensWithCompressedPda<'info> {
    fn get_registered_program_pda(
        &self,
    ) -> &Account<'info, account_compression::instructions::register_program::RegisteredProgram>
    {
        &self.registered_program_pda
    }

    fn get_noop_program(&self) -> &AccountInfo<'info> {
        &self.noop_program
    }

    fn get_account_compression_authority(&self) -> &AccountInfo<'info> {
        &self.account_compression_authority
    }

    fn get_account_compression_program(&self) -> &Program<'info, AccountCompression> {
        &self.account_compression_program
    }

    fn get_compressed_sol_pda(&self) -> Option<&UncheckedAccount<'info>> {
        // compressed_sol_pda doesnt exist... so return None
        None
        // self.compressed_sol_pda.as_ref()
    }

    fn get_compression_recipient(&self) -> Option<&UncheckedAccount<'info>> {
        // compression_recipient doesnt exist... so return None
        None
        // self.compression_recipient.as_ref()
    }

    fn get_system_program(&self) -> &Program<'info, System> {
        &self.system_program
    }
}

// commented-out:
// this is always custom, needs to be "mapped"
// impl<'info> InvokeCpiAccounts<'info> for EscrowCompressedTokensWithCompressedPda<'info> {
//     fn get_invoking_program(&self) -> &AccountInfo<'info> {
//         &self.self_program
//     }
// }

// only if Some, else None
impl<'info> InvokeCpiContextAccount<'info> for EscrowCompressedTokensWithCompressedPda<'info> {
    fn get_cpi_context_account(&self) -> Option<&Account<'info, CpiContextAccount>> {
        Some(&self.cpi_context_account)
    }
}

impl<'info> LightSystemAccount<'info> for EscrowCompressedTokensWithCompressedPda<'info> {
    fn get_light_system_program(&self) -> &Program<'info, LightSystemProgram> {
        &self.light_system_program
    }
}

// both custom!! mapped
// impl<'info> SignerAccounts<'info> for EscrowCompressedTokensWithCompressedPda<'info> {
//     fn get_fee_payer(&self) -> &Signer<'info> {
//         &self.signer
//     }

//     fn get_authority(&self) -> &AccountInfo<'info> {
//         &self.token_owner_pda
//     }
// }

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedInputCompressedPda {
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}
// TODO: add functionality to deposit into an existing escrow account
#[inline(never)]
pub fn cpi_compressed_token_transfer_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    proof: CompressedProof,
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    cpi_context.set_context = true;

    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compression_amount: None,
        cpi_context: Some(cpi_context),
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
