use crate::{
    account::CTokenAccount,
    error::{Result, TokenSdkError},
    instructions::transfer::account_metas::{
        get_transfer_instruction_account_metas, TokenAccountsMetaConfig,
    },
    AnchorSerialize,
};
use light_compressed_token_types::{BatchCompressInstructionData, BATCH_COMPRESS};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
#[derive()]
pub struct Recipient {
    pub pubkey: Pubkey,
    pub amount: u64,
}

pub struct CompressInputs {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub recipient: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipients: Vec<Recipient>,
    pub output_queue_pubkey: Pubkey,
    pub token_pool_pda: Pubkey,
    pub spl_token_program: Pubkey,
    pub lamports: Option<u64>,
    pub token_pool_bump: u8,
}

pub fn batch_compress(inputs: CompressInputs) -> Result<Instruction> {
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
        index: 0,
        lamports: inputs.lamports,
        bump: inputs.token_pool_bump,
    };

    // TODO: calculate exact len.
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Serialize instruction data
    let mut data = Vec::with_capacity(8 + 4 + serialized.len()); // rough estimate
    data.extend_from_slice(&BATCH_COMPRESS);
    data.extend(u32::try_from(serialized.len()).unwrap().to_le_bytes());
    data.extend(serialized);
    solana_msg::msg!("meta config1 {:?}", meta_config);
    let mut account_metas = get_transfer_instruction_account_metas(meta_config);

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    })
}
