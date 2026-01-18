use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey as SolanaPubkey;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke};
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_token_interface::instructions::mint_action::{
    CpiContext, MintActionCompressedInstructionData, MintInstructionData,
};
use light_token_interface::state::MintMetadata;
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use light_token_sdk::{
    compressed_token::{
        ctoken_instruction::CTokenInstruction,
        mint_action::{
            get_mint_action_instruction_account_metas_cpi_write, MintActionCpiAccounts,
            MintActionMetaConfigCpiWrite,
        },
    },
    CompressedProof,
};

/// Instruction data for creating two compressed mints with CPI context.
///
/// First CPI writes first mint creation to CPI context, second CPI executes both.
/// Both mints remain as compressed accounts (no auto-decompress).
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CreateTwoMintsData {
    /// Params for first mint (written to CPI context)
    pub params_1: CreateMintParamsData,
    /// Params for second mint (executed with proof)
    pub params_2: CreateMintParamsData,
    /// Single proof covering both new addresses
    pub proof: CompressedProof,
}

/// Serializable version of CreateMintParams for anchor
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CreateMintParamsData {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub bump: u8,
    pub freeze_authority: Option<Pubkey>,
}

/// Process instruction to create two compressed mints using CPI context.
///
/// The signer (ctx.accounts.signer) is used as both fee_payer and authority.
///
/// Account layout (remaining_accounts):
/// - accounts[0]: light_system_program
/// - accounts[1]: mint_signer_1 (SIGNER)
/// - accounts[2]: mint_signer_2 (SIGNER)
/// - accounts[3]: cpi_authority_pda
/// - accounts[4]: registered_program_pda
/// - accounts[5]: account_compression_authority
/// - accounts[6]: account_compression_program
/// - accounts[7]: system_program
/// - accounts[8]: cpi_context_account (writable)
/// - accounts[9]: output_queue (writable)
/// - accounts[10]: address_tree (writable)
/// - accounts[11]: compressed_token_program (for CPI)
pub fn process_create_two_mints<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    data: CreateTwoMintsData,
) -> Result<()> {
    let accounts = ctx.remaining_accounts;
    let payer = ctx.accounts.signer.to_account_info();

    // === CPI 1: Write first mint to CPI context (no proof) ===
    let mint_instruction_data_1 = MintInstructionData {
        supply: 0,
        decimals: data.params_1.decimals,
        metadata: MintMetadata {
            version: 3,
            mint: data.params_1.mint.to_bytes().into(),
            mint_decompressed: false,
            mint_signer: accounts[1].key().to_bytes(),
            bump: data.params_1.bump,
        },
        mint_authority: Some(data.params_1.mint_authority.to_bytes().into()),
        freeze_authority: data
            .params_1
            .freeze_authority
            .map(|auth| auth.to_bytes().into()),
        extensions: None,
    };

    let cpi_context_1 = CpiContext {
        set_context: false,
        first_set_context: true,
        in_tree_index: 1,
        in_queue_index: 0,
        out_queue_index: 0,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: accounts[10].key().to_bytes(),
    };

    let instruction_data_1 = MintActionCompressedInstructionData::new_mint_write_to_cpi_context(
        data.params_1.address_merkle_tree_root_index,
        mint_instruction_data_1,
        cpi_context_1,
    );

    // Build account metas for CPI write (minimal accounts)
    let cpi_write_config = MintActionMetaConfigCpiWrite {
        fee_payer: SolanaPubkey::new_from_array(payer.key().to_bytes()),
        mint_signer: Some(SolanaPubkey::new_from_array(accounts[1].key().to_bytes())),
        authority: SolanaPubkey::new_from_array(payer.key().to_bytes()),
        cpi_context: SolanaPubkey::new_from_array(accounts[8].key().to_bytes()),
    };

    let account_metas_1 = get_mint_action_instruction_account_metas_cpi_write(cpi_write_config);

    let ix_data_1 = instruction_data_1
        .data()
        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;

    let instruction_1 = Instruction {
        program_id: SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas_1,
        data: ix_data_1,
    };

    // Invoke first CPI (write to context)
    let account_infos_1 = vec![
        accounts[0].clone(),  // light_system_program
        accounts[1].clone(),  // mint_signer_1
        payer.clone(),        // authority (same as fee_payer)
        payer.clone(),        // fee_payer
        accounts[3].clone(),  // cpi_authority_pda
        accounts[8].clone(),  // cpi_context_account
        accounts[11].clone(), // compressed_token_program
    ];

    invoke(&instruction_1, &account_infos_1)?;

    msg!("CPI 1: First mint written to CPI context");

    // === CPI 2: Execute with proof (creates both compressed mints) ===
    let mint_instruction_data_2 = MintInstructionData {
        supply: 0,
        decimals: data.params_2.decimals,
        metadata: MintMetadata {
            version: 3,
            mint: data.params_2.mint.to_bytes().into(),
            mint_decompressed: false,
            mint_signer: accounts[2].key().to_bytes(),
            bump: data.params_2.bump,
        },
        mint_authority: Some(data.params_2.mint_authority.to_bytes().into()),
        freeze_authority: data
            .params_2
            .freeze_authority
            .map(|auth| auth.to_bytes().into()),
        extensions: None,
    };

    // Execute from CPI context: set cpi_context with set_context=false, first_set_context=false
    // This tells the program to READ from CPI context and execute
    // For create_mint in execute mode, in_tree_index must be 1 (hardcoded requirement)
    // Packed accounts: [0]=cpi_context, [1]=output_queue, [2]=address_tree
    let cpi_context_2 = CpiContext {
        set_context: false,
        first_set_context: false,
        in_tree_index: 1, // MUST be 1 for create_mint in execute mode with CPI context
        in_queue_index: 0, // not used for create_mint
        out_queue_index: 0, // output_queue index
        token_out_queue_index: 0,
        assigned_account_index: 1, // Second output account (first is from CPI context)
        read_only_address_trees: [0; 4],
        address_tree_pubkey: accounts[10].key().to_bytes(),
    };

    let instruction_data_2 = MintActionCompressedInstructionData::new_mint(
        data.params_2.address_merkle_tree_root_index,
        data.proof,
        mint_instruction_data_2,
    )
    .with_cpi_context(cpi_context_2);

    // Build account structure for CPI using MintActionCpiAccounts
    let empty_vec: Vec<AccountInfo<'info>> = vec![];
    let mint_action_accounts = MintActionCpiAccounts {
        compressed_token_program: &accounts[11],
        light_system_program: &accounts[0],
        mint_signer: Some(&accounts[2]),
        authority: &payer,
        fee_payer: &payer,
        compressed_token_cpi_authority: &accounts[3],
        registered_program_pda: &accounts[4],
        account_compression_authority: &accounts[5],
        account_compression_program: &accounts[6],
        system_program: &accounts[7],
        cpi_context: Some(&accounts[8]),
        out_output_queue: &accounts[9],
        in_merkle_tree: &accounts[10],
        in_output_queue: None,
        tokens_out_queue: None,
        ctoken_accounts: &empty_vec,
    };

    // Build instruction using the trait method
    let instruction_2 = instruction_data_2
        .instruction(&mint_action_accounts)
        .unwrap();

    // Invoke second CPI (execute with proof, reads from CPI context)
    let account_infos_2: Vec<_> = std::iter::once(payer)
        .chain(accounts.iter().cloned())
        .collect();

    invoke(&instruction_2, &account_infos_2)?;

    msg!("CPI 2: Both compressed mints created with single proof");

    Ok(())
}
