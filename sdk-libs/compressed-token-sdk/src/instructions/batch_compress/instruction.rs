use light_compressed_token_types::{
    instruction::batch_compress::BatchCompressInstructionData, BATCH_COMPRESS,
};
use light_ctoken_types;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    error::{Result, TokenSdkError},
    instructions::batch_compress::account_metas::{
        get_batch_compress_instruction_account_metas, BatchCompressMetaConfig,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Recipient {
    pub pubkey: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct BatchCompressInputs {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub token_pool_pda: Pubkey,
    pub sender_token_account: Pubkey,
    pub token_program: Pubkey,
    pub merkle_tree: Pubkey,
    pub recipients: Vec<Recipient>,
    pub lamports: Option<u64>,
    pub token_pool_index: u8,
    pub token_pool_bump: u8,
    pub sol_pool_pda: Option<Pubkey>,
}

pub fn create_batch_compress_instruction(inputs: BatchCompressInputs) -> Result<Instruction> {
    let mut pubkeys = Vec::with_capacity(inputs.recipients.len());
    let mut amounts = Vec::with_capacity(inputs.recipients.len());

    inputs.recipients.iter().for_each(|recipient| {
        pubkeys.push(recipient.pubkey.to_bytes());
        amounts.push(recipient.amount);
    });

    // Create instruction data
    let instruction_data = BatchCompressInstructionData {
        pubkeys,
        amounts: Some(amounts),
        amount: None,
        index: inputs.token_pool_index,
        lamports: inputs.lamports,
        bump: inputs.token_pool_bump,
    };

    // Serialize instruction data
    let data_vec = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;
    let mut data = Vec::with_capacity(data_vec.len() + 8 + 4);
    data.extend_from_slice(BATCH_COMPRESS.as_slice());
    data.extend_from_slice(
        u32::try_from(data_vec.len())
            .unwrap()
            .to_le_bytes()
            .as_slice(),
    );
    data.extend(&data_vec);
    // Create account meta config for batch_compress (uses MintToInstruction accounts)
    let meta_config = BatchCompressMetaConfig {
        fee_payer: Some(inputs.fee_payer),
        authority: Some(inputs.authority),
        token_pool_pda: inputs.token_pool_pda,
        sender_token_account: inputs.sender_token_account,
        token_program: inputs.token_program,
        merkle_tree: inputs.merkle_tree,
        sol_pool_pda: inputs.sol_pool_pda,
    };

    // Get account metas that match MintToInstruction structure
    let account_metas = get_batch_compress_instruction_account_metas(meta_config);

    Ok(Instruction {
        program_id: Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    })
}
