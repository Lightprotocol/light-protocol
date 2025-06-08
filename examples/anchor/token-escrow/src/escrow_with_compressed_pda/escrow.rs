use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address_legacy,
    compressed_account::{CompressedAccount, CompressedAccountData, PackedMerkleContext},
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
    },
};
use light_compressed_token::{
    process_transfer::{
        CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
        PackedTokenTransferOutputData,
    },
    program::LightCompressedToken,
};
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_sdk::{
    cpi::{verify_borsh, CpiAccounts, CpiAccountsConfig},
    legacy::*,
    light_system_accounts, LightTraits,
};

use crate::{create_change_output_compressed_token_account, program::TokenEscrow, EscrowTimeLock};

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct EscrowCompressedTokensWithCompressedPda<'info> {
    /// CHECK:
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK:
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_program: Program<'info, LightCompressedToken>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    #[self_program]
    pub self_program: Program<'info, TokenEscrow>,
    /// CHECK:
    #[cpi_context]
    #[account(mut)]
    pub cpi_context_account: AccountInfo<'info>,
    /// CHECK:
    #[authority]
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedInputCompressedPda {
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
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
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(lock_up_time, &ctx, &new_address_params)?;
    let escrow_token_data = PackedTokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.token_owner_pda.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
        tlv: None,
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data.clone()],
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
        proof,
        cpi_context,
    )?;
    cpi_compressed_pda_transfer(ctx, proof, new_address_params, compressed_pda, cpi_context)?;
    Ok(())
}
fn cpi_compressed_pda_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    cpi_context.first_set_context = false;
    // Create inputs struct
    let inputs_struct = create_cpi_inputs_for_new_account(
        proof,
        new_address_params,
        compressed_pda,
        Some(cpi_context),
    );
    let mut system_accounts = vec![
        ctx.accounts.get_light_system_program().clone(),
        ctx.accounts.get_authority().clone(),
        ctx.accounts.get_registered_program_pda().clone(),
        ctx.accounts.get_noop_program().clone(),
        ctx.accounts.get_account_compression_authority().clone(),
        ctx.accounts.get_account_compression_program().clone(),
        ctx.accounts.self_program.to_account_info(),
        ctx.accounts.get_system_program().clone(),
        ctx.accounts
            .get_cpi_context_account()
            .unwrap_or(&ctx.accounts.get_light_system_program())
            .clone(),
    ];
    system_accounts.extend_from_slice(ctx.remaining_accounts);
    let light_accounts = CpiAccounts::new_with_config(
        ctx.accounts.signer.as_ref(),
        &system_accounts,
        CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER),
    )
    .unwrap();

    verify_borsh(&light_accounts, &inputs_struct).map_err(ProgramError::from)?;

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
        data_hash: timelock_compressed_pda.hash::<Poseidon>().unwrap(),
    };
    let derive_address = derive_address_legacy(
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .key()
            .into(),
        &new_address_params.seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID.into(),
            lamports: 0,
            address: Some(derive_address),
            data: Some(compressed_account_data),
        },
        merkle_tree_index: 0,
    })
}

impl light_hasher::DataHasher for EscrowTimeLock {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let mut slot_bytes = [0u8; 32];
        slot_bytes[24..].copy_from_slice(&self.slot.to_be_bytes());
        H::hash(&slot_bytes)
    }
}

#[inline(never)]
pub fn cpi_compressed_token_transfer_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    mint: Pubkey,
    _signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
    proof: CompressedProof,
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    cpi_context.set_context = true;

    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context: Some(cpi_context),
        lamports_change_account_merkle_tree_index: None,
        with_transaction_hash: false,
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
        compress_or_decompress_token_account: None,
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
