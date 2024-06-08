#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_compressed_token::InputTokenDataWithContext;
use light_compressed_token::PackedTokenTransferOutputData;
use light_system_program::invoke::processor::CompressedProof;
pub mod escrow_with_compressed_pda;
pub mod escrow_with_pda;

pub use escrow_with_compressed_pda::escrow::*;
pub use escrow_with_pda::escrow::*;
use light_system_program::sdk::CompressedCpiContext;
use light_system_program::NewAddressParamsPacked;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is locked")]
    EscrowLocked,
    #[msg("CpiContextAccountIndexNotFound")]
    CpiContextAccountIndexNotFound,
}

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {

    use self::{
        escrow_with_compressed_pda::withdrawal::process_withdraw_compressed_tokens_with_compressed_pda,
        escrow_with_pda::withdrawal::process_withdraw_compressed_escrow_tokens_with_pda,
    };

    use super::*;

    pub fn escrow_compressed_tokens_with_compressed_pda<'info>(
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

        process_escrow_compressed_tokens_with_compressed_pda(
            ctx,
            lock_up_time,
            escrow_amount,
            proof,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            new_address_params,
            cpi_context,
            bump,
        )
    }   
}



#[light_accounts]
#[derive(Accounts, LightTraits)]
pub struct EscrowCompressedTokensWithCompressedPda<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK:
    #[authority]
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    pub compressed_token_program: Program<'info, LightCompressedToken>,
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    #[self_program]
    pub self_program: Program<'info, TokenEscrow>,
    /// CHECK:
    #[cpi_context]
    #[account(mut)]
    pub cpi_context_account: Account<'info, CpiContextAccount>,
}


#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedInputCompressedPda {
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
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

    verify(ctx, &inputs_struct, &[&signer_seeds])?;

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