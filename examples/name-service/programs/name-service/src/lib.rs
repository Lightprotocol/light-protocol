// TODO: 
// 1) Check trees
// 2) Extend data field/deriv for name_service
// 3) Add update instruction
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_hasher::{errors::HasherError, DataHasher, Hasher, Poseidon};
use light_sdk::{
    traits::{*, InvokeCpiContextAccount, LightSystemAccount},
    pubkey,
    LightTraits,
    light_accounts,
    utils::derive_program_derived_address_seeds,
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccount, CompressedAccountData},
    NewAddressParamsPacked,
    OutputCompressedAccountWithPackedContext,
    invoke_cpi::account::CpiContextAccount,
    program::LightSystemProgram,
};
use account_compression::{program::AccountCompression, RegisteredProgram};


declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

pub const ADDRESS_TREE: Pubkey = pubkey!("C83cpRN6oaafjNgMQJvaYgAz592EP5wunKvbokeTKPLn");
pub const ADDRESS_QUEUE: Pubkey = pubkey!("HNjtNrjt6irUPYEgxhx2Vcs42koK9fxzm3aFLHVaaRWz");

#[program]
pub mod name_service {

    use light_sdk::verify::verify;
    use light_system_program::{errors::SystemProgramError, InstructionDataInvokeCpi};

    use super::*;

    
    pub fn create_name<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateName<'info>>,
        proof: CompressedProof,
        name: u64,
        // The proof gets verified against the tree root. We don't pass the full
        // 32-byte root to save instruction data.
        address_merkle_tree_root_index: u16,
        bump: u8,
    ) -> Result<()> {
        
        // 1. Construct the PDA 
        let compressed_pda = create_compressed_pda(&ctx.accounts.address, name)?;
        
        // Here we expect the first remaining_account to be the
        // pubkey of the state tree that the account shall be inserted into.
        // state_tree [0]
        // state_queue [1]
        // address_tree [2]
        // address_queue [3]
        if (ctx.remaining_accounts[2] != ADDRESS_TREE) {
            return Err(SystemProgramError::InvalidAddressTree);
        }
        if (ctx.remaining_accounts[3] != ADDRESS_QUEUE) {
            return Err(SystemProgramError::InvalidAddressQueue);
        }


        // Create CPI signer seed
        let bump_seed = &[bump];
        let signer_key_bytes = ctx.accounts.signer.key.to_bytes();
        let signer_seeds = [&b"name"[..], &signer_key_bytes[..], bump_seed];

        // Create address params struct
        let new_address_params: NewAddressParamsPacked = NewAddressParamsPacked {
            seed: derive_program_derived_address_seeds(&crate::ID, &[b"name", &signer_key_bytes[..]])?,
            address_merkle_tree_account_index: 1,
            address_queue_account_index: 2,
            address_merkle_tree_root_index,
        };

        // Create inputs struct
        let inputs_struct: InstructionDataInvokeCpi = InstructionDataInvokeCpi {
            // The proof proves that the PDA is new
            proof: Some(proof),
            // Metadata for the VM to verify the new address
            new_address_params: vec![new_address_params],
            // We create a new compressed account, so no input state.
            input_compressed_accounts_with_merkle_context: Vec::new(),
            output_compressed_accounts: vec![compressed_pda],
            signer_seeds: signer_seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
            // These are all advanced params you don't need to worry about for
            // this example:
            is_compress: false,
            compress_or_decompress_lamports: None,
            relay_fee: None,
            cpi_context: None,
        };

        // Verify and apply the state transition
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
    pub address_tree: AccountInfo<'info>,
    #[account(mut)]
    pub address_queue: AccountInfo<'info>,
    #[authority]
    #[account(seeds = [b"name".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub name_owner_pda: AccountInfo<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
}

// Define the account data layout.
#[derive(Debug)]
#[account]
pub struct NamePda {
    name: u64,
}

// Implement the hasher trait. You can define custom hashing schemas for your
// account data.
impl DataHasher for NamePda {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        H::hash(&self.name.to_le_bytes())
    }
}

// 1. set the new data
// 2. derive the address
// 3. return the compressed account in a format recognized by the VM.
fn create_compressed_pda<'info>(
    address: &AccountInfo<'info>,
    name: u64,
) -> Result<OutputCompressedAccountWithPackedContext> {

    let name_pda_data = NamePda {
        name,
    };

    // Create data hash struct as the VM expects it.
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: name_pda_data.try_to_vec().unwrap(),
        data_hash: name_pda_data
            .hash::<Poseidon>()
            .map_err(ProgramError::from)?,
    };
    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID,
            lamports: 0,
            address: Some(address.key.to_bytes()),
            data: Some(compressed_account_data),
        },
        // Set the index of your tree in remaining accounts for the CPI.
        merkle_tree_index: 0,
    })
}

