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
pub enum NameError {
}

declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

#[program]
pub mod name_service {

    use super::*;

    pub fn create_name<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
        proof: CompressedProof,
        name: u64,
        output_state_merkle_tree_account_index: u8,
        new_address_params: NewAddressParamsPacked,
        bump: u8,
    ) -> Result<()> {
        let compressed_pda = create_compressed_pda_data(lock_up_time, &ctx, &new_address_params)?;


        // Construct the PDA 
        // Consists of: account layout, address, data-hash


        // 3. Call the LightVM to verify & apply the state transition

        // Create CPI signer seed
        let bump_seed = &[bump];
        let signer_key_bytes = ctx.accounts.signer.key.to_bytes();
        let signer_seeds = [&b"name"[..], &signer_key_bytes[..], bump_seed];


        // Create inputs struct
        const inputs_struct = InstructionDataInvokeCpi {
            // The proof proves that the PDA is new
            proof: Some(proof),
            // Metadata for the VM to verify the new address
            new_address_params: vec![new_address_params],
            // We create a new compressed account, so no input state.
            input_compressed_accounts_with_merkle_context: Vec::new(),
            output_compressed_accounts: vec![compressed_pda_account],
            signer_seeds: seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
            // These are all advanced params that you dont need to worry about:
            is_compress: false,
            compress_or_decompress_lamports: None,
            relay_fee: None,
            cpi_context: None,
        }
        verify(ctx, &inputs_struct, &[&signer_seeds])?;
        
    }
}



// 1. pda not passed as pubkey, but rather in instructiondata
#[light_accounts]
#[derive(Accounts, LightTraits)]
pub struct CreateName<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    /// CHECK:
    #[authority]
    #[account(seeds = [b"name".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub name_owner_pda: AccountInfo<'info>,
    #[self_program]
    pub self_program: Program<'info, NameService>,
}


fn cpi_compressed_pda_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: CompressedCpiContext,
    bump: u8,
) -> Result<()> {
    
    Ok(())
}


fn create_compressed_pda_data(
    ctx: &Context<'_, '_, '_, '_, EscrowCompressedTokensWithCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
    name: u64,
) -> Result<OutputCompressedAccountWithPackedContext> {

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