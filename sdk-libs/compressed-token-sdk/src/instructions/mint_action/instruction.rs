use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_ctoken_types::{
    instructions::mint_action::{Action, MintActionCompressedInstructionData},
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_msg::msg;

use crate::{
    ctoken_instruction::CTokenInstruction,
    error::{Result, TokenSdkError},
    instructions::mint_action::{cpi_accounts::MintActionCpiAccounts, MintActionCpiWriteAccounts},
};

// Implement the general CTokenInstruction trait for MintActionCompressedInstructionData
impl CTokenInstruction for MintActionCompressedInstructionData {
    type ExecuteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info> =
        MintActionCpiAccounts<'info, A>;
    type CpiWriteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info> =
        MintActionCpiWriteAccounts<'info, A>;

    fn instruction<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::ExecuteAccounts<'_, A>,
    ) -> Result<Instruction> {
        // Validate that this is not a CPI write operation
        if let Some(ref cpi_ctx) = self.cpi_context {
            if cpi_ctx.set_context || cpi_ctx.first_set_context {
                msg!(
                    "CPI context write operations not supported in instruction(). Use instruction_write_to_cpi_context_first() or instruction_write_to_cpi_context_set() instead"
                );
                return Err(TokenSdkError::InvalidAccountData);
            }
        }

        // Serialize instruction data with discriminator using LightInstructionData trait
        let data = self.data().map_err(|_| TokenSdkError::SerializationError)?;

        // Build instruction
        Ok(Instruction {
            program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
            accounts: accounts.to_account_metas(false), // Don't include compressed_token_program in accounts
            data,
        })
    }

    fn instruction_write_to_cpi_context_first<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction> {
        // Set CPI context to first mode
        let mut instruction_data = self;
        if let Some(ref mut cpi_ctx) = instruction_data.cpi_context {
            cpi_ctx.first_set_context = true;
            cpi_ctx.set_context = false;
        } else {
            instruction_data.cpi_context =
                Some(light_ctoken_types::instructions::mint_action::CpiContext {
                    first_set_context: true,
                    ..Default::default()
                });
        }

        build_cpi_write_instruction(instruction_data, accounts)
    }

    fn instruction_write_to_cpi_context_set<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction> {
        // Set CPI context to set mode
        let mut instruction_data = self;
        if let Some(ref mut cpi_ctx) = instruction_data.cpi_context {
            cpi_ctx.set_context = true;
            cpi_ctx.first_set_context = false;
        } else {
            instruction_data.cpi_context =
                Some(light_ctoken_types::instructions::mint_action::CpiContext {
                    set_context: true,
                    ..Default::default()
                });
        }

        build_cpi_write_instruction(instruction_data, accounts)
    }
}

/// Helper function for building CPI write instructions
#[inline(always)]
fn build_cpi_write_instruction<A: light_account_checks::AccountInfoTrait + Clone>(
    instruction_data: MintActionCompressedInstructionData,
    accounts: &MintActionCpiWriteAccounts<A>,
) -> Result<Instruction> {
    // Check that we don't have actions that require proof or multiple accounts
    // CPI write mode is limited to simple operations
    for action in &instruction_data.actions {
        match action {
            Action::CreateSplMint(_) => {
                msg!("CreateSplMint not supported in CPI write mode");
                return Err(TokenSdkError::CannotMintWithDecompressedInCpiWrite);
            }
            Action::MintToCToken(_) => {
                // MintToCToken is allowed but needs recipient accounts
            }
            _ => {} // Other actions are OK
        }
    }

    // Serialize instruction data with discriminator using LightInstructionData trait
    let data = instruction_data
        .data()
        .map_err(|_| TokenSdkError::SerializationError)?;
    // Build instruction
    Ok(Instruction {
        program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
        accounts: {
            let mut account_metas = Vec::with_capacity(
                6 + accounts.recipient_token_accounts.len()
                    + if accounts.mint_signer.is_some() { 1 } else { 0 },
            );

            account_metas.push(solana_instruction::AccountMeta {
                pubkey: accounts.light_system_program.key().into(),
                is_writable: false,
                is_signer: false,
            });

            if let Some(mint_signer) = accounts.mint_signer {
                account_metas.push(solana_instruction::AccountMeta {
                    pubkey: mint_signer.key().into(),
                    is_writable: false,
                    is_signer: true,
                });
            }

            account_metas.push(solana_instruction::AccountMeta {
                pubkey: accounts.authority.key().into(),
                is_writable: false,
                is_signer: true,
            });

            account_metas.push(solana_instruction::AccountMeta {
                pubkey: accounts.fee_payer.key().into(),
                is_writable: true,
                is_signer: true,
            });

            account_metas.push(solana_instruction::AccountMeta {
                pubkey: accounts.cpi_authority_pda.key().into(),
                is_writable: false,
                is_signer: false,
            });

            account_metas.push(solana_instruction::AccountMeta {
                pubkey: accounts.cpi_context.key().into(),
                is_writable: true,
                is_signer: false,
            });

            for acc in &accounts.recipient_token_accounts {
                account_metas.push(solana_instruction::AccountMeta {
                    pubkey: acc.key().into(),
                    is_writable: true,
                    is_signer: false,
                });
            }

            account_metas
        },
        data,
    })
}
