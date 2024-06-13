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

pub const ADDRESS_TREE: Pubkey = pubkey!("C83cpRN6oaafjNgMQJvaYgAz592EP5wunKvbokeTKPLn");
pub const ADDRESS_QUEUE: Pubkey = pubkey!("HNjtNrjt6irUPYEgxhx2Vcs42koK9fxzm3aFLHVaaRWz");

#[program]
pub mod name_service {

    use super::*;

    pub fn create_name<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateName<'info>>,
        proof: CompressedProof,
        name: u64,
        output_state_merkle_tree_account_index: u8,
        // The proof gets verified against the tree root. We don't pass the full
        // 32-byte root to save instruction data.
        address_tree_root_index: u16,
        bump: u8,
    ) -> Result<()> {
        
        // 1. Construct the PDA 
        let compressed_pda = create_compressed_pda(&ctx, &address_tree_root_index, name)?;

        
        
        // 2. Create CPI signer seed
        let bump_seed = &[bump];
        let signer_key_bytes = ctx.accounts.signer.key.to_bytes();
        let signer_seeds = [&b"name"[..], &signer_key_bytes[..], bump_seed];

        // 3. Create inputs struct
        const inputs_struct = InstructionDataInvokeCpi {
            // The proof proves that the PDA is new
            proof: Some(proof),
            // Metadata for the VM to verify the new address
            new_address_params: vec![new_address_params],
            // We create a new compressed account, so no input state.
            input_compressed_accounts_with_merkle_context: Vec::new(),
            output_compressed_accounts: vec![compressed_pda],
            signer_seeds: seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
            // These are all advanced params you don't need to worry about for
            // this example:
            is_compress: false,
            compress_or_decompress_lamports: None,
            relay_fee: None,
            cpi_context: None,
        }
        // 4. Verify and apply the state transition
        verify(ctx, &inputs_struct, &[&signer_seeds])?;

        Ok(())
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
    // This is the address that will be created
    #[account(mut)]
    pub address: AccountInfo<'info>,
    #[account]
    pub address_tree: AccountInfo<'info>,
    #[account(mut)]
    pub address_queue: AccountInfo<'info>,
    #[authority]
    #[account(seeds = [b"name".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub name_owner_pda: AccountInfo<'info>,

    #[self_program]
    pub self_program: Program<'info, NameService>,
}

// Define the account data layout.
#[account]
pub struct NamePda {
    pub name: u64,
}

// Implement the hasher trait. You can define custom hashing schemas for your
// account data. 
impl light_hasher::DataHasher for NamePda {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        H::hash(&self.name.to_le_bytes())
    }
}


// 1. set the new data
// 2. derive the address
// 3. return the compressed account in a format recognized by the VM.
fn create_compressed_pda(
    ctx: &Context<'_, '_, '_, '_, EscrowCompressedTokensWithCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
    name: u64,
) -> Result<OutputCompressedAccountWithPackedContext> {

    let name_pda_data = NamePda {
        name: name,
    };

    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: name_pda_data.try_to_vec().unwrap(),
        data_hash: name_pda_data
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };

    // TODO: this
    let address_seed = derive_address(
        &crate::ID,
        &ns,
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

