// use anchor_lang::prelude::*;
// use light_hasher::{errors::HasherError, Hasher};
// use light_system_program::{
//     invoke::processor::CompressedProof,
//     sdk::{
//         address::derive_address,
//         compressed_account::{CompressedAccount, CompressedAccountData, PackedMerkleContext},
//         CompressedCpiContext,
//     },
//     InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
// };

// declare_id!("7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz");

// pub struct LightContext {
//     pub proof: CompressedProof,
//     pub new_address_params: NewAddressParamsPacked,
//     pub cpi_context: CompressedCpiContext,
//     pub bump: u8,
// }


// #[program]
// pub mod name_service {
//     use super::*;

//     pub fn create_name(ctx: Context<NameService>, light_ctx: LightContext,name: String, parent_name: Option<Pubkey>) -> Result<()> {

//         // hash the name 
//         let compressed_pda = create_compressed_pda_data(hashed_name, &ctx, &new_address_params)?;

//         let name_account = &mut ctx.accounts.name_account;

//         let seeds = [
//             ctx.accounts.owner.key.as_ref(),
//             name.as_bytes(),
//             parent_name.as_ref().map_or(&[][..], |key| key.as_ref()),
//         ];
//         let (pda, _) = Pubkey::find_program_address(&seeds, &ctx.program_id);
    

//         require!(
//             name_account.to_account_info().key == &pda,
//             CustomError::Unauthorized
//         );

        
//         name_account.owner = *ctx.accounts.owner.key;
//         name_account.name = name;
//         name_account.parent_name = parent_name;
//         Ok(())
//     }

//     pub fn update_name(ctx: Context<NameService>, new_name: String) -> Result<()> {
//         let name_account = &mut ctx.accounts.name_account;
//         require!(name_account.owner == *ctx.accounts.owner.key, CustomError::Unauthorized);
//         name_account.name = new_name;
//         Ok(())
//     }

//     pub fn delete_name(ctx: Context<NameService>) -> Result<()> {
//         let name_account = &ctx.accounts.name_account;
//         require!(name_account.owner == *ctx.accounts.owner.key, CustomError::Unauthorized);
//         Ok(())
//     }
// }


// #[account]
// #[derive(Default)]
// pub struct NameRecord {
//     pub owner: Pubkey,
//     pub name: String,
//     pub parent_name: Option<Pubkey>,
// }

// #[error_code]
// pub enum CustomError {
//     #[msg("No authority to perform this action")]
//     Unauthorized,
// }


// // can use for all. acc validation needs be manual. 
// #[derive(Accounts)]
// pub struct NameService<'info> {
//     #[account(mut)]
//     pub signer: Signer<'info>, // this the owner
//     /// CHECK:
//     #[account(seeds = [b"Light Name Service".as_slice(), signer.key.to_bytes().as_slice()], bump)]
//     pub name_account: AccountInfo<'info>,
//     pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
//     pub account_compression_program:
//         Program<'info, account_compression::program::AccountCompression>,
//     /// CHECK:
//     pub account_compression_authority: AccountInfo<'info>,
//     /// CHECK:
//     pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
//     /// CHECK:
//     pub registered_program_pda: AccountInfo<'info>,
//     /// CHECK:
//     pub noop_program: AccountInfo<'info>,
//     pub self_program: Program<'info, crate::program::NameService>,
//     pub system_program: Program<'info, System>,
//     /// CHECK:
//     #[account(mut)]
//     pub cpi_context_account: AccountInfo<'info>,
 
// }


// pub fn get_name_account_key_and_bump(signer: &Pubkey) -> (Pubkey, u8) {
//     Pubkey::find_program_address(
//         &[b"Light Name Service".as_ref(), signer.to_bytes().as_ref()],
//         &crate::id(),
//     )
// }

// // TODO: hash name 
// impl light_hasher::DataHasher for NameRecord {
//     fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
//         H::hash(&self.owner.to_le_bytes())
//     }
// }


// // TODO: move to sys sdk as util
// fn cpi_invoke_system_program<'info>(
//     ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithCompressedPda<'info>>,
//     proof: CompressedProof,
//     new_address_params: NewAddressParamsPacked,
//     compressed_pda: CompressedAccount,
//     cpi_context: CompressedCpiContext,
//     bump: u8,
// ) -> Result<()> {

//     let bump = &[bump];
//     let signer_bytes = ctx.accounts.signer.key.to_bytes();
//     let seeds = [b"Light Name Service".as_slice(), signer_bytes.as_slice(), bump];
//     let inputs_struct: InstructionDataInvokeCpi = InstructionDataInvokeCpi {
//         relay_fee: None,
//         input_compressed_accounts_with_merkle_context: Vec::new(),
//         output_compressed_accounts: vec![compressed_pda],
//         input_root_indices: Vec::new(),
//         output_state_merkle_tree_account_indices: vec![0],
//         proof: Some(proof),
//         new_address_params: vec![new_address_params],
//         compression_lamports: None,
//         is_compress: false,
//         signer_seeds: seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
//         cpi_context: Some(cpi_context),
//     };

//     let mut inputs = Vec::new();
//     InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();
//     let cpi_context_account = match Some(cpi_context) {
//         Some(cpi_context) => Some(
//             ctx.remaining_accounts
//                 .get(cpi_context.cpi_context_account_index as usize)
//                 .unwrap()
//                 .to_account_info(),
//         ),
//         None => return err!(EscrowError::CpiContextAccountIndexNotFound),
//     };
//     let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
//         fee_payer: ctx.accounts.signer.to_account_info(),
//         authority: ctx.accounts.token_owner_pda.to_account_info(),
//         registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
//         noop_program: ctx.accounts.noop_program.to_account_info(),
//         account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
//         account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
//         invoking_program: ctx.accounts.self_program.to_account_info(),
//         compressed_sol_pda: None,
//         compression_recipient: None,
//         system_program: ctx.accounts.system_program.to_account_info(),
//         cpi_context_account,
//     };
//     let seeds = [seeds.as_slice()];
//     let mut cpi_ctx = CpiContext::new_with_signer(
//         ctx.accounts.light_system_program.to_account_info(),
//         cpi_accounts,
//         &seeds,
//     );

//     cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

//     light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
//     Ok(())
// }


// fn create_compressed_pda_data(
//     hashed_name: Vec<u8>,
//     ctx: &Context<'_, '_, '_, '_, EscrowCompressedTokensWithCompressedPda<'_>>,
//     new_address_params: &NewAddressParamsPacked,
// ) -> Result<CompressedAccount> {
//     // let current_slot = Clock::get()?.slot;
//     // let timelock_compressed_pda = EscrowTimeLock {
//     //     slot: current_slot.checked_add(lock_up_time).unwrap(),
//     // };
//     // let compressed_account_data = CompressedAccountData {
//     //     discriminator: 1u64.to_le_bytes(),
//     //     data: timelock_compressed_pda.try_to_vec().unwrap(),
//     //     data_hash: timelock_compressed_pda
//     //         .hash::<Poseidon>()
//     //         .map_err(ProgramError::from)?,
//     // };
//     // let derive_address = derive_address(
//     //     &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
//     //         .key(),
//     //     &new_address_params.seed,
//     // )
//     // .map_err(|_| ProgramError::InvalidArgument)?;
//     Ok(CompressedAccount {
//         owner: crate::ID,
//         lamports: 0,
//         address: Some(derive_address),
//         data: Some(compressed_account_data),
//     })
// }