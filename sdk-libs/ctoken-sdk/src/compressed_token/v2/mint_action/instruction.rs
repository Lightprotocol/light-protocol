use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_token_interface::{
    instructions::mint_action::MintActionCompressedInstructionData, LIGHT_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_msg::msg;
use solana_program_error::ProgramError;

use super::{cpi_accounts::MintActionCpiAccounts, MintActionCpiWriteAccounts};
use crate::{compressed_token::ctoken_instruction::CTokenInstruction, error::CTokenSdkError};

impl CTokenInstruction for MintActionCompressedInstructionData {
    type ExecuteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info> =
        MintActionCpiAccounts<'info, A>;
    type CpiWriteAccounts<'info, A: light_account_checks::AccountInfoTrait + Clone + 'info> =
        MintActionCpiWriteAccounts<'info, A>;

    fn instruction<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::ExecuteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError> {
        if let Some(ref cpi_ctx) = self.cpi_context {
            if cpi_ctx.set_context || cpi_ctx.first_set_context {
                msg!(
                    "CPI context write operations not supported in instruction(). Use instruction_write_to_cpi_context_first() or instruction_write_to_cpi_context_set() instead"
                );
                return Err(ProgramError::from(CTokenSdkError::InvalidAccountData));
            }
        }

        let data = self.data().map_err(ProgramError::from)?;

        Ok(Instruction {
            program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
            accounts: accounts.to_account_metas(),
            data,
        })
    }

    fn instruction_write_to_cpi_context_first<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError> {
        let mut instruction_data = self;
        if let Some(ref mut cpi_ctx) = instruction_data.cpi_context {
            cpi_ctx.first_set_context = true;
            cpi_ctx.set_context = false;
        } else {
            instruction_data.cpi_context = Some(
                light_token_interface::instructions::mint_action::CpiContext {
                    first_set_context: true,
                    ..Default::default()
                },
            );
        }

        build_cpi_write_instruction(instruction_data, accounts)
    }

    fn instruction_write_to_cpi_context_set<A: light_account_checks::AccountInfoTrait + Clone>(
        self,
        accounts: &Self::CpiWriteAccounts<'_, A>,
    ) -> Result<Instruction, ProgramError> {
        let mut instruction_data = self;
        if let Some(ref mut cpi_ctx) = instruction_data.cpi_context {
            cpi_ctx.set_context = true;
            cpi_ctx.first_set_context = false;
        } else {
            instruction_data.cpi_context = Some(
                light_token_interface::instructions::mint_action::CpiContext {
                    set_context: true,
                    ..Default::default()
                },
            );
        }

        build_cpi_write_instruction(instruction_data, accounts)
    }
}

/// Helper function for building CPI write instructions
#[inline(always)]
fn build_cpi_write_instruction<A: light_account_checks::AccountInfoTrait + Clone>(
    instruction_data: MintActionCompressedInstructionData,
    accounts: &MintActionCpiWriteAccounts<A>,
) -> Result<Instruction, ProgramError> {
    let data = instruction_data.data().map_err(ProgramError::from)?;
    Ok(Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
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
